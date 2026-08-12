//! Payment notify fulfillment building block（支付通知履约积木）。
//!
//! A provider webhook that confirms a successful order payment is dispatched
//! to the [`PaymentNotifyHandler`] registered for the order's business type.
//! The block is pluggable: new businesses register their own handler in a
//! [`PaymentNotifyHandlerRegistry`] wired at settlement time, and unregistered
//! business types fall back to a deterministic no-op fulfillment so the
//! payment/order status transition never blocks on missing handlers.
//!
//! Extension contract (MODULE_SPEC §6): implement `PaymentNotifyHandler` for
//! the new business type, register it via
//! `DefaultPaymentNotifyHandlerRegistry::with`, and wire the registry into
//! `settle_owner_order_after_payment_success_with_registry`. No pipeline code
//! needs to change. Handlers MUST be idempotent: every fulfillment already
//! keys on `{prefix}:{order_id}`, so provider webhook redelivery replays
//! without double effects.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use sdkwork_contract_service::CommerceServiceError;

use crate::{
    default_fulfill_account_value_order_command, default_fulfill_points_recharge_command,
    fulfill_account_value_order, fulfill_points_recharge_order,
    mark_points_recharge_payment_succeeded, membership_purchase_fulfillment_idempotency_key,
    membership_quota_recharge_idempotency_key, physical_goods_fulfillment_idempotency_key,
    points_recharge_payment_success_idempotency_key, redeem_coupon_and_fulfill_account_value_order,
    AccountPointsCreditPort, AccountValueFulfillmentStore, AccountValueLedgerPort,
    AccountValueOrderSubject, CouponRedemptionPort, FulfillPaidPhysicalOrderRequest,
    MarkPointsRechargePaymentSucceededCommand, MembershipPurchaseFulfillmentPort,
    MembershipPurchaseFulfillmentRequest, MembershipPurchaseSettlementSnapshot,
    MembershipQuotaRechargeFulfillmentRequest, OrderPaymentSettlementAttempt, OrderSubjectKind,
    OwnerOrderSettlementPorts, PhysicalGoodsFulfillmentPort, PointsRechargeFulfillmentStore,
};

/// Canonical business type for points recharge orders.
pub const PAYMENT_NOTIFY_BUSINESS_POINTS_RECHARGE: &str = "points_recharge";
/// Canonical business type for TokenBank recharge orders.
pub const PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_RECHARGE: &str = "token_bank_recharge";
/// Canonical business type for TokenBank plan purchase orders.
pub const PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_PLAN_PURCHASE: &str = "token_bank_plan_purchase";
/// Canonical business type for TokenBank plan renewal orders.
pub const PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_PLAN_RENEWAL: &str = "token_bank_plan_renewal";
/// Canonical business type for account recharge package orders.
pub const PAYMENT_NOTIFY_BUSINESS_ACCOUNT_RECHARGE_PACKAGE: &str = "account_recharge_package";
/// Canonical business type for coupon recharge orders.
pub const PAYMENT_NOTIFY_BUSINESS_COUPON_RECHARGE: &str = "coupon_recharge";
/// Canonical business type for membership purchase/activation orders.
pub const PAYMENT_NOTIFY_BUSINESS_MEMBERSHIP: &str = "membership";
/// Canonical business type for physical goods orders.
pub const PAYMENT_NOTIFY_BUSINESS_PRODUCT: &str = "product";
/// Canonical business type for virtual goods / external fulfillment orders.
pub const PAYMENT_NOTIFY_BUSINESS_EXTERNAL: &str = "external";
/// Fallback business type when the order subject cannot be resolved.
pub const PAYMENT_NOTIFY_BUSINESS_UNKNOWN: &str = "unknown";

pub type PaymentNotifyHandlerFuture<'ports, 'data> = Pin<
    Box<
        dyn Future<Output = Result<PaymentNotifyHandlingOutcome, CommerceServiceError>>
            + Send
            + 'ports,
    >,
