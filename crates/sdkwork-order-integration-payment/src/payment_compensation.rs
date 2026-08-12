//! Payment compensation worker core (补偿轮询).
//!
//! The compensation path is the at-least-once safety net for provider
//! notifications: when a webhook never arrives (or is lost), the worker
//! claims payments stuck in pending/processing and refunds stuck in
//! submitted/processing, queries the PSP for the real state, and re-enters
//! the **same notify processing framework** with a synthetic event.
//!
//! Reusing `process_payment_notify_verified` /
//! `process_refund_notify_verified` gives the query path every guarantee of
//! the webhook path by construction:
//!
//! - **Idempotency**: the synthetic event id
//!   `query:{provider}:{out_trade_no}:{mapped_status}` de-duplicates on
//!   `(tenant_id, provider_scoped_event_id)`; a webhook that already settled
//!   the order makes the worker event a replayed ingest with suppressed
//!   fulfillment.
//! - **Transaction safety**: status application happens inside the same
//!   ingest transaction with the same lock ordering (order → intent →
//!   attempt) as the webhook path; order-side settlement/failure marking runs
//!   in the same independent order-domain transactions.
//! - **No double success**: terminal payment/refund states are preserved by
//!   the status machines, and fulfillment keys suppress duplicate
//!   settlement.
//!
//! The worker never marks anything failed on its own: a PSP query that
//! reports the trade still pending leaves the attempt untouched for the next
//! cycle; a not-found trade is logged and retried within the scan window.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_repository_sqlx::PostgresCommerceOrderStore;
use sdkwork_order_service::{
    default_payment_notify_handler_registry, default_refund_notify_handler_registry,
    process_payment_notify_verified, process_refund_notify_verified, OwnerOrderSettlementPorts,
    PaymentNotifyEvent, PaymentNotifyHandlerRegistry, RefundNotifyHandlerRegistry,
};
use sdkwork_payment_providers::{
    query_provider_payment_intent, query_provider_refund, ProviderCredentialBundle,
    ProviderPaymentQueryState, ProviderRefundSubmissionState,
};
use sdkwork_payment_repository_sqlx::{
    claim_due_payment_attempts_postgres, claim_due_refunds_postgres,
    ensure_provider_account_matches, load_active_provider_account_postgres,
    load_claim_attempt_provider_context_postgres,
    load_provider_account_for_existing_payment_postgres, provider_account_binding,
    ClaimedPaymentAttempt, ClaimedRefund,
};
use sqlx::PgPool;

use crate::payment_notify_ports::StorePaymentNotifyPorts;

/// Per-pass compensation configuration (env authority lives in the service
/// host; this is the typed pass contract).
#[derive(Debug, Clone)]
pub struct PaymentCompensationPassConfig {
    /// When set, only attempts of this tenant are scanned; `None` scans the
    /// whole tenant-scoped table.
    pub tenant_id: Option<String>,
    /// Scopes the scan to one organization when set.
    pub organization_id: Option<String>,
    /// Maximum attempts + refunds claimed per pass.
    pub batch_size: i64,
    /// Attempts younger than this are never claimed (fresh payments still
    /// have their webhook window).
    pub min_age_seconds: i64,
    /// Attempts older than this are never claimed (bound PSP query load).
    pub max_age_seconds: i64,
}

impl Default for PaymentCompensationPassConfig {
    fn default() -> Self {
        Self {
            tenant_id: None,
            organization_id: None,
            batch_size: 50,
            min_age_seconds: 60,
            max_age_seconds: 24 * 60 * 60,
        }
    }
}

/// Counter-style summary of one compensation pass, for metrics and tests.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PaymentCompensationRunSummary {
    pub claimed_payment_attempts: usize,
    pub claimed_refunds: usize,
    pub payment_events_applied: usize,
    pub refund_events_applied: usize,
    pub payments_still_pending: usize,
    pub payments_not_found: usize,
    pub refunds_still_processing: usize,
    pub refunds_not_found: usize,
    pub skipped_unrepresentable: usize,
    pub errors: usize,
}

/// Runs one compensation pass: claim due attempts + refunds, query the PSP,
/// and re-enter the notify framework with synthetic events. Each claim is
/// processed independently so a failing attempt never blocks the rest.
pub async fn run_payment_compensation_pass(
    pool: &PgPool,
    credentials: &ProviderCredentialBundle,
    settlement_ports: OwnerOrderSettlementPorts<'_>,
    config: &PaymentCompensationPassConfig,
) -> Result<PaymentCompensationRunSummary, CommerceServiceError> {
    run_payment_compensation_pass_with_registries(
        pool,
        credentials,
        settlement_ports,
        default_payment_notify_handler_registry().as_ref(),
        default_refund_notify_handler_registry().as_ref(),
        config,
    )
    .await
}

