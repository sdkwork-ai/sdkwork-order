use sdkwork_contract_service::CommerceServiceError;

use crate::service::payment_notify::{
    default_payment_notify_handler_registry, dispatch_payment_notify_handler,
    PaymentNotifyHandlerRegistry, PAYMENT_NOTIFY_BUSINESS_ACCOUNT_RECHARGE_PACKAGE,
    PAYMENT_NOTIFY_BUSINESS_COUPON_RECHARGE, PAYMENT_NOTIFY_BUSINESS_EXTERNAL,
    PAYMENT_NOTIFY_BUSINESS_MEMBERSHIP, PAYMENT_NOTIFY_BUSINESS_POINTS_RECHARGE,
    PAYMENT_NOTIFY_BUSINESS_PRODUCT, PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_RECHARGE,
    PAYMENT_NOTIFY_BUSINESS_UNKNOWN,
};
use crate::{
    MembershipPurchaseSettlementSnapshot, OrderPaymentSettlementAttempt,
    OwnerOrderPaymentConfirmationPort, OwnerOrderPaymentStatePort,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OrderSubjectKind {
    PointsRecharge,
    TokenBankRecharge,
    TokenBankPlanPurchase,
    TokenBankPlanRenewal,
    AccountRechargePackage,
    CouponRecharge,
    Product,
    VirtualGoods,
    Membership,
    CouponPackage,
    External,
    Unknown,
}

impl OrderSubjectKind {
    pub fn parse(subject: Option<&str>) -> Self {
        match subject.map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) if value.eq_ignore_ascii_case("points_recharge") => Self::PointsRecharge,
            Some(value) if value.eq_ignore_ascii_case("token_bank_recharge") => {
                Self::TokenBankRecharge
            }
            Some(value) if value.eq_ignore_ascii_case("token_bank_plan_purchase") => {
                Self::TokenBankPlanPurchase
            }
            Some(value) if value.eq_ignore_ascii_case("token_bank_plan_renewal") => {
                Self::TokenBankPlanRenewal
            }
            Some(value) if value.eq_ignore_ascii_case("account_recharge_package") => {
                Self::AccountRechargePackage
            }
            Some(value) if value.eq_ignore_ascii_case("coupon_recharge") => Self::CouponRecharge,
            Some(value) if value.eq_ignore_ascii_case("product") => Self::Product,
            Some(value) if value.eq_ignore_ascii_case("physical") => Self::Product,
            Some(value) if value.eq_ignore_ascii_case("physical_shipment") => Self::Product,
            Some(value) if value.eq_ignore_ascii_case("virtual_goods") => Self::VirtualGoods,
            Some(value) if value.eq_ignore_ascii_case("virtual") => Self::VirtualGoods,
            Some(value) if value.eq_ignore_ascii_case("virtual_delivery") => Self::VirtualGoods,
            Some(value) if value.eq_ignore_ascii_case("membership") => Self::Membership,
            Some(value) if value.eq_ignore_ascii_case("membership_activation") => Self::Membership,
            Some(value) if value.eq_ignore_ascii_case("coupon_package") => Self::CouponPackage,
            Some(value) if value.eq_ignore_ascii_case("points_credit") => Self::PointsRecharge,
            Some(value) if is_machine_subject(value) => Self::External,
            Some(_) => Self::Unknown,
            None => Self::Unknown,
        }
    }

    pub fn is_fulfillment_implemented(self) -> bool {
        matches!(
            self,
            Self::PointsRecharge
                | Self::TokenBankRecharge
                | Self::TokenBankPlanPurchase
                | Self::TokenBankPlanRenewal
                | Self::AccountRechargePackage
                | Self::CouponRecharge
                | Self::Membership
        )
    }

    /// Canonical notify business type used as the fulfillment dispatch key.
    /// The subject resolution is the order domain's authoritative derivation;
    /// every kind maps to exactly one business type so the
    /// `PaymentNotifyHandlerRegistry` lookup is total.
    pub fn business_type(self) -> &'static str {
        match self {
            Self::PointsRecharge => PAYMENT_NOTIFY_BUSINESS_POINTS_RECHARGE,
            Self::TokenBankRecharge => PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_RECHARGE,
            Self::TokenBankPlanPurchase | Self::TokenBankPlanRenewal => {
                // Plan purchase and renewal share the token bank account value
                // handler family but keep distinct business types so per-flow
                // idempotency and metrics stay separated.
                if self == Self::TokenBankPlanPurchase {
                    "token_bank_plan_purchase"
                } else {
                    "token_bank_plan_renewal"
                }
            }
            Self::AccountRechargePackage => PAYMENT_NOTIFY_BUSINESS_ACCOUNT_RECHARGE_PACKAGE,
            Self::CouponRecharge => PAYMENT_NOTIFY_BUSINESS_COUPON_RECHARGE,
            Self::Product => PAYMENT_NOTIFY_BUSINESS_PRODUCT,
            Self::Membership => PAYMENT_NOTIFY_BUSINESS_MEMBERSHIP,
            Self::VirtualGoods | Self::CouponPackage | Self::External => {
                PAYMENT_NOTIFY_BUSINESS_EXTERNAL
            }
            Self::Unknown => PAYMENT_NOTIFY_BUSINESS_UNKNOWN,
        }
    }
}