>;

/// Standard normalized fulfillment context handed to business handlers. Every
/// fact a business flow needs to fulfill a paid order exactly once. `'ports`
/// is the settlement port aggregate lifetime; `'data` covers the business
/// facts (attempt, paid_at, snapshot, request metadata).
pub struct PaymentNotifyContext<'ports, 'data> {
    pub attempt: &'data OrderPaymentSettlementAttempt,
    pub ports: &'ports OwnerOrderSettlementPorts<'ports>,
    pub paid_at: &'data str,
    pub membership_purchase: Option<&'data MembershipPurchaseSettlementSnapshot>,
    pub request_no: &'data str,
    pub subject_kind: OrderSubjectKind,
    /// Provider webhook event type when available (e.g. refund notifications).
    pub event_type: Option<&'data str>,
}

/// Outcome of a business notify handler. The settlement flow merges these
/// fields into the canonical `OwnerOrderSettlementOutcome`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PaymentNotifyHandlingOutcome {
    pub accepted: bool,
    pub replayed: bool,
    pub points_credited: i64,
    pub status: String,
}

impl PaymentNotifyHandlingOutcome {
    /// Outcome for business types without dedicated in-process fulfillment.
    pub fn deferred(status: impl Into<String>) -> Self {
        Self {
            accepted: false,
            replayed: false,
            points_credited: 0,
            status: status.into(),
        }
    }
}

/// Business fulfillment handler for one canonical notify business type.
pub trait PaymentNotifyHandler: Send + Sync {
    fn business_type(&self) -> &'static str;

    fn handle<'ports, 'data>(
        &'ports self,
        ctx: PaymentNotifyContext<'ports, 'data>,
    ) -> PaymentNotifyHandlerFuture<'ports, 'data>
    where
        'data: 'ports;
}

/// Resolves the handler registered for a business type. Unknown business
/// types resolve to `None` and the pipeline falls back to the deterministic
/// unknown-subject fulfillment.
pub trait PaymentNotifyHandlerRegistry: Send + Sync {
    fn resolve(&self, business_type: &str) -> Option<Arc<dyn PaymentNotifyHandler>>;
}

/// Default business-type → handler registry. The built-in handlers cover every
/// `OrderSubjectKind`; additional business flows register their handlers with
/// `with` before wiring the registry into the settlement entrypoint.
pub struct DefaultPaymentNotifyHandlerRegistry {
    handlers: HashMap<String, Arc<dyn PaymentNotifyHandler>>,
}

impl DefaultPaymentNotifyHandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn with(mut self, handler: Arc<dyn PaymentNotifyHandler>) -> Self {
        self.handlers
            .insert(handler.business_type().to_owned(), handler);
        self
    }
}

impl Default for DefaultPaymentNotifyHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PaymentNotifyHandlerRegistry for DefaultPaymentNotifyHandlerRegistry {
    fn resolve(&self, business_type: &str) -> Option<Arc<dyn PaymentNotifyHandler>> {
        self.handlers.get(business_type).cloned()
    }
}

/// Factory for the standard registry: every built-in `OrderSubjectKind`
/// handler plus the documented fallbacks for external and unknown subjects.
pub fn default_payment_notify_handler_registry() -> Arc<dyn PaymentNotifyHandlerRegistry> {
    Arc::new(
        DefaultPaymentNotifyHandlerRegistry::new()
            .with(Arc::new(PointsRechargeNotifyHandler))
            .with(Arc::new(AccountValueNotifyHandler {
                business_type: PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_RECHARGE,
                account_value_subject: AccountValueOrderSubject::TokenBankRecharge,
            }))
            .with(Arc::new(AccountValueNotifyHandler {
                business_type: PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_PLAN_PURCHASE,
                account_value_subject: AccountValueOrderSubject::TokenBankPlanPurchase,
            }))
            .with(Arc::new(AccountValueNotifyHandler {
                business_type: PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_PLAN_RENEWAL,
                account_value_subject: AccountValueOrderSubject::TokenBankPlanRenewal,
            }))
            .with(Arc::new(AccountValueNotifyHandler {
                business_type: PAYMENT_NOTIFY_BUSINESS_ACCOUNT_RECHARGE_PACKAGE,
                account_value_subject: AccountValueOrderSubject::AccountRechargePackage,
            }))
            .with(Arc::new(CouponRechargeNotifyHandler))
            .with(Arc::new(MembershipNotifyHandler))
            .with(Arc::new(PhysicalGoodsNotifyHandler))
            .with(Arc::new(ExternalFulfillmentNotifyHandler))
            .with(Arc::new(UnknownSubjectNotifyHandler)),
    )
}

