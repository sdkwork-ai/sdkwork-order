use std::sync::Arc;

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_membership_repository_sqlx::{
    AppMembershipStore, AppMembershipSubject, FulfillPaidMembershipPurchaseCommand,
    GrantCouponSubscriptionCommand, PostgresCommerceMembershipStore,
    RechargeSubscriptionQuotaCommand,
};
use sdkwork_order_service::{
    CouponSubscriptionFulfillmentOutcome, CouponSubscriptionFulfillmentRequest,
    MembershipPurchaseFulfillmentFuture, MembershipPurchaseFulfillmentOutcome,
    MembershipPurchaseFulfillmentPort, MembershipPurchaseFulfillmentRequest,
    MembershipQuotaRechargeFulfillmentOutcome, MembershipQuotaRechargeFulfillmentRequest,
};

#[derive(Clone)]
pub struct StoreMembershipFulfillmentAdapter {
    store: PostgresCommerceMembershipStore,
}

impl StoreMembershipFulfillmentAdapter {
    pub fn from_database_pool(pool: &DatabasePool) -> Result<Self, String> {
        // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
        #[allow(unreachable_patterns)]
        let pool = match pool {
            DatabasePool::Postgres(pool, _) => pool,
            _ => panic!("authoritative persistence requires a PostgreSQL pool (DATABASE_SPEC: authoritative-server)"),
        };
        Ok(Self {
            store: PostgresCommerceMembershipStore::new(pool.clone()),
        })
    }
}

impl MembershipPurchaseFulfillmentPort for StoreMembershipFulfillmentAdapter {
    fn fulfill_membership_purchase<'a>(
        &'a self,
        request: MembershipPurchaseFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, MembershipPurchaseFulfillmentOutcome> {
        Box::pin(async move {
            let membership_id = format!("membership-{}", request.order_id);
            let command = FulfillPaidMembershipPurchaseCommand {
                subject: membership_subject(
                    &request.tenant_id,
                    request.organization_id.as_deref(),
                    &request.owner_user_id,
                )?,
                package_id: request.package_id,
                order_id: request.order_id,
                membership_id,
                order_no: request.order_no,
                request_no: request.request_no,
                idempotency_key: request.idempotency_key,
                paid_at: request.paid_at,
                action: request.action,
            };
            let outcome = self.store.fulfill_paid_purchase(command).await?;
            Ok(MembershipPurchaseFulfillmentOutcome {
                accepted: outcome.accepted,
                replayed: outcome.replayed,
                fulfillment_status: outcome.fulfillment_status,
            })
        })
    }

    fn fulfill_coupon_subscription<'a>(
        &'a self,
        request: CouponSubscriptionFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, CouponSubscriptionFulfillmentOutcome> {
        Box::pin(async move {
            let subject = membership_subject(
                &request.tenant_id,
                request.organization_id.as_deref(),
                &request.owner_user_id,
            )?;
            let command = GrantCouponSubscriptionCommand {
                subject,
                product_id: request.product_id,
                sku_id: request.sku_id,
                package_id: request.package_id,
                order_id: request.order_id.clone(),
                subscription_id: format!("coupon-subscription-{}", request.order_id),
                request_no: request.request_no,
                idempotency_key: request.idempotency_key,
                requested_at: sdkwork_membership_repository_sqlx::shared::current_timestamp_string(
                ),
                period: request.period,
                duration_days: request.duration_days,
                daily_quota: request.daily_quota,
                total_quota: request.total_quota,
            };
            let outcome = self.store.grant_coupon_subscription(command).await?;
            Ok(CouponSubscriptionFulfillmentOutcome {
                accepted: outcome.accepted,
                replayed: outcome.replayed,
                subscription_id: outcome.subscription_id,
                starts_at: outcome.starts_at,
                expires_at: outcome.expires_at,
                fulfillment_status: outcome.fulfillment_status,
            })
        })
    }

    fn fulfill_membership_quota_recharge<'a>(
        &'a self,
        request: MembershipQuotaRechargeFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, MembershipQuotaRechargeFulfillmentOutcome> {
        Box::pin(async move {
            let subject = membership_subject(
                &request.tenant_id,
                request.organization_id.as_deref(),
                &request.owner_user_id,
            )?;
            let command = RechargeSubscriptionQuotaCommand {
                subject,
                order_id: request.order_id,
                quantity: request.quantity,
                request_no: request.request_no,
                idempotency_key: request.idempotency_key,
                requested_at: sdkwork_membership_repository_sqlx::shared::current_timestamp_string(
                ),
            };
            let outcome = self.store.recharge_subscription_quota(command).await?;
            Ok(MembershipQuotaRechargeFulfillmentOutcome {
                accepted: outcome.accepted,
                replayed: outcome.replayed,
                subscription_id: outcome.subscription_id,
                balance_after: outcome.balance_after,
                fulfillment_status: if outcome.replayed {
                    "replayed".to_owned()
                } else {
                    "fulfilled".to_owned()
                },
            })
        })
    }
}

pub fn membership_purchase_fulfillment_port_from_database_pool(
    pool: &DatabasePool,
) -> Result<Arc<dyn MembershipPurchaseFulfillmentPort>, String> {
    StoreMembershipFulfillmentAdapter::from_database_pool(pool).map(|adapter| {
        let port: Arc<dyn MembershipPurchaseFulfillmentPort> = Arc::new(adapter);
        port
    })
}

fn membership_subject(
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
) -> Result<AppMembershipSubject, CommerceServiceError> {
    Ok(AppMembershipSubject {
        tenant_id: positive_identifier(tenant_id, "tenant id")?,
        organization_id: optional_identifier(organization_id, "organization id")?,
        user_id: positive_identifier(owner_user_id, "owner user id")?,
    })
}

fn positive_identifier(value: &str, name: &str) -> Result<i64, CommerceServiceError> {
    value
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| CommerceServiceError::validation(format!("membership {name} is invalid")))
}

fn optional_identifier(value: Option<&str>, name: &str) -> Result<i64, CommerceServiceError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(0);
    };
    value
        .parse::<i64>()
        .ok()
        .filter(|value| *value >= 0)
        .ok_or_else(|| CommerceServiceError::validation(format!("membership {name} is invalid")))
}
