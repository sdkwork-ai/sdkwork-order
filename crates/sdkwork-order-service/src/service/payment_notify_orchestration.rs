//! Payment notify orchestration（支付通知编排积木）。
//!
//! `process_payment_notify` owns the complete provider-webhook decision chain
//! that HTTP surfaces delegate to:
//!
//! ```text
//! verify signature → normalize event → ingest idempotently
//!   → load order settlement context
//!   → applied_status dispatch:
//!       succeeded        → settle order (notify handler registry)
//!       failed/canceled  → mark owner order failed (idempotent, terminal-safe)
//!       unmapped/other   → audit warn + accepted ack (no PSP retry storm)
//! ```
//!
//! The at-least-once contract: the ingest port commits the webhook event
//! before settlement, so a settlement failure surfaces as a server error and
//! the provider redelivers the same event; the replay path re-applies the
//! stored status and re-enters settlement, which is idempotent end to end
//! (`{prefix}:{order_id}` fulfillment keys, terminal-order guards).

use sdkwork_contract_service::CommerceServiceError;

use sdkwork_utils_rust::sha256_hash;

use crate::ports::{
    PaymentNotifyAttemptContext, PaymentNotifyEvent, PaymentNotifyIngestPort,
    PaymentNotifyOrderContextPort, PaymentNotifyVerifyPort,
};
use crate::{
    settle_owner_order_after_payment_success_with_registry, OrderPaymentSettlementAttempt,
    OwnerOrderSettlementOutcome, OwnerOrderSettlementPorts, PaymentNotifyHandlerRegistry,
};

/// Canonical statuses reported for the notify response surface.
pub const PAYMENT_NOTIFY_STATUS_SETTLED: &str = "settled";
pub const PAYMENT_NOTIFY_STATUS_ORDER_FAILURE_MARKED: &str = "order_failure_marked";
pub const PAYMENT_NOTIFY_STATUS_ACCEPTED: &str = "accepted";
pub const PAYMENT_NOTIFY_STATUS_UNMAPPED: &str = "unmapped";

/// Outcome of one payment notify processing run, projected for the HTTP
/// response surface.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PaymentNotifyProcessingOutcome {
    pub webhook_event_id: String,
    pub payment_attempt_id: Option<String>,
    pub replayed: bool,
    pub applied_status: Option<String>,
    pub settlement: Option<OwnerOrderSettlementOutcome>,
    pub order_failure_marked: bool,
    pub status: String,
}

/// True when the normalized event type belongs to the refund notification
/// family (WeChat `REFUND.*`, Stripe `charge.refunded`, ...). Used by HTTP
/// surfaces to route single-URL providers (WeChat sends payment and refund
/// notifications to the same configured URL).
pub fn is_refund_event_type(event_type: Option<&str>) -> bool {
    event_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some_and(|value| value.to_ascii_lowercase().contains("refund"))
}

/// Verifies the provider signature and normalizes the event, then derives a
/// stable event identity. The fallback identity includes the event type and a
/// payload fingerprint so providers without event ids (sandbox, some Alipay
/// flows) still deduplicate redeliveries of the SAME event while distinct
/// events for one out-trade-no stay distinct.
pub async fn verify_and_normalize_event(
    verify_port: &dyn PaymentNotifyVerifyPort,
    provider_code: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<PaymentNotifyEvent, CommerceServiceError> {
    let mut event = verify_port
        .verify_and_normalize(provider_code, headers, body)
        .await?;
    normalize_event_identity(&mut event, provider_code);
    Ok(event)
}

/// Derives the stable event identity fallback when the provider supplies no
/// event id.
pub fn normalize_event_identity(event: &mut PaymentNotifyEvent, provider_code: &str) {
    if event
        .provider_event_id
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return;
    }
    let out_trade_no = event
        .out_trade_no
        .as_deref()
        .unwrap_or("unknown-out-trade-no");
    let event_type = event
        .event_type
        .as_deref()
        .unwrap_or("payment")
        .to_ascii_lowercase();
    let payload_fingerprint = payload_fingerprint(&event.payload);
    event.provider_event_id = Some(format!(
        "{provider_code}:{out_trade_no}:{event_type}:{payload_fingerprint}"
    ));
}

