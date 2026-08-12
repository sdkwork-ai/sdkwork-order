//! Order to Payment executor integration adapters.

mod payment_compensation;
mod payment_notify_ports;
mod payment_reconciliation;

pub use payment_compensation::{
    run_payment_compensation_pass, run_payment_compensation_pass_with_registries,
    PaymentCompensationPassConfig, PaymentCompensationRunSummary,
};
pub use payment_notify_ports::StorePaymentNotifyPorts;
pub use payment_reconciliation::{
    owner_order_payment_reconciliation_port_from_database_pool,
    StoreOwnerOrderPaymentReconciliationAdapter,
};

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service::{
    AccountValueFuture, PaymentExecutorOutcome, PaymentRefundExecutionRequest,
    PaymentRefundExecutorPort,
};
use sdkwork_payment_providers::{
    create_provider_refund, normalize_provider_code, provider_registry_for_account,
    query_provider_refund, PaymentProviderRegistry, ProviderCredentialBundle,
    ProviderRefundSubmissionState,
};
use sdkwork_payment_repository_sqlx::{
    ensure_provider_account_matches, load_active_provider_account_postgres,
    load_payment_attempt_provider_context_by_id_postgres,
    load_provider_account_for_existing_payment_postgres, provider_account_binding,
    PostgresCommerceRefundStore,
};
use sdkwork_payment_service::{CreateOwnerRefundCommand, RefundView};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct StorePaymentRefundExecutorAdapter {
    store: StorePaymentRefundExecutorStore,
}

#[derive(Clone)]
enum StorePaymentRefundExecutorStore {
    Postgres {
        pool: PgPool,
        refunds: Arc<PostgresCommerceRefundStore>,
        credentials: ProviderCredentialBundle,
    },
}

impl StorePaymentRefundExecutorAdapter {
    pub fn postgres(pool: PgPool) -> Self {
        Self::postgres_with_credentials(pool, ProviderCredentialBundle::from_env())
    }

    pub fn postgres_with_credentials(pool: PgPool, credentials: ProviderCredentialBundle) -> Self {
        Self {
            store: StorePaymentRefundExecutorStore::Postgres {
                refunds: Arc::new(PostgresCommerceRefundStore::new(pool.clone())),
                pool,
                credentials,
            },
        }
    }

    pub fn from_database_pool(pool: &DatabasePool) -> Self {
        // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
        let DatabasePool::Postgres(pool, _) = pool else {
            panic!("payment refund executor adapter requires a PostgreSQL database pool");
        };
        Self::postgres(pool.clone())
    }
}

impl PaymentRefundExecutorPort for StorePaymentRefundExecutorAdapter {
    fn execute_provider_refund<'a>(
        &'a self,
        request: PaymentRefundExecutionRequest,
    ) -> AccountValueFuture<'a, PaymentExecutorOutcome> {
        Box::pin(async move {
            match &self.store {
                StorePaymentRefundExecutorStore::Postgres {
                    pool,
                    refunds,
                    credentials,
                } => execute_postgres_refund(pool, refunds.as_ref(), credentials, request).await,
            }
        })
    }
}
async fn execute_postgres_refund(
    pool: &PgPool,
    refunds: &PostgresCommerceRefundStore,
    credentials: &ProviderCredentialBundle,
    request: PaymentRefundExecutionRequest,
) -> Result<PaymentExecutorOutcome, CommerceServiceError> {
    let command = payment_refund_command(&request)?;
    let mut refund = refunds.create_owner_refund(command).await?;
    if !refund_requires_provider_submission(&refund.status) {
        return Ok(refund_outcome(refund));
    }
    if refund_requires_processing_claim(&refund.status) {
        refund = refunds
            .mark_owner_refund_provider_submission_processing(
                &request.tenant_id,
                request.organization_id.as_deref(),
                &refund.refund_id,
                "buyer",
                Some(&request.owner_user_id),
                &request.request_no,
                &request.idempotency_key,
            )
            .await?;
    }
    match submit_postgres_provider_refund(
        pool,
        credentials,
        &request.tenant_id,
        request.organization_id.as_deref(),
        &refund,
        Some(request.refund_request_id.clone()),
    )
    .await
    {
        Ok(ProviderRefundSubmissionState::Processing) => {}
        Ok(ProviderRefundSubmissionState::Succeeded) => {
            refund = refunds
                .mark_owner_refund_provider_submission_succeeded(
                    &request.tenant_id,
                    request.organization_id.as_deref(),
                    &refund.refund_id,
                    "buyer",
                    Some(&request.owner_user_id),
                    &request.request_no,
                    &request.idempotency_key,
                )
                .await?;
        }
        Ok(ProviderRefundSubmissionState::Failed) => {
            refund = refunds
                .mark_owner_refund_provider_submission_failed(
                    &request.tenant_id,
                    request.organization_id.as_deref(),
                    &refund.refund_id,
                    "buyer",
                    Some(&request.owner_user_id),
                    &request.request_no,
                    &request.idempotency_key,
                )
                .await?;
        }
        Err(error) => {
            if refund_submission_retryable(&error) {
                return Ok(refund_outcome(refund));
            }
            let _ = refunds
                .mark_owner_refund_provider_submission_failed(
                    &request.tenant_id,
                    request.organization_id.as_deref(),
                    &refund.refund_id,
                    "buyer",
                    Some(&request.owner_user_id),
                    &request.request_no,
                    &request.idempotency_key,
                )
                .await;
            return Err(error);
        }
    }
    Ok(refund_outcome(refund))
}