/// Dispatches the fulfillment of a confirmed successful payment to the
/// handler registered for the subject's business type. Unregistered business
/// types fall back to the deterministic unknown-subject fulfillment.
pub async fn dispatch_payment_notify_handler<'ports, 'data>(
    registry: &dyn PaymentNotifyHandlerRegistry,
    ports: &'ports OwnerOrderSettlementPorts<'ports>,
    subject: OrderSubjectKind,
    attempt: &'data OrderPaymentSettlementAttempt,
    paid_at: &'data str,
    membership_purchase: Option<&'data MembershipPurchaseSettlementSnapshot>,
    request_no: &'data str,
    event_type: Option<&'data str>,
) -> Result<PaymentNotifyHandlingOutcome, CommerceServiceError>
where
    'data: 'ports,
{
    let business_type = subject.business_type();
    let ctx = PaymentNotifyContext {
        attempt,
        ports,
        paid_at,
        membership_purchase,
        request_no,
        subject_kind: subject,
        event_type,
    };
    match registry.resolve(business_type) {
        Some(handler) => handler.handle(ctx).await,
        None => {
            tracing::warn!(
                target = "order.payment_notify",
                order_id = %attempt.order_id,
                business_type,
                "payment confirmed; no notify handler is registered for this business type"
            );
            Ok(PaymentNotifyHandlingOutcome::deferred(
                "awaiting_subject_resolution",
            ))
        }
    }
}

/// Points recharge fulfillment: marks the recharge payment succeeded and
/// credits the user points through the account-domain port (idempotent).
pub struct PointsRechargeNotifyHandler;

impl PaymentNotifyHandler for PointsRechargeNotifyHandler {
    fn business_type(&self) -> &'static str {
        PAYMENT_NOTIFY_BUSINESS_POINTS_RECHARGE
    }

    fn handle<'ports, 'data>(
        &'ports self,
        ctx: PaymentNotifyContext<'ports, 'data>,
    ) -> PaymentNotifyHandlerFuture<'ports, 'data>
    where
        'data: 'ports,
    {
        Box::pin(async move {
            settle_points_recharge_subject(
                ctx.ports.recharge_store,
                ctx.ports.credit_port,
                ctx.attempt,
                ctx.paid_at,
                ctx.request_no,
            )
            .await
        })
    }
}

async fn settle_points_recharge_subject<S, P>(
    recharge_store: &S,
    credit_port: &P,
    attempt: &OrderPaymentSettlementAttempt,
    paid_at: &str,
    request_no: &str,
) -> Result<PaymentNotifyHandlingOutcome, CommerceServiceError>
where
    S: PointsRechargeFulfillmentStore + ?Sized,
    P: AccountPointsCreditPort + ?Sized,
{
    let idempotency_key = points_recharge_payment_success_idempotency_key(&attempt.order_id);
    let payment_command = MarkPointsRechargePaymentSucceededCommand::new(
        &attempt.tenant_id,
        attempt.organization_id.as_deref(),
        &attempt.owner_user_id,
        &attempt.order_id,
        paid_at,
        request_no,
        &idempotency_key,
    )?;
    mark_points_recharge_payment_succeeded(recharge_store, payment_command).await?;

    let fulfill_command = default_fulfill_points_recharge_command(
        &attempt.tenant_id,
        attempt.organization_id.as_deref(),
        &attempt.owner_user_id,
        &attempt.order_id,
        request_no,
    )?;
    let fulfill_outcome =
        fulfill_points_recharge_order(recharge_store, credit_port, fulfill_command).await?;

    Ok(PaymentNotifyHandlingOutcome {
        accepted: fulfill_outcome.accepted,
        replayed: fulfill_outcome.replayed,
        points_credited: fulfill_outcome.points_credited,
        status: fulfill_outcome.fulfillment_status,
    })
}

