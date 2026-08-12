pub use sdkwork_payment_service::{
    ConfirmOwnerOrderPaymentOutcome, OrderPaymentSettlementAttempt,
    OwnerOrderPaymentConfirmationFuture, OwnerOrderPaymentConfirmationPort,
    OWNER_ORDER_PAYMENT_CONFIRMATION_PORT,
};

use sdkwork_contract_service::CommerceServiceError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconcileOwnerOrderPaymentRequest {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconcileOwnerOrderPaymentOutcome {
    pub attempt: OrderPaymentSettlementAttempt,
    pub provider_code: String,
    pub provider_status: String,
    pub replayed: bool,
}

/// Resolves an exact attempt and confirms the PSP state before manual recovery.
pub trait OwnerOrderPaymentReconciliationPort: Send + Sync {
    fn reconcile_owner_order_payment<'a>(
        &'a self,
        request: ReconcileOwnerOrderPaymentRequest,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, ReconcileOwnerOrderPaymentOutcome>;
}

#[derive(Default)]
pub struct UnavailableOwnerOrderPaymentReconciliationPort;

impl OwnerOrderPaymentReconciliationPort for UnavailableOwnerOrderPaymentReconciliationPort {
    fn reconcile_owner_order_payment<'a>(
        &'a self,
        _request: ReconcileOwnerOrderPaymentRequest,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, ReconcileOwnerOrderPaymentOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::provider_unavailable(
                "payment provider reconciliation is not configured",
            ))
        })
    }
}

pub const OWNER_ORDER_PAYMENT_RECONCILIATION_PORT: &str =
    "payment.owner_order_payment.reconciliation";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnerOrderPaymentStateOutcome {
    pub order_status: String,
    pub terminal_order_preserved: bool,
}

/// Order-owned persistence boundary for the payment-success/failure parts of
/// settlement.
///
/// Payment confirms the provider intent/attempt, while Order remains the only
/// owner allowed to advance `commerce_order` lifecycle state.
pub trait OwnerOrderPaymentStatePort: Send + Sync {
    fn mark_owner_order_payment_succeeded<'a>(
        &'a self,
        attempt: &'a OrderPaymentSettlementAttempt,
        paid_at: &'a str,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, OwnerOrderPaymentStateOutcome>;

    /// Marks the owner order failed/cancelled after a provider payment failure
    /// or closure webhook. The transition is idempotent: a terminal order
    /// (already paid, fulfilled, or cancelled) is preserved and reported via
    /// `terminal_order_preserved`, so a late failure callback can never
    /// overwrite a confirmed success.
    ///
    /// The default implementation fails loudly so deployments without the
    /// order-owned Postgres store notice that failure-state marking is
    /// unconfigured instead of silently acking the webhook.
    fn mark_owner_order_payment_failed<'a>(
        &'a self,
        _attempt: &'a OrderPaymentSettlementAttempt,
        _failure_status: &'a str,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, OwnerOrderPaymentStateOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::provider_unavailable(
                "order payment failure state marking is not configured",
            ))
        })
    }
}

pub const OWNER_ORDER_PAYMENT_STATE_PORT: &str = "order.owner_order_payment.state";
