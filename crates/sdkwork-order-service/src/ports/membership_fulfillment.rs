use std::future::Future;
use std::pin::Pin;

use sdkwork_contract_service::CommerceServiceError;

pub type MembershipPurchaseFulfillmentFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MembershipPurchaseSettlementSnapshot {
    pub action: String,
    pub order_no: String,
    pub package_id: i64,
    /// 订阅期额度充值数量（仅 action=recharge）。
    pub grant_quantity: Option<i64>,
    /// 席位数量：团队/组织套餐按席位计价，单席位订单为 1。
    pub quantity: i64,
}

/// 订阅期额度充值履约请求：向当前有效会员订阅的权益账户追加额度。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MembershipQuotaRechargeFulfillmentRequest {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
    pub quantity: i64,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MembershipQuotaRechargeFulfillmentOutcome {
    pub accepted: bool,
    pub replayed: bool,
    pub subscription_id: String,
    pub balance_after: i64,
    pub fulfillment_status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MembershipPurchaseFulfillmentRequest {
    pub action: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
    pub order_no: String,
    pub package_id: i64,
    pub paid_at: String,
    pub request_no: String,
    pub idempotency_key: String,
    /// 席位数量：团队/组织套餐按席位计价，单席位订单为 1。
    pub quantity: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MembershipPurchaseFulfillmentOutcome {
    pub accepted: bool,
    pub replayed: bool,
    pub fulfillment_status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CouponSubscriptionFulfillmentRequest {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
    pub product_id: String,
    pub sku_id: String,
    pub package_id: i64,
    pub period: String,
    pub duration_days: i64,
    pub daily_quota: i64,
    pub total_quota: i64,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CouponSubscriptionFulfillmentOutcome {
    pub accepted: bool,
    pub replayed: bool,
    pub subscription_id: String,
    pub starts_at: String,
    pub expires_at: String,
    pub fulfillment_status: String,
}

pub trait MembershipPurchaseFulfillmentPort: Send + Sync {
    fn fulfill_membership_purchase<'a>(
        &'a self,
        request: MembershipPurchaseFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, MembershipPurchaseFulfillmentOutcome>;

    fn fulfill_coupon_subscription<'a>(
        &'a self,
        request: CouponSubscriptionFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, CouponSubscriptionFulfillmentOutcome>;

    fn fulfill_membership_quota_recharge<'a>(
        &'a self,
        request: MembershipQuotaRechargeFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, MembershipQuotaRechargeFulfillmentOutcome>;
}

pub fn membership_purchase_fulfillment_idempotency_key(order_id: &str) -> String {
    format!("membership-purchase:fulfill:{order_id}")
}

/// 订阅期额度充值的结算幂等键。
pub fn membership_quota_recharge_idempotency_key(order_id: &str) -> String {
    format!("membership-quota-recharge:fulfill:{order_id}")
}

pub const MEMBERSHIP_PURCHASE_FULFILLMENT_PORT: &str = "membership.purchase.fulfillment";

/// No-op adapter used when membership fulfillment is not wired at gateway assembly.
pub struct NoopMembershipPurchaseFulfillmentPort;

impl MembershipPurchaseFulfillmentPort for NoopMembershipPurchaseFulfillmentPort {
    fn fulfill_membership_purchase<'a>(
        &'a self,
        _request: MembershipPurchaseFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, MembershipPurchaseFulfillmentOutcome> {
        Box::pin(async move {
            Ok(MembershipPurchaseFulfillmentOutcome {
                accepted: false,
                replayed: false,
                fulfillment_status: "awaiting_external_fulfillment".to_owned(),
            })
        })
    }

    fn fulfill_coupon_subscription<'a>(
        &'a self,
        _request: CouponSubscriptionFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, CouponSubscriptionFulfillmentOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::unsupported_capability(
                "membership coupon subscription fulfillment port is not configured",
            ))
        })
    }

    fn fulfill_membership_quota_recharge<'a>(
        &'a self,
        _request: MembershipQuotaRechargeFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, MembershipQuotaRechargeFulfillmentOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::unsupported_capability(
                "membership quota recharge fulfillment port is not configured",
            ))
        })
    }
}