/// Account value fulfillment for TokenBank recharge/plan and account recharge
/// package orders (idempotent ledger credit).
pub struct AccountValueNotifyHandler {
    business_type: &'static str,
    account_value_subject: AccountValueOrderSubject,
}

impl PaymentNotifyHandler for AccountValueNotifyHandler {
    fn business_type(&self) -> &'static str {
        self.business_type
    }

    fn handle<'ports, 'data>(
        &'ports self,
        ctx: PaymentNotifyContext<'ports, 'data>,
    ) -> PaymentNotifyHandlerFuture<'ports, 'data>
    where
        'data: 'ports,
    {
        let account_value_subject = self.account_value_subject;
        Box::pin(async move {
            settle_account_value_subject(
                account_value_subject,
                ctx.ports.account_value_store,
                ctx.ports.account_value_ledger_port,
                ctx.attempt,
                ctx.request_no,
            )
            .await
        })
    }
}

async fn settle_account_value_subject<A, L>(
    account_value_subject: AccountValueOrderSubject,
    account_value_store: &A,
    account_value_ledger_port: &L,
    attempt: &OrderPaymentSettlementAttempt,
    request_no: &str,
) -> Result<PaymentNotifyHandlingOutcome, CommerceServiceError>
where
    A: AccountValueFulfillmentStore + ?Sized,
    L: AccountValueLedgerPort + ?Sized,
{
    let command = default_fulfill_account_value_order_command(
        account_value_subject,
        &attempt.tenant_id,
        attempt.organization_id.as_deref(),
        &attempt.owner_user_id,
        &attempt.order_id,
        request_no,
    )?;
    let outcome =
        fulfill_account_value_order(account_value_store, account_value_ledger_port, command)
            .await?;

    Ok(PaymentNotifyHandlingOutcome {
        accepted: outcome.accepted,
        replayed: outcome.replayed,
        points_credited: 0,
        status: outcome.fulfillment_status,
    })
}

/// Coupon recharge fulfillment: redeems the coupon benefit then credits the
/// account value (idempotent).
pub struct CouponRechargeNotifyHandler;

impl PaymentNotifyHandler for CouponRechargeNotifyHandler {
    fn business_type(&self) -> &'static str {
        PAYMENT_NOTIFY_BUSINESS_COUPON_RECHARGE
    }

    fn handle<'ports, 'data>(
        &'ports self,
        ctx: PaymentNotifyContext<'ports, 'data>,
    ) -> PaymentNotifyHandlerFuture<'ports, 'data>
    where
        'data: 'ports,
    {
        Box::pin(async move {
            settle_coupon_recharge_subject(
                ctx.ports.account_value_store,
                ctx.ports.coupon_redemption_port,
                ctx.ports.account_value_ledger_port,
                ctx.attempt,
                ctx.request_no,
            )
            .await
        })
    }
}