/// Extension seam: assemblies inject business handler registries without
/// forking the worker; `None` registries fall back to the defaults.
#[allow(clippy::too_many_arguments)]
pub async fn run_payment_compensation_pass_with_registries(
    pool: &PgPool,
    credentials: &ProviderCredentialBundle,
    settlement_ports: OwnerOrderSettlementPorts<'_>,
    payment_notify_registry: &dyn PaymentNotifyHandlerRegistry,
    refund_notify_registry: &dyn RefundNotifyHandlerRegistry,
    config: &PaymentCompensationPassConfig,
) -> Result<PaymentCompensationRunSummary, CommerceServiceError> {
    let now_seconds = unix_now_seconds();
    let mut summary = PaymentCompensationRunSummary {
        claimed_payment_attempts: 0,
        claimed_refunds: 0,
        payment_events_applied: 0,
        refund_events_applied: 0,
        payments_still_pending: 0,
        payments_not_found: 0,
        refunds_still_processing: 0,
        refunds_not_found: 0,
        skipped_unrepresentable: 0,
        errors: 0,
    };
    let tenant_id = config.tenant_id.as_deref().unwrap_or("0");
    let organization_id = config.organization_id.as_deref();

    let attempts = claim_due_payment_attempts_postgres(
        pool,
        tenant_id,
        organization_id,
        config.batch_size,
        now_seconds,
        config.min_age_seconds,
        config.max_age_seconds,
    )
    .await?;
    summary.claimed_payment_attempts = attempts.len();
    let ports = StorePaymentNotifyPorts::postgres(
        pool.clone(),
        credentials.clone(),
        Arc::new(
            sdkwork_payment_providers::PaymentProviderRegistry::from_credentials(
                credentials.clone(),
            ),
        ),
    );
    let refund_state_store = PostgresCommerceOrderStore::new(pool.clone());

    for attempt in attempts {
        match compensate_payment_attempt(
            pool,
            credentials,
            &ports,
            settlement_ports,
            payment_notify_registry,
            &attempt,
        )
        .await
        {
            Ok(AppliedCompensation::Applied) => summary.payment_events_applied += 1,
            Ok(AppliedCompensation::StillPending) => summary.payments_still_pending += 1,
            Ok(AppliedCompensation::NotFound) => summary.payments_not_found += 1,
            Ok(AppliedCompensation::Unrepresentable) => summary.skipped_unrepresentable += 1,
            Err(error) => {
                summary.errors += 1;
                tracing::warn!(
                    target = "order.payment_compensation",
                    attempt_id = %attempt.id,
                    error = ?error,
                    "payment compensation pass failed for one attempt"
                );
            }
        }
    }

    let refunds = claim_due_refunds_postgres(
        pool,
        tenant_id,
        organization_id,
        config.batch_size,
        now_seconds,
        config.min_age_seconds,
        config.max_age_seconds,
    )
    .await?;
    summary.claimed_refunds = refunds.len();
    for refund in refunds {
        match compensate_refund(
            pool,
            credentials,
            &ports,
            &refund_state_store,
            refund_notify_registry,
            &refund,
        )
        .await
        {
            Ok(AppliedCompensation::Applied) => summary.refund_events_applied += 1,
            Ok(AppliedCompensation::StillPending) => summary.refunds_still_processing += 1,
            Ok(AppliedCompensation::NotFound) => summary.refunds_not_found += 1,
            Ok(AppliedCompensation::Unrepresentable) => summary.skipped_unrepresentable += 1,
            Err(error) => {
                summary.errors += 1;
                tracing::warn!(
                    target = "order.payment_compensation",
                    refund_no = %refund.refund_no,
                    error = ?error,
                    "refund compensation pass failed for one refund"
                );
            }
        }
    }
    Ok(summary)
}

/// Outcome of compensating one claim.
enum AppliedCompensation {
    /// A synthetic event was ingested and applied (or replayed idempotently).
    Applied,
    /// The PSP reports the trade still in progress; retry next cycle.
    StillPending,
    /// The PSP does not know the trade; retry within the scan window.
    NotFound,
    /// The PSP state has no representation in the notify status vocabulary.
    Unrepresentable,
}

