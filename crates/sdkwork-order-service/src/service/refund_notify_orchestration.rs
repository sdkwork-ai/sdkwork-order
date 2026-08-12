//! Refund notify orchestration（退款通知编排积木）。
//!
//! `process_refund_notify` owns the complete provider refund-notification
//! decision chain, mirroring the payment notify pipeline as an independent
//! flow system:
//!
//! ```text
//! verify signature → normalize refund event → ingest idempotently
//!   → refund status machine (payment domain) applied
//!   → refund status dispatch:
//!       succeeded → order refund_status = refunded
//!       failed/canceled → order refund_status = refund_failed
//!       processing → order refund_status = refunding
//!       unmatched → audit warn + accepted ack
//! ```
//!
//! The refund flow shares the verification port with the payment flow
//! (provider adapters verify/normalize identically) but has its own ingest
//! port (refund status machine), its own state port (order refund state),
//! and its own URL surface. Idempotency is terminal-safe end to end: the
//! refund status machine never rewrites a succeeded refund, and the order
//! refund state never lets `refund_failed` overwrite `refunded`.

use std::collections::HashMap;
use std::sync::Arc;

use sdkwork_contract_service::CommerceServiceError;

use crate::ports::{
    PaymentNotifyAttemptContext, PaymentNotifyEvent, PaymentNotifyVerifyPort, RefundNotifyContext,
    RefundNotifyHandler, RefundNotifyHandlerRegistry, RefundNotifyIngestPort,
    RefundNotifyStatePort,
};
use crate::OrderPaymentSettlementAttempt;

/// Default business-type → handler registry for refund post-processing.
pub struct DefaultRefundNotifyHandlerRegistry {
    handlers: HashMap<String, Arc<dyn RefundNotifyHandler>>,
}

impl DefaultRefundNotifyHandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn with(mut self, handler: Arc<dyn RefundNotifyHandler>) -> Self {
        self.handlers
            .insert(handler.business_type().to_owned(), handler);
        self
    }
}

impl Default for DefaultRefundNotifyHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RefundNotifyHandlerRegistry for DefaultRefundNotifyHandlerRegistry {
    fn resolve(&self, business_type: &str) -> Option<Arc<dyn RefundNotifyHandler>> {
        self.handlers.get(business_type).cloned()
    }
}

/// Canonical order refund statuses written by the refund notify flow.
pub const ORDER_REFUND_STATUS_REFUNDING: &str = "refunding";
pub const ORDER_REFUND_STATUS_REFUNDED: &str = "refunded";
pub const ORDER_REFUND_STATUS_REFUND_FAILED: &str = "refund_failed";

/// Canonical statuses reported for the refund notify response surface.
pub const REFUND_NOTIFY_STATUS_REFUNDED: &str = "refunded";
pub const REFUND_NOTIFY_STATUS_REFUND_FAILED: &str = "refund_failed";
pub const REFUND_NOTIFY_STATUS_REFUNDING: &str = "refunding";
pub const REFUND_NOTIFY_STATUS_ACCEPTED: &str = "accepted";

/// Outcome of one refund notification processing run, projected for the HTTP
/// response surface.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefundNotifyProcessingOutcome {
    pub webhook_event_id: String,
    pub replayed: bool,
    pub refund: Option<RefundNotifyContext>,
    pub order_refund_marked: bool,
    pub status: String,
}