async fn settle_coupon_recharge_subject<A, C, L>(
    account_value_store: &A,
    coupon_redemption_port: &C,
    account_value_ledger_port: &L,
    attempt: &OrderPaymentSettlementAttempt,
    request_no: &str,
) -> Result<PaymentNotifyHandlingOutcome, CommerceServiceError>
where
    A: AccountValueFulfillmentStore + ?Sized,
    C: CouponRedemptionPort + ?Sized,
    L: AccountValueLedgerPort + ?Sized,
{
    let command = default_fulfill_account_value_order_command(
        AccountValueOrderSubject::CouponRecharge,
        &attempt.tenant_id,
        attempt.organization_id.as_deref(),
        &attempt.owner_user_id,
        &attempt.order_id,
        request_no,
    )?;
    let outcome = redeem_coupon_and_fulfill_account_value_order(
        account_value_store,
        coupon_redemption_port,
        account_value_ledger_port,
        command,
    )
    .await?;
    Ok(PaymentNotifyHandlingOutcome {
        accepted: outcome.accepted,
        replayed: outcome.replayed,
        points_credited: 0,
        status: outcome.fulfillment_status,
    })
}

/// Membership purchase/activation or quota recharge fulfillment (idempotent).
/// A missing settlement snapshot degrades to `awaiting_subject_resolution`
/// instead of failing the webhook, so provider redelivery can recover once
/// the snapshot becomes available.
pub struct MembershipNotifyHandler;

impl PaymentNotifyHandler for MembershipNotifyHandler {
    fn business_type(&self) -> &'static str {
        PAYMENT_NOTIFY_BUSINESS_MEMBERSHIP
    }

    fn handle<'ports, 'data>(
        &'ports self,
        ctx: PaymentNotifyContext<'ports, 'data>,
    ) -> PaymentNotifyHandlerFuture<'ports, 'data>
    where
        'data: 'ports,
    {
        let snapshot = ctx.membership_purchase.map(|snapshot| snapshot.clone());
        Box::pin(async move {
            let Some(snapshot) = snapshot else {
                tracing::warn!(
                    target = "order.payment_notify",
                    order_id = %ctx.attempt.order_id,
                    "membership order settlement snapshot is unavailable; fulfillment deferred"
                );
                return Ok(PaymentNotifyHandlingOutcome::deferred(
                    "awaiting_subject_resolution",
                ));
            };
            settle_membership_subject(
                ctx.ports.membership_port,
                ctx.attempt,
                ctx.paid_at,
                &snapshot,
                ctx.request_no,
            )
            .await
        })
    }
}

async fn settle_membership_subject<M>(
    membership_port: &M,
    attempt: &OrderPaymentSettlementAttempt,
    paid_at: &str,
    snapshot: &MembershipPurchaseSettlementSnapshot,
    request_no: &str,
) -> Result<PaymentNotifyHandlingOutcome, CommerceServiceError>
where
    M: MembershipPurchaseFulfillmentPort + ?Sized,
{
    // 订阅期额度充值：结算时向会员权益账户追加额度（幂等）
    if snapshot.action == "recharge" {
        let quantity = snapshot.grant_quantity.ok_or_else(|| {
            CommerceServiceError::invalid_state(
                "membership quota recharge settlement snapshot has no grant quantity",
            )
        })?;
        let idempotency_key = membership_quota_recharge_idempotency_key(&attempt.order_id);
        let outcome = membership_port
            .fulfill_membership_quota_recharge(MembershipQuotaRechargeFulfillmentRequest {
                tenant_id: attempt.tenant_id.clone(),
                organization_id: attempt.organization_id.clone(),
                owner_user_id: attempt.owner_user_id.clone(),
                order_id: attempt.order_id.clone(),
                quantity,
                request_no: request_no.to_owned(),
                idempotency_key,
            })
            .await?;
        return Ok(PaymentNotifyHandlingOutcome {
            accepted: outcome.accepted,
            replayed: outcome.replayed,
            points_credited: 0,
            status: outcome.fulfillment_status,
        });
    }
    let idempotency_key = membership_purchase_fulfillment_idempotency_key(&attempt.order_id);
    let outcome = membership_port
        .fulfill_membership_purchase(MembershipPurchaseFulfillmentRequest {
            action: snapshot.action.clone(),
            tenant_id: attempt.tenant_id.clone(),
            organization_id: attempt.organization_id.clone(),
            owner_user_id: attempt.owner_user_id.clone(),
            order_id: attempt.order_id.clone(),
            order_no: snapshot.order_no.clone(),
            package_id: snapshot.package_id,
            paid_at: paid_at.to_owned(),
            request_no: request_no.to_owned(),
            idempotency_key,
        })
        .await?;

    Ok(PaymentNotifyHandlingOutcome {
        accepted: outcome.accepted,
        replayed: outcome.replayed,
        points_credited: 0,
        status: outcome.fulfillment_status,
    })
}

