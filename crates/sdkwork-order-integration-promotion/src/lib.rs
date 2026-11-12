use sdkwork_commerce_promotion_repository_sqlx::PostgresCommercePromotionStore;
use sdkwork_commerce_promotion_service::{
    ApplyPromotionDiscountCommand, PromotionCodeRedemptionCommand, PromotionOrderCouponBenefit,
    PromotionOrderCouponBenefitKind, PromotionSubscriptionPeriod, PromotionUserCouponItem,
};
use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service::{
    AccountValueFuture, CouponRedemptionBenefit, CouponRedemptionOutcome, CouponRedemptionPort,
    CouponRedemptionRequest, CouponSubscriptionPeriod, PromotionDiscountFuture,
    PromotionDiscountOutcome, PromotionDiscountPort, PromotionDiscountRequest,
    PromotionDiscountSettlementRequest,
};
use std::sync::Arc;

#[derive(Clone)]
struct PromotionStore {
    postgres: PostgresCommercePromotionStore,
}

#[derive(Clone)]
pub struct PromotionCouponRedemptionAdapter {
    store: PromotionStore,
}

impl PromotionCouponRedemptionAdapter {
    pub fn from_database_pool(pool: &DatabasePool) -> Self {
        // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
        #[allow(unreachable_patterns)]
        let pool = match pool {
            DatabasePool::Postgres(pool, _) => pool,
            _ => panic!("authoritative persistence requires a PostgreSQL pool (DATABASE_SPEC: authoritative-server)"),
        };
        Self {
            store: PromotionStore {
                postgres: PostgresCommercePromotionStore::new(pool.clone()),
            },
        }
    }

    async fn preview(
        &self,
        request: CouponRedemptionRequest,
    ) -> Result<CouponRedemptionOutcome, CommerceServiceError> {
        let command = promotion_command(&request)?;
        let benefit = self
            .store
            .postgres
            .preview_promotion_code_for_order(command)
            .await?;
        map_benefit(benefit)
    }

    async fn redeem(
        &self,
        request: CouponRedemptionRequest,
    ) -> Result<CouponRedemptionOutcome, CommerceServiceError> {
        let command = promotion_command(&request)?;
        let benefit = self
            .store
            .postgres
            .redeem_promotion_code_for_order(command)
            .await?;
        map_benefit(benefit)
    }
}

impl CouponRedemptionPort for PromotionCouponRedemptionAdapter {
    fn preview_coupon<'a>(
        &'a self,
        request: CouponRedemptionRequest,
    ) -> AccountValueFuture<'a, CouponRedemptionOutcome> {
        Box::pin(async move { self.preview(request).await })
    }

    fn redeem_coupon<'a>(
        &'a self,
        request: CouponRedemptionRequest,
    ) -> AccountValueFuture<'a, CouponRedemptionOutcome> {
        Box::pin(async move { self.redeem(request).await })
    }
}

pub fn promotion_coupon_redemption_port_from_database_pool(
    pool: &DatabasePool,
) -> Arc<dyn CouponRedemptionPort> {
    Arc::new(PromotionCouponRedemptionAdapter::from_database_pool(pool))
}

fn discount_amount_minor_from_item(
    item: &PromotionUserCouponItem,
) -> Result<i64, CommerceServiceError> {
    item.amount.as_str().trim().parse::<i64>().map_err(|_| {
        CommerceServiceError::storage("promotion discount amount is not integer minor units")
    })
}

#[derive(Clone)]
pub struct PromotionDiscountAdapter {
    store: PromotionStore,
}

impl PromotionDiscountAdapter {
    pub fn from_database_pool(pool: &DatabasePool) -> Self {
        #[allow(unreachable_patterns)]
        let pool = match pool {
            DatabasePool::Postgres(pool, _) => pool,
            _ => panic!("authoritative persistence requires a PostgreSQL pool (DATABASE_SPEC: authoritative-server)"),
        };
        Self {
            store: PromotionStore {
                postgres: PostgresCommercePromotionStore::new(pool.clone()),
            },
        }
    }
}

