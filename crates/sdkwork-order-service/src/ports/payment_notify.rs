//! Payment notify orchestration ports.
//!
//! The webhook pipeline is owned by order-service: HTTP surfaces implement
//! these ports and delegate the whole verify → ingest → settle/fail decision
//! chain to `service::payment_notify_orchestration::process_payment_notify`.
//! The DTOs below are order-domain projections of the provider/payment facts,
//! so the service crate stays independent of provider adapters and the
//! payment repository implementation (RUST_CODE_SPEC layer roles).

use std::future::Future;
use std::pin::Pin;

use sdkwork_contract_service::CommerceServiceError;

use crate::MembershipPurchaseSettlementSnapshot;

/// Normalized provider webhook event produced by the verification port.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PaymentNotifyEvent {
    pub provider_code: String,
    /// Provider event id when the provider supplies one; the orchestrator
    /// derives a stable fallback when absent.
    pub provider_event_id: Option<String>,
    pub event_type: Option<String>,
    pub out_trade_no: Option<String>,
    pub payment_status: Option<String>,
    pub payload: serde_json::Value,
    /// Scope resolved by the verification port (tenant account binding or the
    /// attempt fallback); used to persist events when the out-trade-no cannot
    /// be resolved to an attempt yet.
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
}

/// Exact payment attempt resolved by the ingest port.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PaymentNotifyAttemptContext {
    pub payment_attempt_id: String,
    pub out_trade_no: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
}

/// Outcome of persisting a normalized webhook event (idempotent ingest).
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PaymentNotifyIngestOutcome {
    pub webhook_event_id: String,
    pub replayed: bool,
    pub applied_status: Option<String>,
    /// False when the webhook status conflicted with a terminal payment state
    /// and nothing was transitioned — ack without re-entering settlement.
    pub applied: bool,
    pub attempt: Option<PaymentNotifyAttemptContext>,
}

/// Order settlement facts loaded for the payment attempt's order.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PaymentNotifyOrderContext {
    pub subject: String,
    pub membership_purchase: Option<MembershipPurchaseSettlementSnapshot>,
}

pub type PaymentNotifyVerifyFuture<'a> =
    Pin<Box<dyn Future<Output = Result<PaymentNotifyEvent, CommerceServiceError>> + Send + 'a>>;
pub type PaymentNotifyIngestFuture<'a> = Pin<
    Box<dyn Future<Output = Result<PaymentNotifyIngestOutcome, CommerceServiceError>> + Send + 'a>,
>;
pub type PaymentNotifyOrderContextFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<Option<PaymentNotifyOrderContext>, CommerceServiceError>>
            + Send
            + 'a,
    >,
>;

/// Verifies the provider signature and normalizes the raw webhook into the
/// order-domain event projection.
pub trait PaymentNotifyVerifyPort: Send + Sync {
    fn verify_and_normalize<'a>(
        &'a self,
        provider_code: &'a str,
        headers: &'a [(String, String)],
        body: &'a [u8],
    ) -> PaymentNotifyVerifyFuture<'a>;
}

/// Persists the normalized webhook event idempotently and applies the provider
/// payment status to the exact attempt/intent.
pub trait PaymentNotifyIngestPort: Send + Sync {
    fn ingest<'a>(&'a self, event: PaymentNotifyEvent) -> PaymentNotifyIngestFuture<'a>;
}

/// Loads the settlement facts (subject + membership snapshot) for an order.
pub trait PaymentNotifyOrderContextPort: Send + Sync {
    fn load_order_settlement_context<'a>(
        &'a self,
        tenant_id: &'a str,
        organization_id: Option<&'a str>,
        order_id: &'a str,
    ) -> PaymentNotifyOrderContextFuture<'a>;
}