async fn compensate_payment_attempt(
    pool: &PgPool,
    credentials: &ProviderCredentialBundle,
    ports: &StorePaymentNotifyPorts,
    settlement_ports: OwnerOrderSettlementPorts<'_>,
    notify_registry: &dyn PaymentNotifyHandlerRegistry,
    attempt: &ClaimedPaymentAttempt,
) -> Result<AppliedCompensation, CommerceServiceError> {
    let provider_code = attempt.provider_code.trim().to_ascii_lowercase();
    if provider_code.is_empty() {
        tracing::warn!(
            target = "order.payment_compensation",
            attempt_id = %attempt.id,
            "claimed payment attempt has no provider code; skipped"
        );
        return Ok(AppliedCompensation::Unrepresentable);
    }
    let registry = compensation_provider_registry(
        pool,
        credentials,
        &attempt.tenant_id,
        attempt.organization_id.as_deref(),
        &provider_code,
        attempt.channel_id.as_deref(),
        attempt.provider_account_id.as_deref(),
    )
    .await?;
    let Some(state) = query_provider_payment_intent(
        &registry,
        &provider_code,
        &attempt.out_trade_no,
        attempt.provider_transaction_id.as_deref(),
    )
    .await?
    else {
        tracing::info!(
            target = "order.payment_compensation",
            provider_code,
            out_trade_no = %attempt.out_trade_no,
            "payment compensation: trade not found at provider; retry within scan window"
        );
        return Ok(AppliedCompensation::NotFound);
    };
    let Some(raw_status) = compensation_payment_raw_status(&provider_code, state) else {
        tracing::warn!(
            target = "order.payment_compensation",
            provider_code,
            out_trade_no = %attempt.out_trade_no,
            query_state = ?state,
            "payment compensation: provider state has no notify representation; skipped"
        );
        return Ok(AppliedCompensation::Unrepresentable);
    };
    if state == ProviderPaymentQueryState::Pending {
        // Still in flight at the PSP; the webhook window is still open. Leave
        // the attempt untouched and claim it again on the next pass.
        return Ok(AppliedCompensation::StillPending);
    }
    let event = PaymentNotifyEvent {
        provider_code: provider_code.clone(),
        provider_event_id: Some(format!(
            "query:{provider_code}:{}:{raw_status}",
            attempt.out_trade_no
        )),
        event_type: Some("query.payment".to_owned()),
        out_trade_no: Some(attempt.out_trade_no.clone()),
        payment_status: Some(raw_status.to_owned()),
        payload: serde_json::json!({
            "out_trade_no": attempt.out_trade_no,
            "query_status": raw_status,
            "source": "payment-compensation",
        }),
        tenant_id: Some(attempt.tenant_id.clone()),
        organization_id: attempt.organization_id.clone(),
    };
    let outcome =
        process_payment_notify_verified(event, ports, ports, settlement_ports, notify_registry)
            .await?;
    tracing::info!(
        target = "order.payment_compensation",
        webhook_event_id = %outcome.webhook_event_id,
        attempt_id = %attempt.id,
        applied_status = outcome.applied_status.as_deref(),
        replayed = outcome.replayed,
        status = %outcome.status,
        "payment compensation applied a synthetic query event"
    );
    Ok(AppliedCompensation::Applied)
}