fn payload_fingerprint(payload: &serde_json::Value) -> String {
    let canonical = serde_json::to_string(payload).unwrap_or_default();
    let digest = sha256_hash(canonical.as_bytes());
    digest.chars().take(16).collect()
}

/// Processes a provider payment notify end to end (verify → ingest → settle).
/// Kept for direct callers; HTTP surfaces should use
/// [`verify_and_normalize_event`] + [`process_payment_notify_verified`] so
/// they can route refund events to the refund pipeline first.
pub async fn process_payment_notify(
    verify_port: &dyn PaymentNotifyVerifyPort,
    ingest_port: &dyn PaymentNotifyIngestPort,
    order_context_port: &dyn PaymentNotifyOrderContextPort,
    settlement_ports: OwnerOrderSettlementPorts<'_>,
    notify_handler_registry: &dyn PaymentNotifyHandlerRegistry,
    provider_code: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<PaymentNotifyProcessingOutcome, CommerceServiceError> {
    let event = verify_and_normalize_event(verify_port, provider_code, headers, body).await?;
    process_payment_notify_verified(
        event,
        ingest_port,
        order_context_port,
        settlement_ports,
        notify_handler_registry,
    )
    .await
}

/// Payment pipeline from a verified, identity-normalized event: ingest
/// idempotently, then dispatch on the applied status.
pub async fn process_payment_notify_verified(
    event: PaymentNotifyEvent,
    ingest_port: &dyn PaymentNotifyIngestPort,
    order_context_port: &dyn PaymentNotifyOrderContextPort,
    settlement_ports: OwnerOrderSettlementPorts<'_>,
    notify_handler_registry: &dyn PaymentNotifyHandlerRegistry,
) -> Result<PaymentNotifyProcessingOutcome, CommerceServiceError> {
    let ingest = ingest_port.ingest(event.clone()).await?;
    let Some(attempt) = ingest.attempt.as_ref() else {
        // No exact attempt matched the webhook (or the provider status could
        // not be applied). Ack without side effects so the provider does not
        // retry forever; the event is already persisted for forensics.
        if let Some(status) = ingest.applied_status.as_deref() {
            tracing::warn!(
                target = "order.payment_notify",
                provider_code = %event.provider_code,
                applied_status = status,
                webhook_event_id = %ingest.webhook_event_id,
                "payment notify applied a status without an exact order attempt"
            );
        } else {
            tracing::warn!(
                target = "order.payment_notify",
                provider_code = %event.provider_code,
                webhook_event_id = %ingest.webhook_event_id,
                event_type = event.event_type.as_deref(),
                "payment notify status could not be mapped; event accepted for forensics"
            );
        }
        let status = if ingest.applied_status.is_some() {
            PAYMENT_NOTIFY_STATUS_ACCEPTED.to_string()
        } else {
            PAYMENT_NOTIFY_STATUS_UNMAPPED.to_string()
        };
        return Ok(PaymentNotifyProcessingOutcome {
            webhook_event_id: ingest.webhook_event_id,
            payment_attempt_id: None,
            replayed: ingest.replayed,
            applied_status: ingest.applied_status,
            settlement: None,
            order_failure_marked: false,
            status,
        });
    };

    if !ingest.applied {
        // Terminal-conflict ack: a stale webhook (e.g. failed after success)
        // conflicted with the terminal payment state; the event is persisted
        // but nothing was transitioned. Ack with the current status WITHOUT
        // re-entering settlement or failure marking.
        tracing::warn!(
            target = "order.payment_notify",
            order_id = %attempt.order_id,
            applied_status = ingest.applied_status.as_deref(),
            "payment notify conflicted with a terminal payment state; acked without effects"
        );
        return Ok(PaymentNotifyProcessingOutcome {
            webhook_event_id: ingest.webhook_event_id,
            payment_attempt_id: Some(attempt.payment_attempt_id.clone()),
            replayed: ingest.replayed,
            applied_status: ingest.applied_status,
            settlement: None,
            order_failure_marked: false,
            status: PAYMENT_NOTIFY_STATUS_ACCEPTED.to_owned(),
        });
    }

    let context = order_context_port
        .load_order_settlement_context(
            &attempt.tenant_id,
            attempt.organization_id.as_deref(),
            &attempt.order_id,
        )
        .await?;
    let Some(context) = context else {
        return Err(CommerceServiceError::not_found(
            "order was not found for payment webhook",
        ));
    };

    let settlement_attempt = order_payment_settlement_attempt_from_notify(attempt);
    let request_no = format!("webhook:{}", ingest.webhook_event_id);

    match ingest.applied_status.as_deref() {
        Some("succeeded") => {
            let settlement = settle_owner_order_after_payment_success_with_registry(
                settlement_ports,
                &settlement_attempt,
                Some(context.subject.as_str()),
                context.membership_purchase.as_ref(),
                &request_no,
                event.event_type.as_deref(),
                notify_handler_registry,
            )
            .await?;
            tracing::info!(
                target = "order.payment_notify",
                webhook_event_id = %ingest.webhook_event_id,
                order_id = %attempt.order_id,
                payment_attempt_id = %attempt.payment_attempt_id,
                replayed = ingest.replayed,
                fulfillment_status = %settlement.fulfillment_status,
                "payment notify settled"
            );
            Ok(PaymentNotifyProcessingOutcome {
                webhook_event_id: ingest.webhook_event_id,
                payment_attempt_id: Some(attempt.payment_attempt_id.clone()),
                replayed: ingest.replayed,
                applied_status: ingest.applied_status,
                settlement: Some(settlement),
                order_failure_marked: false,
                status: PAYMENT_NOTIFY_STATUS_SETTLED.to_string(),
            })
        }
        Some(status @ ("failed" | "canceled" | "closed")) => {
            let failure_outcome = settlement_ports
                .order_state_store
                .mark_owner_order_payment_failed(&settlement_attempt, status)
                .await?;
            if failure_outcome.terminal_order_preserved {
                tracing::warn!(
                    target = "order.payment_notify",
                    order_id = %attempt.order_id,
                    order_status = %failure_outcome.order_status,
                    applied_status = status,
                    "payment failure notify did not override a terminal order state"
                );
            }
            Ok(PaymentNotifyProcessingOutcome {
                webhook_event_id: ingest.webhook_event_id,
                payment_attempt_id: Some(attempt.payment_attempt_id.clone()),
                replayed: ingest.replayed,
                applied_status: ingest.applied_status,
                settlement: None,
                order_failure_marked: !failure_outcome.terminal_order_preserved,
                status: PAYMENT_NOTIFY_STATUS_ORDER_FAILURE_MARKED.to_string(),
            })
        }
        Some(other) => {
            tracing::warn!(
                target = "order.payment_notify",
                order_id = %attempt.order_id,
                applied_status = other,
                "payment notify applied an unmapped status; order state untouched"
            );
            Ok(PaymentNotifyProcessingOutcome {
                webhook_event_id: ingest.webhook_event_id,
                payment_attempt_id: Some(attempt.payment_attempt_id.clone()),
                replayed: ingest.replayed,
                applied_status: ingest.applied_status,
                settlement: None,
                order_failure_marked: false,
                status: PAYMENT_NOTIFY_STATUS_ACCEPTED.to_string(),
            })
        }
        None => {
            tracing::warn!(
                target = "order.payment_notify",
                order_id = %attempt.order_id,
                "payment notify carried no applicable status; order state untouched"
            );
            Ok(PaymentNotifyProcessingOutcome {
                webhook_event_id: ingest.webhook_event_id,
                payment_attempt_id: Some(attempt.payment_attempt_id.clone()),
                replayed: ingest.replayed,
                applied_status: None,
                settlement: None,
                order_failure_marked: false,
                status: PAYMENT_NOTIFY_STATUS_UNMAPPED.to_string(),
            })
        }
    }
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