fn refund_requires_provider_submission(status: &str) -> bool {
    matches!(status, "submitted" | "processing" | "failed")
}

fn refund_requires_processing_claim(status: &str) -> bool {
    matches!(status, "submitted" | "failed")
}

fn refund_submission_retryable(error: &CommerceServiceError) -> bool {
    !matches!(
        error.code(),
        "not-found" | "validation" | "validation-failed" | "forbidden" | "conflict"
    )
}

fn payment_refund_command(
    request: &PaymentRefundExecutionRequest,
) -> Result<CreateOwnerRefundCommand, CommerceServiceError> {
    CreateOwnerRefundCommand::new_with_currency(
        &request.tenant_id,
        request.organization_id.as_deref(),
        &request.owner_user_id,
        &request.original_order_id,
        None,
        Some(request.amount.as_str()),
        Some(&request.currency_code),
        Some("order_refund_request"),
        &request.request_no,
        &request.idempotency_key,
    )
}
async fn submit_postgres_provider_refund(
    pool: &PgPool,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    refund: &RefundView,
    reason: Option<String>,
) -> Result<ProviderRefundSubmissionState, CommerceServiceError> {
    let Some(context) =
        load_payment_attempt_provider_context_by_id_postgres(pool, &refund.payment_attempt_id)
            .await?
    else {
        return Err(CommerceServiceError::not_found(
            "payment attempt provider context was not found for refund execution",
        ));
    };
    let provider_code = normalize_provider_code(&context.provider_code);
    let registry = if provider_code == "sandbox" || provider_code.is_empty() {
        PaymentProviderRegistry::from_credentials(credentials.clone())
    } else {
        let account = match context.provider_account_id.as_deref() {
            Some(provider_account_id) => load_provider_account_for_existing_payment_postgres(
                pool,
                tenant_id,
                organization_id,
                provider_account_id,
            )
            .await?
            .ok_or_else(|| {
                CommerceServiceError::conflict(
                    "original payment provider account is unavailable for refund",
                )
            })?
            .into(),
            None if context.channel_id.is_some() => None,
            None => {
                load_active_provider_account_postgres(
                    pool,
                    tenant_id,
                    organization_id,
                    &provider_code,
                )
                .await?
            }
        };
        ensure_provider_account_matches(account.as_ref(), &provider_code)?;
        provider_registry_for_account(credentials, account.as_ref().map(provider_account_binding))
    };
    submit_provider_refund(
        &registry,
        &provider_code,
        &context.out_trade_no,
        context.provider_transaction_id.as_deref(),
        refund,
        &context.amount,
        reason,
    )
    .await
}

async fn submit_provider_refund(
    registry: &PaymentProviderRegistry,
    provider_code: &str,
    out_trade_no: &str,
    provider_transaction_id: Option<&str>,
    refund: &RefundView,
    total_amount: &str,
    reason: Option<String>,
) -> Result<ProviderRefundSubmissionState, CommerceServiceError> {
    let total_amount = CommerceMoney::new(total_amount).map_err(CommerceServiceError::storage)?;
    if refund.status == "processing" {
        if let Some(state) = query_provider_refund(
            registry,
            provider_code,
            out_trade_no,
            provider_transaction_id,
            &refund.refund_no,
        )
        .await?
        {
            return Ok(state);
        }
    }
    create_provider_refund(
        registry,
        provider_code,
        out_trade_no,
        provider_transaction_id,
        &refund.refund_no,
        &refund.amount,
        &total_amount,
        reason,
    )
    .await
}

fn refund_outcome(refund: RefundView) -> PaymentExecutorOutcome {
    PaymentExecutorOutcome {
        accepted: true,
        replayed: false,
        provider_reference_id: Some(refund.refund_id),
        status: refund.status,
    }
}

pub fn payment_refund_executor_port_from_database_pool(
    pool: &DatabasePool,
) -> Arc<dyn PaymentRefundExecutorPort> {
    Arc::new(StorePaymentRefundExecutorAdapter::from_database_pool(pool))
}

#[cfg(test)]
mod tests {
    use super::{
        refund_requires_processing_claim, refund_requires_provider_submission,
        refund_submission_retryable,
    };
    use sdkwork_contract_service::CommerceServiceError;

    #[test]
    fn ambiguous_refund_submission_reuses_the_processing_refund() {
        assert!(refund_requires_provider_submission("processing"));
        assert!(!refund_requires_processing_claim("processing"));
        assert!(refund_requires_provider_submission("failed"));
        assert!(refund_requires_processing_claim("failed"));
        assert!(!refund_requires_provider_submission("succeeded"));
    }

    #[test]
    fn deterministic_validation_failure_is_not_ambiguous() {
        assert!(!refund_submission_retryable(
            &CommerceServiceError::validation("invalid provider refund input")
        ));
        assert!(refund_submission_retryable(&CommerceServiceError::storage(
            "provider response was not observed"
        )));
    }
}