/// Physical goods fulfillment (idempotent; the port decides the actual
/// shipping/inventory flow).
pub struct PhysicalGoodsNotifyHandler;

impl PaymentNotifyHandler for PhysicalGoodsNotifyHandler {
    fn business_type(&self) -> &'static str {
        PAYMENT_NOTIFY_BUSINESS_PRODUCT
    }

    fn handle<'ports, 'data>(
        &'ports self,
        ctx: PaymentNotifyContext<'ports, 'data>,
    ) -> PaymentNotifyHandlerFuture<'ports, 'data>
    where
        'data: 'ports,
    {
        Box::pin(async move {
            settle_physical_goods_subject(
                ctx.ports.physical_goods_port,
                ctx.attempt,
                ctx.paid_at,
                ctx.request_no,
            )
            .await
        })
    }
}

async fn settle_physical_goods_subject<P>(
    physical_goods_port: &P,
    attempt: &OrderPaymentSettlementAttempt,
    paid_at: &str,
    request_no: &str,
) -> Result<PaymentNotifyHandlingOutcome, CommerceServiceError>
where
    P: PhysicalGoodsFulfillmentPort + ?Sized,
{
    let outcome = physical_goods_port
        .fulfill_paid_physical_order(FulfillPaidPhysicalOrderRequest {
            tenant_id: attempt.tenant_id.clone(),
            organization_id: attempt.organization_id.clone(),
            owner_user_id: attempt.owner_user_id.clone(),
            order_id: attempt.order_id.clone(),
            paid_at: paid_at.to_owned(),
            request_no: request_no.to_owned(),
            idempotency_key: physical_goods_fulfillment_idempotency_key(&attempt.order_id),
        })
        .await?;

    Ok(PaymentNotifyHandlingOutcome {
        accepted: outcome.accepted,
        replayed: outcome.replayed,
        points_credited: 0,
        status: outcome.fulfillment_status,
    })
}

/// Virtual goods / coupon package / external capability orders: payment is
/// confirmed and fulfillment is owned by external commerce capabilities.
pub struct ExternalFulfillmentNotifyHandler;

impl PaymentNotifyHandler for ExternalFulfillmentNotifyHandler {
    fn business_type(&self) -> &'static str {
        PAYMENT_NOTIFY_BUSINESS_EXTERNAL
    }

    fn handle<'ports, 'data>(
        &'ports self,
        ctx: PaymentNotifyContext<'ports, 'data>,
    ) -> PaymentNotifyHandlerFuture<'ports, 'data>
    where
        'data: 'ports,
    {
        Box::pin(async move {
            tracing::info!(
                target = "order.payment_notify",
                order_id = %ctx.attempt.order_id,
                subject = ?ctx.subject_kind,
                event_type = ctx.event_type,
                "payment confirmed; fulfillment is owned by external commerce capabilities"
            );
            Ok(PaymentNotifyHandlingOutcome::deferred(
                "awaiting_external_fulfillment",
            ))
        })
    }
}

/// Fallback handler for unresolved order subjects. Payment/order status
/// transition already happened; the fulfillment waits for subject resolution.
pub struct UnknownSubjectNotifyHandler;

