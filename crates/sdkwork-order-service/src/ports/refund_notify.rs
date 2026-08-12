//! Refund notify orchestration ports（退款通知框架端口）。
//!
//! The refund notify flow system mirrors the payment notify building block
//! but is an independent pipeline: provider refund notifications arrive at
//! their own URL (`/app/v3/api/orders/refunds/webhooks/{providerCode}`),
//! verify through the shared verification port, ingest through the
//! payment-domain refund ingestion (which advances the `commerce_refund`
//! status machine), and settle the order-side refund state through
//! [`RefundNotifyStatePort`]. Order domain owns the order state; the payment
//! domain owns the refund facts — the two sides only communicate through
//! these ports (high cohesion, low coupling).

use std::future::Future;
use std::pin::Pin;

use sdkwork_contract_service::CommerceServiceError;

use crate::ports::PaymentNotifyAttemptContext;
use crate::OrderPaymentSettlementAttempt;

/// Refund fact resolved by the payment-domain refund ingestion after the
/// `commerce_refund` status machine applied the provider notification.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefundNotifyContext {
    pub refund_id: String,
    pub refund_no: String,
    pub order_id: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    /// Applied commerce refund status (`succeeded`/`failed`/`canceled`/`processing`).
    pub status: String,
    pub amount: String,
    /// Canonical business type for refund post-processing. Defaults to
    /// `refund`; business flows (e.g. account-value refund hold release,
    /// after-sales linkage) register a `RefundNotifyHandler` for their type.
    pub business_type: String,
}

/// Canonical default business type for refund post-processing.
pub const REFUND_NOTIFY_BUSINESS_REFUND: &str = "refund";

pub type RefundNotifyHandlerFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), CommerceServiceError>> + Send + 'a>>;

/// Business-specific refund post-processing hook. The canonical order
/// `refund_status` update always runs in the pipeline; handlers add
/// business effects (releasing holds, linking after-sales requests) and MUST
/// be idempotent (stable keys per refund).
pub trait RefundNotifyHandler: Send + Sync {
    fn business_type(&self) -> &'static str;

    fn handle<'a>(
        &'a self,
        ctx: &'a RefundNotifyContext,
        attempt: &'a PaymentNotifyAttemptContext,
    ) -> RefundNotifyHandlerFuture<'a>;
}

/// Resolves the handler registered for a refund business type. Unknown types
/// resolve to `None` and the pipeline completes without post-processing.
pub trait RefundNotifyHandlerRegistry: Send + Sync {
    fn resolve(&self, business_type: &str) -> Option<std::sync::Arc<dyn RefundNotifyHandler>>;
}

/// Outcome of persisting a refund notification (idempotent ingest).
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefundNotifyIngestOutcome {
    pub webhook_event_id: String,
    pub replayed: bool,
    pub refund: Option<RefundNotifyContext>,
    /// Original payment attempt context when the out-trade-no resolves, so
    /// the order side can scope the order-state write.
    pub payment_attempt: Option<PaymentNotifyAttemptContext>,
}

pub type RefundNotifyIngestFuture<'a> = Pin<
    Box<dyn Future<Output = Result<RefundNotifyIngestOutcome, CommerceServiceError>> + Send + 'a>,
>;

/// Persists the refund notification idempotently and advances the refund
/// status machine (payment domain implementation).
pub trait RefundNotifyIngestPort: Send + Sync {
    fn ingest<'a>(
        &'a self,
        event: crate::ports::PaymentNotifyEvent,
    ) -> RefundNotifyIngestFuture<'a>;
}

/// Outcome of advancing the owner order refund state.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OwnerOrderRefundStateOutcome {
    pub refund_status: String,
    /// True when the order was already in a terminal refund state and the
    /// write was suppressed (idempotent replay protection).
    pub terminal_preserved: bool,
}

pub type OwnerOrderRefundStateFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<OwnerOrderRefundStateOutcome, CommerceServiceError>> + Send + 'a,
    >,
>;

/// Order-owned persistence boundary for the refund-notify part of the flow.
/// Only the order repository advances `commerce_order.refund_status`.
///
/// The default implementation fails loudly so deployments without the
/// order-owned Postgres store notice that refund-state marking is
/// unconfigured instead of silently acking the notification.
pub trait RefundNotifyStatePort: Send + Sync {
    fn mark_owner_order_refund_status<'a>(
        &'a self,
        _attempt: &'a OrderPaymentSettlementAttempt,
        _refund_status: &'a str,
    ) -> OwnerOrderRefundStateFuture<'a> {
        Box::pin(async move {
            Err(CommerceServiceError::provider_unavailable(
                "order refund state marking is not configured",
            ))
        })
    }
}