/// Resolve a checkout/order subject from machine-readable merchandise facts.
/// Display titles are intentionally ignored because they are localized and mutable.
pub fn stable_checkout_order_subject(
    fulfillment_type: Option<&str>,
    sku_snapshot_json: Option<&str>,
) -> String {
    normalized_machine_subject(fulfillment_type)
        .or_else(|| stable_subject_from_snapshot(sku_snapshot_json))
        .unwrap_or_else(|| "product".to_owned())
}

/// Resolve the subject used by payment settlement for existing and new orders.
/// Snapshot metadata wins for checkout orders; canonical header subjects remain
/// the fallback for recharge and membership orders that do not use SKU snapshots.
pub fn stable_order_settlement_subject(
    stored_subject: Option<&str>,
    sku_snapshot_json: Option<&str>,
) -> String {
    stable_subject_from_snapshot(sku_snapshot_json)
        .or_else(|| canonical_stored_order_subject(stored_subject))
        .unwrap_or_else(|| "product".to_owned())
}

fn canonical_stored_order_subject(subject: Option<&str>) -> Option<String> {
    let subject = normalized_machine_subject(subject)?;
    match OrderSubjectKind::parse(Some(&subject)) {
        OrderSubjectKind::External | OrderSubjectKind::Unknown => None,
        _ => Some(subject),
    }
}

fn stable_subject_from_snapshot(sku_snapshot_json: Option<&str>) -> Option<String> {
    let snapshot = sku_snapshot_json
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let value = serde_json::from_str::<serde_json::Value>(snapshot).ok()?;
    [
        "fulfillment_type",
        "fulfillmentType",
        "product_type",
        "productType",
    ]
    .into_iter()
    .find_map(|key| value.get(key).and_then(serde_json::Value::as_str))
    .and_then(|subject| normalized_machine_subject(Some(subject)))
}

fn normalized_machine_subject(subject: Option<&str>) -> Option<String> {
    let subject = subject?.trim();
    if !is_machine_subject(subject) {
        return None;
    }
    Some(subject.to_ascii_lowercase())
}