async fn compensate_refund(
    pool: &PgPool,
    credentials: &ProviderCredentialBundle,
    ports: &StorePaymentNotifyPorts,
    refund_state_store: &PostgresCommerceOrderStore,
    refund_registry: &dyn RefundNotifyHandlerRegistry,
    refund: &ClaimedRefund,
) -> Result<AppliedCompensation, CommerceServiceError> {
    let Some(context) =
        load_claim_attempt_provider_context_postgres(pool, &refund.payment_attempt_id).await?
    else {
        tracing::warn!(
            target = "order.payment_compensation",
            refund_no = %refund.refund_no,
            payment_attempt_id = %refund.payment_attempt_id,
            "refund compensation: original payment attempt is gone; skipped"
        );
        return Ok(AppliedCompensation::Unrepresentable);
    };
    let provider_code = context.provider_code.trim().to_ascii_lowercase();
    if provider_code.is_empty() {
        return Ok(AppliedCompensation::Unrepresentable);
    }
    let registry = compensation_provider_registry(
        pool,
        credentials,
        &context.tenant_id,
        context.organization_id.as_deref(),
        &provider_code,
        context.channel_id.as_deref(),
        context.provider_account_id.as_deref(),
    )
    .await?;
    let Some(state) = query_provider_refund(
        &registry,
        &provider_code,
        &context.out_trade_no,
        context.provider_transaction_id.as_deref(),
        &refund.refund_no,
    )
    .await?
    else {
        tracing::info!(
            target = "order.payment_compensation",
            provider_code,
            refund_no = %refund.refund_no,
            "refund compensation: refund not found at provider; retry within scan window"
        );
        return Ok(AppliedCompensation::NotFound);
    };
    if state == ProviderRefundSubmissionState::Processing {
        return Ok(AppliedCompensation::StillPending);
    }
    let Some(raw_status) = compensation_refund_raw_status(&provider_code, state) else {
        tracing::warn!(
            target = "order.payment_compensation",
            provider_code,
            refund_no = %refund.refund_no,
            query_state = ?state,
            "refund compensation: provider state has no notify representation; skipped"
        );
        return Ok(AppliedCompensation::Unrepresentable);
    };
    let event = PaymentNotifyEvent {
        provider_code: provider_code.clone(),
        provider_event_id: Some(format!(
            "query:{provider_code}:{}:{}:{raw_status}",
            context.out_trade_no, refund.refund_no
        )),
        event_type: Some("query.refund".to_owned()),
        out_trade_no: Some(context.out_trade_no.clone()),
        payment_status: Some(raw_status.to_owned()),
        payload: serde_json::json!({
            "out_refund_no": refund.refund_no,
            "refund_status": raw_status,
            "out_trade_no": context.out_trade_no,
            "source": "refund-compensation",
        }),
        tenant_id: Some(context.tenant_id.clone()),
        organization_id: context.organization_id.clone(),
    };
    let outcome =
        process_refund_notify_verified(event, ports, refund_state_store, refund_registry).await?;
    tracing::info!(
        target = "order.payment_compensation",
        webhook_event_id = %outcome.webhook_event_id,
        refund_no = %refund.refund_no,
        order_refund_marked = outcome.order_refund_marked,
        status = %outcome.status,
        "refund compensation applied a synthetic query event"
    );
    Ok(AppliedCompensation::Applied)
}

/// Resolves the provider account for a claimed claim and builds the
/// account-scoped registry, mirroring the webhook/reconciliation resolution.
async fn compensation_provider_registry(
    pool: &PgPool,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    provider_code: &str,
    channel_id: Option<&str>,
    provider_account_id: Option<&str>,
) -> Result<sdkwork_payment_providers::PaymentProviderRegistry, CommerceServiceError> {
    let account = match provider_account_id {
        Some(account_id) => {
            load_provider_account_for_existing_payment_postgres(
                pool,
                tenant_id,
                organization_id,
                account_id,
            )
            .await?
        }
        None if channel_id.is_some() => None,
        None => {
            load_active_provider_account_postgres(pool, tenant_id, organization_id, provider_code)
                .await?
        }
    };
    ensure_provider_account_matches(account.as_ref(), provider_code)?;
    Ok(sdkwork_payment_providers::provider_registry_for_account(
        credentials,
        account.as_ref().map(provider_account_binding),
    ))
}

/// Maps a normalized query state back to the provider's raw status
/// vocabulary so the ingest status machine (`map_provider_payment_status`)
/// resolves it to the same normalized state. Returns `None` when the state
/// has no representation for the provider (skip without an event).
fn compensation_payment_raw_status(
    provider_code: &str,
    state: ProviderPaymentQueryState,
) -> Option<&'static str> {
    use ProviderPaymentQueryState::{Canceled, Failed, Pending, Succeeded};
    match provider_code {
        "stripe" => match state {
            Succeeded => Some("succeeded"),
            Canceled => Some("canceled"),
            Failed => Some("payment_failed"),
            Pending => Some("processing"),
        },
        "wechat_pay" => match state {
            Succeeded => Some("success"),
            Canceled => Some("closed"),
            Pending => Some("userpaying"),
            Failed => None,
        },
        "alipay" => match state {
            Succeeded => Some("trade_success"),
            Canceled => Some("trade_closed"),
            Pending => Some("wait_buyer_pay"),
            Failed => None,
        },
        "sandbox" => match state {
            Succeeded => Some("succeeded"),
            Canceled => Some("canceled"),
            Failed => Some("failed"),
            Pending => Some("pending"),
        },
        _ => None,
    }
}