impl PaymentNotifyHandler for UnknownSubjectNotifyHandler {
    fn business_type(&self) -> &'static str {
        PAYMENT_NOTIFY_BUSINESS_UNKNOWN
    }

    fn handle<'ports, 'data>(
        &'ports self,
        ctx: PaymentNotifyContext<'ports, 'data>,
    ) -> PaymentNotifyHandlerFuture<'ports, 'data>
    where
        'data: 'ports,
    {
        Box::pin(async move {
            tracing::warn!(
                target = "order.payment_notify",
                order_id = %ctx.attempt.order_id,
                "payment confirmed; order subject is missing or unsupported for automated fulfillment"
            );
            Ok(PaymentNotifyHandlingOutcome::deferred(
                "awaiting_subject_resolution",
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_resolves_built_in_business_types() {
        let registry = default_payment_notify_handler_registry();
        for business_type in [
            PAYMENT_NOTIFY_BUSINESS_POINTS_RECHARGE,
            PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_RECHARGE,
            PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_PLAN_PURCHASE,
            PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_PLAN_RENEWAL,
            PAYMENT_NOTIFY_BUSINESS_ACCOUNT_RECHARGE_PACKAGE,
            PAYMENT_NOTIFY_BUSINESS_COUPON_RECHARGE,
            PAYMENT_NOTIFY_BUSINESS_MEMBERSHIP,
            PAYMENT_NOTIFY_BUSINESS_PRODUCT,
            PAYMENT_NOTIFY_BUSINESS_EXTERNAL,
            PAYMENT_NOTIFY_BUSINESS_UNKNOWN,
        ] {
            let handler = registry
                .resolve(business_type)
                .unwrap_or_else(|| panic!("{business_type} must be registered"));
            assert_eq!(business_type, handler.business_type());
        }
    }

    #[test]
    fn registry_unregistered_business_type_resolves_to_none() {
        let registry = default_payment_notify_handler_registry();
        assert!(registry.resolve("membership_renewal").is_none());
    }

    #[test]
    fn order_subject_business_type_mapping_is_complete() {
        for (subject, expected) in [
            ("points_recharge", PAYMENT_NOTIFY_BUSINESS_POINTS_RECHARGE),
            (
                "token_bank_recharge",
                PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_RECHARGE,
            ),
            (
                "token_bank_plan_purchase",
                PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_PLAN_PURCHASE,
            ),
            (
                "token_bank_plan_renewal",
                PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_PLAN_RENEWAL,
            ),
            (
                "account_recharge_package",
                PAYMENT_NOTIFY_BUSINESS_ACCOUNT_RECHARGE_PACKAGE,
            ),
            ("coupon_recharge", PAYMENT_NOTIFY_BUSINESS_COUPON_RECHARGE),
            ("membership", PAYMENT_NOTIFY_BUSINESS_MEMBERSHIP),
            ("product", PAYMENT_NOTIFY_BUSINESS_PRODUCT),
            ("virtual_goods", PAYMENT_NOTIFY_BUSINESS_EXTERNAL),
            ("coupon_package", PAYMENT_NOTIFY_BUSINESS_EXTERNAL),
            ("some_machine_flow", PAYMENT_NOTIFY_BUSINESS_EXTERNAL),
            ("display title", PAYMENT_NOTIFY_BUSINESS_UNKNOWN),
        ] {
            let kind = OrderSubjectKind::parse(Some(subject));
            assert_eq!(
                expected,
                kind.business_type(),
                "subject {subject:?} must map to {expected}"
            );
        }
    }

    #[test]
    fn deferred_outcome_reports_no_credit_and_deferred_status() {
        let outcome = PaymentNotifyHandlingOutcome::deferred("awaiting_subject_resolution");
        assert!(!outcome.accepted);
        assert!(!outcome.replayed);
        assert_eq!(0, outcome.points_credited);
        assert_eq!("awaiting_subject_resolution", outcome.status);
    }
}