impl PromotionDiscountPort for PromotionDiscountAdapter {
    fn apply_discount<'a>(
        &'a self,
        request: PromotionDiscountRequest,
    ) -> PromotionDiscountFuture<'a, PromotionDiscountOutcome> {
        Box::pin(async move {
            let command = ApplyPromotionDiscountCommand::new(
                &request.tenant_id,
                request.organization_id.as_deref(),
                &request.owner_user_id,
                &request.order_id,
                &request.user_coupon_id,
                &request.request_no,
                &request.idempotency_key,
            )?;
            let item = self.store.postgres.apply_promotion_discount(command).await?;
            Ok(PromotionDiscountOutcome {
                discount_amount_minor: discount_amount_minor_from_item(&item)?,
            })
        })
    }

    fn settle_discounts<'a>(
        &'a self,
        request: PromotionDiscountSettlementRequest,
    ) -> PromotionDiscountFuture<'a, usize> {
        Box::pin(async move {
            self.store
                .postgres
                .settle_promotion_discounts_for_order(
                    &request.tenant_id,
                    request.organization_id.as_deref(),
                    &request.order_id,
                )
                .await
        })
    }

    fn release_discounts<'a>(
        &'a self,
        request: PromotionDiscountSettlementRequest,
    ) -> PromotionDiscountFuture<'a, usize> {
        Box::pin(async move {
            self.store
                .postgres
                .release_promotion_discounts_for_order(
                    &request.tenant_id,
                    request.organization_id.as_deref(),
                    &request.order_id,
                )
                .await
        })
    }
}

pub fn promotion_discount_port_from_database_pool(
    pool: &DatabasePool,
) -> Arc<dyn PromotionDiscountPort> {
    Arc::new(PromotionDiscountAdapter::from_database_pool(pool))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applied_discount_amount_is_read_as_integer_minor_units() {
        let item = PromotionUserCouponItem::new(
            "coupon-1",
            "CP-1",
            "3000",
            "2026-01-01T00:00:00.000Z",
            "success",
        )
        .expect("item");
        assert_eq!(
            discount_amount_minor_from_item(&item).expect("minor"),
            3000
        );

        let decimal = PromotionUserCouponItem::new(
            "coupon-2",
            "CP-2",
            "30.00",
            "2026-01-01T00:00:00.000Z",
            "success",
        )
        .expect("item");
        assert!(
            discount_amount_minor_from_item(&decimal).is_err(),
            "the order boundary only accepts the promotion minor-unit integer"
        );
    }
}

fn promotion_command(
    request: &CouponRedemptionRequest,
) -> Result<PromotionCodeRedemptionCommand, CommerceServiceError> {
    PromotionCodeRedemptionCommand::new(
        &request.tenant_id,
        request.organization_id.as_deref(),
        &request.owner_user_id,
        &request.coupon_code,
        &request.order_id,
        &request.idempotency_key,
    )
}

fn map_benefit(
    benefit: PromotionOrderCouponBenefit,
) -> Result<CouponRedemptionOutcome, CommerceServiceError> {
    let replayed = benefit.replayed;
    let benefit = match benefit.kind {
        PromotionOrderCouponBenefitKind::TokenBankCredit { grant_units, .. } => {
            CouponRedemptionBenefit::TokenBankCredit {
                grant_amount: CommerceMoney::new(&grant_units.to_string())
                    .map_err(CommerceServiceError::validation)?,
            }
        }
        PromotionOrderCouponBenefitKind::PointsCredit { grant_points } => {
            CouponRedemptionBenefit::PointsCredit {
                grant_amount: CommerceMoney::new(&grant_points.to_string())
                    .map_err(CommerceServiceError::validation)?,
            }
        }
        PromotionOrderCouponBenefitKind::CashCredit {
            grant_units,
            currency_code,
        } => CouponRedemptionBenefit::CashCredit {
            grant_amount: CommerceMoney::new(&grant_units.to_string())
                .map_err(CommerceServiceError::validation)?,
            currency_code,
        },
        PromotionOrderCouponBenefitKind::Subscription {
            product_id,
            sku_id,
            package_id,
            period,
            duration_days,
            daily_quota,
            total_quota,
        } => CouponRedemptionBenefit::Subscription {
            product_id,
            sku_id,
            package_id,
            period: map_subscription_period(period),
            duration_days,
            daily_quota,
            total_quota,
        },
    };
    Ok(CouponRedemptionOutcome {
        accepted: true,
        replayed,
        benefit,
    })
}

fn map_subscription_period(period: PromotionSubscriptionPeriod) -> CouponSubscriptionPeriod {
    match period {
        PromotionSubscriptionPeriod::Day => CouponSubscriptionPeriod::Day,
        PromotionSubscriptionPeriod::Week => CouponSubscriptionPeriod::Week,
        PromotionSubscriptionPeriod::Month => CouponSubscriptionPeriod::Month,
        PromotionSubscriptionPeriod::Quarter => CouponSubscriptionPeriod::Quarter,
        PromotionSubscriptionPeriod::Year => CouponSubscriptionPeriod::Year,
    }
}