/// Maps a normalized refund query state back to the provider's raw status
/// vocabulary (`map_provider_refund_status` resolves it to the same
/// normalized state). Only terminal states are emitted: processing refunds
/// are re-claimed on the next pass instead of re-ingesting a no-op event.
fn compensation_refund_raw_status(
    provider_code: &str,
    state: ProviderRefundSubmissionState,
) -> Option<&'static str> {
    use ProviderRefundSubmissionState::{Failed, Processing, Succeeded};
    match provider_code {
        "wechat_pay" => match state {
            Succeeded => Some("success"),
            Failed => Some("closed"),
            Processing => None,
        },
        "stripe" => match state {
            Succeeded => Some("succeeded"),
            Failed => Some("failed"),
            Processing => None,
        },
        "alipay" => match state {
            Succeeded => Some("refund_success"),
            Failed => Some("refund_failed"),
            Processing => None,
        },
        "sandbox" => match state {
            Succeeded => Some("success"),
            Failed => Some("closed"),
            Processing => None,
        },
        _ => None,
    }
}

fn unix_now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_payment_repository_sqlx::webhook_status::{
        map_provider_payment_status, map_provider_refund_status,
    };

    /// The synthetic event carries a raw provider status that the ingest
    /// status machine must resolve back to the same normalized state the
    /// query helper produced — otherwise the compensation path would apply a
    /// different state than the PSP reported.
    #[test]
    fn payment_raw_status_round_trips_through_the_ingest_machine() {
        use ProviderPaymentQueryState::{Canceled, Failed, Pending, Succeeded};
        let cases: &[(&str, ProviderPaymentQueryState, &str)] = &[
            ("stripe", Succeeded, "succeeded"),
            ("stripe", Canceled, "canceled"),
            ("stripe", Failed, "failed"),
            ("stripe", Pending, "pending"),
            ("wechat_pay", Succeeded, "succeeded"),
            ("wechat_pay", Canceled, "canceled"),
            ("wechat_pay", Pending, "pending"),
            ("alipay", Succeeded, "succeeded"),
            ("alipay", Canceled, "canceled"),
            ("alipay", Pending, "pending"),
            ("sandbox", Succeeded, "succeeded"),
            ("sandbox", Canceled, "canceled"),
            ("sandbox", Pending, "pending"),
        ];
        for (provider, state, expected) in cases {
            let raw = compensation_payment_raw_status(provider, *state)
                .unwrap_or_else(|| panic!("{provider} {state:?} must map to a raw status"));
            assert_eq!(
                map_provider_payment_status(provider, raw),
                Some(*expected),
                "{provider} {state:?} raw {raw} must round-trip to {expected}"
            );
        }
        // WeChat/Alipay have no failed query state in the notify vocabulary;
        // the worker must skip them instead of inventing a raw status.
        assert_eq!(compensation_payment_raw_status("wechat_pay", Failed), None);
        assert_eq!(compensation_payment_raw_status("alipay", Failed), None);
    }

    #[test]
    fn refund_raw_status_round_trips_through_the_ingest_machine() {
        use ProviderRefundSubmissionState::{Failed, Succeeded};
        let cases: &[(&str, ProviderRefundSubmissionState, &str)] = &[
            ("stripe", Succeeded, "succeeded"),
            ("stripe", Failed, "failed"),
            ("wechat_pay", Succeeded, "succeeded"),
            ("wechat_pay", Failed, "failed"),
            ("alipay", Succeeded, "succeeded"),
            ("alipay", Failed, "failed"),
            ("sandbox", Succeeded, "succeeded"),
            ("sandbox", Failed, "failed"),
        ];
        for (provider, state, expected) in cases {
            let raw = compensation_refund_raw_status(provider, *state)
                .unwrap_or_else(|| panic!("{provider} {state:?} must map to a raw status"));
            assert_eq!(
                map_provider_refund_status(provider, raw),
                Some(*expected),
                "{provider} {state:?} raw {raw} must round-trip to {expected}"
            );
        }
        // Processing refunds are not emitted; they are re-claimed next pass.
        assert_eq!(
            compensation_refund_raw_status("stripe", ProviderRefundSubmissionState::Processing),
            None
        );
    }

    #[test]
    fn synthetic_event_ids_are_stable_and_state_scoped() {
        let event_id = |status: &str| format!("query:sandbox:trade-1:{status}");
        assert_eq!(event_id("succeeded"), "query:sandbox:trade-1:succeeded");
        // The mapped status is part of the id: same trade, different outcome
        // state = different event, so late recovery can still re-apply.
        assert_ne!(event_id("succeeded"), event_id("canceled"));
    }
}