fn is_machine_subject(subject: &str) -> bool {
    !subject.is_empty()
        && subject
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OwnerOrderSettlementOutcome {
    pub payment_confirmed: bool,
    pub payment_replayed: bool,
    pub fulfillment_accepted: bool,
    pub fulfillment_replayed: bool,
    pub order_id: String,
    pub points_credited: i64,
    pub fulfillment_status: String,
}

#[derive(Clone, Copy)]
pub struct OwnerOrderSettlementPorts<'a> {
    pub payment_store: &'a dyn OwnerOrderPaymentConfirmationPort,
    pub order_state_store: &'a dyn OwnerOrderPaymentStatePort,
    pub recharge_store: &'a dyn crate::PointsRechargeFulfillmentStore,
    pub account_value_store: &'a dyn crate::AccountValueFulfillmentStore,
    pub credit_port: &'a dyn crate::AccountPointsCreditPort,
    pub account_value_ledger_port: &'a dyn crate::AccountValueLedgerPort,
    pub coupon_redemption_port: &'a dyn crate::CouponRedemptionPort,
    pub membership_port: &'a dyn crate::MembershipPurchaseFulfillmentPort,
    pub physical_goods_port: &'a dyn crate::PhysicalGoodsFulfillmentPort,
}

/// Settles a confirmed successful owner order payment with the standard
/// built-in notify handler registry.
pub async fn settle_owner_order_after_payment_success(
    ports: OwnerOrderSettlementPorts<'_>,
    attempt: &OrderPaymentSettlementAttempt,
    order_subject: Option<&str>,
    membership_purchase: Option<&MembershipPurchaseSettlementSnapshot>,
    request_no: &str,
) -> Result<OwnerOrderSettlementOutcome, CommerceServiceError> {
    settle_owner_order_after_payment_success_with_registry(
        ports,
        attempt,
        order_subject,
        membership_purchase,
        request_no,
        None,
        default_payment_notify_handler_registry().as_ref(),
    )
    .await
}

/// Settles a confirmed successful owner order payment with an injected notify
/// handler registry. The registry is the module extension point: business
/// flows plug their own handlers without touching the canonical pipeline.
pub async fn settle_owner_order_after_payment_success_with_registry(
    ports: OwnerOrderSettlementPorts<'_>,
    attempt: &OrderPaymentSettlementAttempt,
    order_subject: Option<&str>,
    membership_purchase: Option<&MembershipPurchaseSettlementSnapshot>,
    request_no: &str,
    event_type: Option<&str>,
    notify_handler_registry: &dyn PaymentNotifyHandlerRegistry,
) -> Result<OwnerOrderSettlementOutcome, CommerceServiceError> {
    let payment_outcome = ports
        .payment_store
        .confirm_owner_order_payment(attempt)
        .await?;

    let order_state_outcome = ports
        .order_state_store
        .mark_owner_order_payment_succeeded(attempt, &payment_outcome.paid_at)
        .await?;

    if order_state_outcome.terminal_order_preserved {
        tracing::warn!(
            target = "order.settlement",
            order_id = %attempt.order_id,
            order_status = %order_state_outcome.order_status,
            "payment confirmed after negative terminal order state; automated fulfillment suppressed"
        );
        return Ok(OwnerOrderSettlementOutcome {
            payment_confirmed: true,
            payment_replayed: payment_outcome.replayed,
            fulfillment_accepted: false,
            fulfillment_replayed: false,
            order_id: attempt.order_id.clone(),
            points_credited: 0,
            fulfillment_status: "late_payment_requires_recovery".to_owned(),
        });
    }

    let subject_kind = OrderSubjectKind::parse(order_subject);
    let fulfillment = dispatch_payment_notify_handler(
        notify_handler_registry,
        &ports,
        subject_kind,
        attempt,
        &payment_outcome.paid_at,
        membership_purchase,
        request_no,
        event_type,
    )
    .await?;

    Ok(OwnerOrderSettlementOutcome {
        payment_confirmed: true,
        payment_replayed: payment_outcome.replayed,
        fulfillment_accepted: fulfillment.accepted,
        fulfillment_replayed: fulfillment.replayed,
        order_id: attempt.order_id.clone(),
        points_credited: fulfillment.points_credited,
        fulfillment_status: fulfillment.status,
    })
}
