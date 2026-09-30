use sdkwork_commerce_promotion_repository_sqlx::PostgresCommercePromotionStore;
use sdkwork_commerce_promotion_service::{
    PromotionCodeRedemptionCommand, PromotionOrderCouponBenefit, PromotionOrderCouponBenefitKind,
    PromotionSubscriptionPeriod,
};
use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service::{
    AccountValueFuture, CouponRedemptionBenefit, CouponRedemptionOutcome, CouponRedemptionPort,
    CouponRedemptionRequest, CouponSubscriptionPeriod,
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
        let DatabasePool::Postgres(pool, _) = pool;
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
        PromotionOrderCouponBenefitKind::CashCredit { grant_units, .. } => {
            CouponRedemptionBenefit::CashCredit {
                grant_amount: CommerceMoney::new(&grant_units.to_string())
                    .map_err(CommerceServiceError::validation)?,
            }
        }
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