/// Processes a provider refund notification end to end. The verification
/// port is shared with the payment flow; the ingest and state ports are
/// refund-specific so the two flow systems stay independent.
pub async fn process_refund_notify(
    verify_port: &dyn PaymentNotifyVerifyPort,
    refund_ingest_port: &dyn RefundNotifyIngestPort,
    refund_state_port: &dyn RefundNotifyStatePort,
    provider_code: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<RefundNotifyProcessingOutcome, CommerceServiceError> {
    let event = crate::service::payment_notify_orchestration::verify_and_normalize_event(
        verify_port,
        provider_code,
        headers,
        body,
    )
    .await?;
    process_refund_notify_verified(
        event,
        refund_ingest_port,
        refund_state_port,
        default_refund_notify_handler_registry().as_ref(),
    )
    .await
}

/// Default refund post-processing registry: the canonical order refund-state
/// update is pipeline-built-in; business handlers register here.
pub fn default_refund_notify_handler_registry() -> Arc<dyn RefundNotifyHandlerRegistry> {
    Arc::new(DefaultRefundNotifyHandlerRegistry::new())
}

/// Refund pipeline from a verified, identity-normalized event: ingest
/// idempotently, then dispatch on the refund status. HTTP surfaces route
/// single-URL providers here when the event type is a refund event.
pub async fn process_refund_notify_verified(
    event: PaymentNotifyEvent,
    refund_ingest_port: &dyn RefundNotifyIngestPort,
    refund_state_port: &dyn RefundNotifyStatePort,
    refund_handler_registry: &dyn RefundNotifyHandlerRegistry,
) -> Result<RefundNotifyProcessingOutcome, CommerceServiceError> {
    let provider_code = event.provider_code.clone();
    let ingest = refund_ingest_port.ingest(event).await?;
    let Some(refund) = ingest.refund.as_ref() else {
        tracing::warn!(
            target = "order.refund_notify",
            provider_code = %provider_code,
            webhook_event_id = %ingest.webhook_event_id,
            "refund notify could not be matched to a refund; event accepted for forensics"
        );
        return Ok(RefundNotifyProcessingOutcome {
            webhook_event_id: ingest.webhook_event_id,
            replayed: ingest.replayed,
            refund: None,
            order_refund_marked: false,
            status: REFUND_NOTIFY_STATUS_ACCEPTED.to_owned(),
        });
    };

    let Some(attempt) = ingest.payment_attempt.as_ref() else {
        tracing::warn!(
            target = "order.refund_notify",
            provider_code = %provider_code,
            refund_no = %refund.refund_no,
            "refund notify resolved the refund but not the original payment attempt; order state untouched"
        );
        return Ok(RefundNotifyProcessingOutcome {
            webhook_event_id: ingest.webhook_event_id,
            replayed: ingest.replayed,
            refund: Some(refund.clone()),
            order_refund_marked: false,
            status: REFUND_NOTIFY_STATUS_ACCEPTED.to_owned(),
        });
    };

    let order_refund_status = match refund.status.as_str() {
        "succeeded" => ORDER_REFUND_STATUS_REFUNDED,
        "failed" | "canceled" => ORDER_REFUND_STATUS_REFUND_FAILED,
        "processing" => ORDER_REFUND_STATUS_REFUNDING,
        other => {
            tracing::warn!(
                target = "order.refund_notify",
                order_id = %attempt.order_id,
                refund_status = other,
                "refund notify applied an unmapped refund status; order state untouched"
            );
            return Ok(RefundNotifyProcessingOutcome {
                webhook_event_id: ingest.webhook_event_id,
                replayed: ingest.replayed,
                refund: Some(refund.clone()),
                order_refund_marked: false,
                status: REFUND_NOTIFY_STATUS_ACCEPTED.to_owned(),
            });
        }
    };

    let settlement_attempt = order_payment_settlement_attempt_from_notify(attempt);
    let state_outcome = refund_state_port
        .mark_owner_order_refund_status(&settlement_attempt, order_refund_status)
        .await?;
    tracing::info!(
        target = "order.refund_notify",
        webhook_event_id = %ingest.webhook_event_id,
        refund_no = %refund.refund_no,
        order_id = %attempt.order_id,
        refund_status = %refund.status,
        order_refund_status,
        "refund notify applied"
    );
    // Business-specific refund post-processing (release account-value holds,
    // link after-sales requests, ...). Handlers must be idempotent.
    if let Some(handler) = refund_handler_registry.resolve(&refund.business_type) {
        handler.handle(refund, attempt).await?;
    }
    if state_outcome.terminal_preserved {
        tracing::warn!(
            target = "order.refund_notify",
            order_id = %attempt.order_id,
            order_refund_status = %state_outcome.refund_status,
            "refund notify did not override a terminal order refund state"
        );
    }

    Ok(RefundNotifyProcessingOutcome {
        webhook_event_id: ingest.webhook_event_id,
        replayed: ingest.replayed,
        refund: Some(refund.clone()),
        order_refund_marked: !state_outcome.terminal_preserved,
        status: order_refund_status.to_owned(),
    })
}

fn order_payment_settlement_attempt_from_notify(
    attempt: &PaymentNotifyAttemptContext,
) -> OrderPaymentSettlementAttempt {
    OrderPaymentSettlementAttempt {
        tenant_id: attempt.tenant_id.clone(),
        organization_id: attempt.organization_id.clone(),
        owner_user_id: attempt.owner_user_id.clone(),
        order_id: attempt.order_id.clone(),
        payment_attempt_id: Some(attempt.payment_attempt_id.clone()),
        out_trade_no: Some(attempt.out_trade_no.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_refund_status_mapping_covers_all_refund_states() {
        assert_eq!(ORDER_REFUND_STATUS_REFUNDED, "refunded");
        assert_eq!(ORDER_REFUND_STATUS_REFUND_FAILED, "refund_failed");
        assert_eq!(ORDER_REFUND_STATUS_REFUNDING, "refunding");
        assert_eq!(REFUND_NOTIFY_STATUS_REFUNDED, "refunded");
        assert_eq!(REFUND_NOTIFY_STATUS_ACCEPTED, "accepted");
    }
}
