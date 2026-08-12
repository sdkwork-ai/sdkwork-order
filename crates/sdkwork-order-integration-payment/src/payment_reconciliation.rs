use std::collections::BTreeMap;
use std::sync::Arc;

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service::{
    OrderPaymentSettlementAttempt, OwnerOrderPaymentConfirmationFuture,
    OwnerOrderPaymentReconciliationPort, ReconcileOwnerOrderPaymentOutcome,
    ReconcileOwnerOrderPaymentRequest,
};
use sdkwork_payment_providers::{
    normalize_provider_code, provider_registry_for_account, PaymentProviderRegistry,
    PaymentQueryPaymentIntentRequest, ProviderCredentialBundle,
};
use sdkwork_payment_repository_sqlx::{
    ensure_provider_account_matches, load_active_provider_account_postgres,
    load_payment_attempt_provider_context_by_id_postgres,
    load_provider_account_for_existing_payment_postgres, persist_attempt_enrichment_postgres,
    provider_account_binding, PaymentAttemptProviderContext,
};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct StoreOwnerOrderPaymentReconciliationAdapter {
    store: ReconciliationStore,
}

#[derive(Clone)]
enum ReconciliationStore {
    Postgres {
        pool: PgPool,
        credentials: ProviderCredentialBundle,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReconciliationCandidate {
    attempt_id: String,
    provider_code: String,
    out_trade_no: String,
    amount: String,
    currency_code: String,
    status: String,
}

impl StoreOwnerOrderPaymentReconciliationAdapter {
    pub fn postgres(pool: PgPool) -> Self {
        Self::postgres_with_credentials(pool, ProviderCredentialBundle::from_env())
    }

    pub fn postgres_with_credentials(pool: PgPool, credentials: ProviderCredentialBundle) -> Self {
        Self {
            store: ReconciliationStore::Postgres { pool, credentials },
        }
    }

    pub fn from_database_pool(pool: &DatabasePool) -> Self {
        // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
        let DatabasePool::Postgres(pool, _) = pool else {
            panic!("payment reconciliation adapter requires a PostgreSQL database pool");
        };
        Self::postgres(pool.clone())
    }
}

impl OwnerOrderPaymentReconciliationPort for StoreOwnerOrderPaymentReconciliationAdapter {
    fn reconcile_owner_order_payment<'a>(
        &'a self,
        request: ReconcileOwnerOrderPaymentRequest,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, ReconcileOwnerOrderPaymentOutcome> {
        Box::pin(async move {
            match &self.store {
                ReconciliationStore::Postgres { pool, credentials } => {
                    reconcile_postgres(pool, credentials, request).await
                }
            }
        })
    }
}
async fn reconcile_postgres(
    pool: &PgPool,
    credentials: &ProviderCredentialBundle,
    request: ReconcileOwnerOrderPaymentRequest,
) -> Result<ReconcileOwnerOrderPaymentOutcome, CommerceServiceError> {
    let candidate = load_candidate_postgres(pool, &request).await?;
    if candidate.status.eq_ignore_ascii_case("succeeded") {
        return Ok(reconciliation_outcome(
            request,
            candidate,
            "succeeded",
            true,
        ));
    }

    let context = load_payment_attempt_provider_context_by_id_postgres(pool, &candidate.attempt_id)
        .await?
        .ok_or_else(|| {
            CommerceServiceError::conflict("payment attempt disappeared during reconciliation")
        })?;
    ensure_candidate_context(&candidate, &context, &request)?;
    let registry = reconciliation_registry_postgres(pool, credentials, &request, &context).await?;
    let provider = query_and_validate_provider(&registry, &candidate, &context).await?;
    persist_provider_query_postgres(pool, &request.tenant_id, &candidate, &provider).await?;
    Ok(reconciliation_outcome(
        request,
        candidate,
        &provider.status,
        false,
    ))
}

fn reconciliation_outcome(
    request: ReconcileOwnerOrderPaymentRequest,
    candidate: ReconciliationCandidate,
    provider_status: &str,
    replayed: bool,
) -> ReconcileOwnerOrderPaymentOutcome {
    ReconcileOwnerOrderPaymentOutcome {
        attempt: OrderPaymentSettlementAttempt {
            tenant_id: request.tenant_id,
            organization_id: request.organization_id,
            owner_user_id: request.owner_user_id,
            order_id: request.order_id,
            payment_attempt_id: Some(candidate.attempt_id),
            out_trade_no: Some(candidate.out_trade_no),
        },
        provider_code: candidate.provider_code,
        provider_status: provider_status.to_owned(),
        replayed,
    }
}
async fn load_candidate_postgres(
    pool: &PgPool,
    request: &ReconcileOwnerOrderPaymentRequest,
) -> Result<ReconciliationCandidate, CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT id, provider_code, out_trade_no, CAST(amount AS BIGINT)::TEXT AS amount,
               currency_code, status
        FROM commerce_payment_attempt
        WHERE tenant_id = CAST($1 AS TEXT)
          AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
          AND owner_user_id = CAST($3 AS TEXT)
          AND order_id = CAST($4 AS TEXT)
          AND deleted_at IS NULL
        ORDER BY id
        LIMIT 2
        "#,
    )
    .bind(&request.tenant_id)
    .bind(request.organization_id.as_deref())
    .bind(&request.owner_user_id)
    .bind(&request.order_id)
    .fetch_all(pool)
    .await
    .map_err(|error| store_error("failed to load payment reconciliation candidate", error))?;
    unique_candidate(rows.iter().map(map_candidate).collect())
}

fn map_candidate<R: Row>(row: &R) -> ReconciliationCandidate
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    String: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    ReconciliationCandidate {
        attempt_id: row.try_get("id").unwrap_or_default(),
        provider_code: normalize_provider_code(
            &row.try_get::<String, _>("provider_code")
                .unwrap_or_default(),
        ),
        out_trade_no: row.try_get("out_trade_no").unwrap_or_default(),
        amount: row.try_get("amount").unwrap_or_default(),
        currency_code: row.try_get("currency_code").unwrap_or_default(),
        status: row.try_get("status").unwrap_or_default(),
    }
}

fn unique_candidate(
    candidates: Vec<ReconciliationCandidate>,
) -> Result<ReconciliationCandidate, CommerceServiceError> {
    match candidates.as_slice() {
        [] => Err(CommerceServiceError::not_found(
            "owner order payment attempt was not found",
        )),
        [_first, _second, ..] => Err(CommerceServiceError::conflict(
            "multiple payment attempts match payment reconciliation",
        )),
        [candidate] => Ok(candidate.clone()),
    }
}

fn ensure_candidate_context(
    candidate: &ReconciliationCandidate,
    context: &PaymentAttemptProviderContext,
    request: &ReconcileOwnerOrderPaymentRequest,
) -> Result<(), CommerceServiceError> {
    if context.attempt_id != candidate.attempt_id
        || context.tenant_id != request.tenant_id
        || normalize_provider_code(&context.provider_code) != candidate.provider_code
        || context.out_trade_no != candidate.out_trade_no
        || context.amount != candidate.amount
    {
        return Err(CommerceServiceError::conflict(
            "payment attempt identity changed during provider reconciliation",
        ));
    }
    Ok(())
}
async fn reconciliation_registry_postgres(
    pool: &PgPool,
    credentials: &ProviderCredentialBundle,
    request: &ReconcileOwnerOrderPaymentRequest,
    context: &PaymentAttemptProviderContext,
) -> Result<PaymentProviderRegistry, CommerceServiceError> {
    let account = match context.provider_account_id.as_deref() {
        Some(account_id) => load_provider_account_for_existing_payment_postgres(
            pool,
            &request.tenant_id,
            request.organization_id.as_deref(),
            account_id,
        )
        .await?
        .ok_or_else(|| CommerceServiceError::conflict("payment provider account is unavailable"))?
        .into(),
        None if context.channel_id.is_some() => None,
        None => {
            load_active_provider_account_postgres(
                pool,
                &request.tenant_id,
                request.organization_id.as_deref(),
                &context.provider_code,
            )
            .await?
        }
    };
    ensure_provider_account_matches(account.as_ref(), &context.provider_code)?;
    Ok(provider_registry_for_account(
        credentials,
        account.as_ref().map(provider_account_binding),
    ))
}

struct VerifiedProviderPayment {
    native_id: Option<String>,
    status: String,
}

async fn query_and_validate_provider(
    registry: &PaymentProviderRegistry,
    candidate: &ReconciliationCandidate,
    context: &PaymentAttemptProviderContext,
) -> Result<VerifiedProviderPayment, CommerceServiceError> {
    if candidate.provider_code == "sandbox" || candidate.provider_code.is_empty() {
        return Ok(VerifiedProviderPayment {
            native_id: None,
            status: "succeeded".to_owned(),
        });
    }

    let adapter = registry.resolve(&candidate.provider_code).ok_or_else(|| {
        CommerceServiceError::provider_unavailable(format!(
            "payment provider {} is not configured",
            candidate.provider_code
        ))
    })?;
    let reference = provider_query_reference(candidate, context)?;
    let outcome = adapter
        .query_payment_intent(PaymentQueryPaymentIntentRequest {
            payment_intent_id: Some(reference.to_owned()),
            metadata: json!({ "out_trade_no": candidate.out_trade_no }),
        })
        .await
        .map_err(|_| {
            CommerceServiceError::provider_unavailable(
                "payment provider query did not produce a conclusive result",
            )
        })?;
    if candidate.provider_code == "stripe" && outcome.native_id.as_deref() != Some(reference) {
        return Err(CommerceServiceError::conflict(
            "Stripe payment query returned a mismatched payment intent id",
        ));
    }
    if normalize_provider_code(&outcome.provider_code) != candidate.provider_code {
        return Err(CommerceServiceError::conflict(
            "payment provider query returned a mismatched provider identity",
        ));
    }
    let raw_status = outcome
        .raw_status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CommerceServiceError::provider_unavailable(
                "payment provider query returned no payment status",
            )
        })?;
    if normalized_provider_status(&candidate.provider_code, raw_status) != Some("succeeded") {
        return Err(CommerceServiceError::conflict(format!(
            "payment provider has not confirmed success (status: {raw_status})"
        )));
    }
    validate_provider_identity_and_amount(
        candidate,
        &outcome.payload,
        outcome.native_id.as_deref(),
    )?;

    Ok(VerifiedProviderPayment {
        native_id: outcome.native_id,
        status: raw_status.to_owned(),
    })
}

fn provider_query_reference<'a>(
    candidate: &'a ReconciliationCandidate,
    context: &'a PaymentAttemptProviderContext,
) -> Result<&'a str, CommerceServiceError> {
    if candidate.provider_code == "stripe" {
        return context
            .provider_transaction_id
            .as_deref()
            .filter(|value| value.starts_with("pi_"))
            .ok_or_else(|| {
                CommerceServiceError::conflict(
                    "Stripe payment reconciliation requires the original payment intent id",
                )
            });
    }
    Ok(candidate.out_trade_no.as_str())
}

fn normalized_provider_status(provider_code: &str, raw_status: &str) -> Option<&'static str> {
    let status = raw_status.trim().to_ascii_lowercase();
    match provider_code {
        "stripe" => match status.as_str() {
            "succeeded" => Some("succeeded"),
            "canceled" | "cancelled" | "payment_failed" => Some("failed"),
            "requires_payment_method"
            | "requires_confirmation"
            | "requires_action"
            | "processing"
            | "requires_capture" => Some("pending"),
            _ => None,
        },
        "alipay" => match status.as_str() {
            "trade_success" | "trade_finished" => Some("succeeded"),
            "trade_closed" => Some("failed"),
            "wait_buyer_pay" => Some("pending"),
            _ => None,
        },
        "wechat_pay" => match status.as_str() {
            "success" => Some("succeeded"),
            "notpay" | "userpaying" => Some("pending"),
            "closed" | "revoked" | "payerror" => Some("failed"),
            _ => None,
        },
        "sandbox" => Some("succeeded"),
        _ => None,
    }
}

fn validate_provider_identity_and_amount(
    candidate: &ReconciliationCandidate,
    payload: &Value,
    native_id: Option<&str>,
) -> Result<(), CommerceServiceError> {
    let expected_minor = candidate.amount.parse::<i64>().map_err(|_| {
        CommerceServiceError::storage("payment attempt amount is not a valid minor-unit amount")
    })?;
    match candidate.provider_code.as_str() {
        "stripe" => {
            let returned_id = payload.get("id").and_then(Value::as_str).or(native_id);
            if returned_id.is_none() {
                return Err(provider_mismatch("Stripe payment intent id is missing"));
            }
            let amount = payload
                .get("amount_received")
                .and_then(Value::as_i64)
                .or_else(|| payload.get("amount").and_then(Value::as_i64));
            ensure_minor_amount(amount, expected_minor)?;
            ensure_currency(
                payload.get("currency").and_then(Value::as_str),
                &candidate.currency_code,
            )?;
        }
        "alipay" => {
            ensure_out_trade_no(payload, &candidate.out_trade_no)?;
            let amount = payload
                .get("total_amount")
                .and_then(Value::as_str)
                .and_then(decimal_major_to_minor);
            ensure_minor_amount(amount, expected_minor)?;
            ensure_currency(Some("CNY"), &candidate.currency_code)?;
        }
        "wechat_pay" => {
            ensure_out_trade_no(payload, &candidate.out_trade_no)?;
            let amount = payload.pointer("/amount/total").and_then(Value::as_i64);
            ensure_minor_amount(amount, expected_minor)?;
            ensure_currency(
                payload.pointer("/amount/currency").and_then(Value::as_str),
                &candidate.currency_code,
            )?;
        }
        _ => {
            return Err(CommerceServiceError::provider_unavailable(format!(
                "payment provider {} is unsupported for reconciliation",
                candidate.provider_code
            )));
        }
    }
    Ok(())
}

fn ensure_out_trade_no(payload: &Value, expected: &str) -> Result<(), CommerceServiceError> {
    match payload.get("out_trade_no").and_then(Value::as_str) {
        Some(actual) if actual == expected => Ok(()),
        _ => Err(provider_mismatch(
            "provider merchant order number does not match",
        )),
    }
}

fn ensure_minor_amount(actual: Option<i64>, expected: i64) -> Result<(), CommerceServiceError> {
    match actual {
        Some(actual) if actual == expected => Ok(()),
        _ => Err(provider_mismatch("provider payment amount does not match")),
    }
}

fn ensure_currency(actual: Option<&str>, expected: &str) -> Result<(), CommerceServiceError> {
    match actual {
        Some(actual) if actual.eq_ignore_ascii_case(expected) => Ok(()),
        _ => Err(provider_mismatch(
            "provider payment currency does not match",
        )),
    }
}

fn provider_mismatch(message: &str) -> CommerceServiceError {
    CommerceServiceError::conflict(message)
}

fn decimal_major_to_minor(value: &str) -> Option<i64> {
    let (major, fraction) = value.trim().split_once('.').unwrap_or((value.trim(), ""));
    if major.is_empty()
        || !major.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 2
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let major = major.parse::<i64>().ok()?;
    let fraction = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().ok()? * 10,
        _ => fraction.parse::<i64>().ok()?,
    };
    major.checked_mul(100)?.checked_add(fraction)
}
async fn persist_provider_query_postgres(
    pool: &PgPool,
    tenant_id: &str,
    candidate: &ReconciliationCandidate,
    provider: &VerifiedProviderPayment,
) -> Result<(), CommerceServiceError> {
    let enrichment = provider_enrichment(provider);
    persist_attempt_enrichment_postgres(pool, tenant_id, &candidate.attempt_id, &enrichment).await
}

fn provider_enrichment(provider: &VerifiedProviderPayment) -> BTreeMap<String, String> {
    let mut enrichment = BTreeMap::new();
    enrichment.insert("providerStatus".to_owned(), provider.status.clone());
    if let Some(native_id) = provider.native_id.as_ref() {
        enrichment.insert("providerTransactionId".to_owned(), native_id.clone());
    }
    enrichment
}

fn store_error(context: &str, error: sqlx::Error) -> CommerceServiceError {
    CommerceServiceError::storage(format!("{context}: {error}"))
}

pub fn owner_order_payment_reconciliation_port_from_database_pool(
    pool: &DatabasePool,
) -> Arc<dyn OwnerOrderPaymentReconciliationPort> {
    Arc::new(StoreOwnerOrderPaymentReconciliationAdapter::from_database_pool(pool))
}

#[cfg(test)]
mod tests {
    use sdkwork_order_service::{
        OwnerOrderPaymentReconciliationPort, ReconcileOwnerOrderPaymentRequest,
    };
    use sdkwork_payment_providers::ProviderCredentialBundle;
    use sdkwork_payment_repository_sqlx::PostgresCommerceOwnerOrderPaymentStore;
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::Row;

    use super::{
        decimal_major_to_minor, normalized_provider_status, validate_provider_identity_and_amount,
        ReconciliationCandidate, StoreOwnerOrderPaymentReconciliationAdapter,
    };

    #[test]
    fn provider_success_statuses_are_explicit_and_fail_closed() {
        assert_eq!(
            normalized_provider_status("stripe", "succeeded"),
            Some("succeeded")
        );
        assert_eq!(
            normalized_provider_status("alipay", "TRADE_SUCCESS"),
            Some("succeeded")
        );
        assert_eq!(
            normalized_provider_status("wechat_pay", "SUCCESS"),
            Some("succeeded")
        );
        assert_eq!(
            normalized_provider_status("wechat_pay", "NOTPAY"),
            Some("pending")
        );
        assert_eq!(normalized_provider_status("new-provider", "SUCCESS"), None);
    }

    #[test]
    fn provider_payload_must_match_trade_amount_and_currency() {
        let candidate = ReconciliationCandidate {
            attempt_id: "attempt-1".to_owned(),
            provider_code: "wechat_pay".to_owned(),
            out_trade_no: "trade-1".to_owned(),
            amount: "9900".to_owned(),
            currency_code: "CNY".to_owned(),
            status: "pending".to_owned(),
        };
        validate_provider_identity_and_amount(
            &candidate,
            &json!({
                "transaction_id": "wx-1",
                "out_trade_no": "trade-1",
                "amount": { "total": 9900, "currency": "CNY" }
            }),
            Some("wx-1"),
        )
        .expect("matching provider payment");

        let mismatch = json!({
            "transaction_id": "wx-1",
            "out_trade_no": "trade-1",
            "amount": { "total": 1, "currency": "CNY" }
        });
        assert!(
            validate_provider_identity_and_amount(&candidate, &mismatch, Some("wx-1")).is_err()
        );
    }

    #[test]
    fn alipay_decimal_amount_is_compared_as_minor_units_without_float_rounding() {
        assert_eq!(decimal_major_to_minor("99.00"), Some(9900));
        assert_eq!(decimal_major_to_minor("0.01"), Some(1));
        assert_eq!(decimal_major_to_minor("1.001"), None);
    }

    #[tokio::test]
    async fn provider_query_then_transactional_confirmation_replays_idempotently() {
        // 服务端测试必须使用 PostgreSQL（DATABASE_SPEC：authoritative-server）
        let Some(url) = std::env::var("SDKWORK_DATABASE_TEST_POSTGRES_URL").ok() else {
            eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
            return;
        };
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .expect("postgres pool");
        for statement in [
            "DROP TABLE IF EXISTS commerce_payment_attempt",
            "DROP TABLE IF EXISTS commerce_payment_intent",
            "DROP TABLE IF EXISTS commerce_payment_channel",
            "DROP TABLE IF EXISTS commerce_order",
            "CREATE TABLE commerce_order (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, owner_user_id TEXT NOT NULL)",
            "CREATE TABLE commerce_payment_channel (id TEXT PRIMARY KEY, provider_account_id TEXT)",
            "CREATE TABLE commerce_payment_intent (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, owner_user_id TEXT NOT NULL, order_id TEXT NOT NULL, status TEXT NOT NULL, updated_at TIMESTAMPTZ, deleted_at TEXT)",
            "CREATE TABLE commerce_payment_attempt (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, owner_user_id TEXT NOT NULL, order_id TEXT NOT NULL, payment_intent_id TEXT NOT NULL, provider_code TEXT NOT NULL, channel_id TEXT, out_trade_no TEXT NOT NULL, amount TEXT NOT NULL, currency_code TEXT NOT NULL, status TEXT NOT NULL, callback_payload TEXT NOT NULL, idempotency_key TEXT NOT NULL, paid_at TIMESTAMPTZ, updated_at TIMESTAMPTZ, deleted_at TEXT)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement))
                .execute(&pool)
                .await
                .expect("create payment reconciliation table");
        }
        sqlx::query("INSERT INTO commerce_order VALUES ('order-1', 'tenant-1', 'org-1', 'user-1')")
            .execute(&pool)
            .await
            .expect("seed order");
        sqlx::query("INSERT INTO commerce_payment_channel VALUES ('channel-1', NULL)")
            .execute(&pool)
            .await
            .expect("seed channel");
        sqlx::query("INSERT INTO commerce_payment_intent VALUES ('intent-1', 'tenant-1', 'org-1', 'user-1', 'order-1', 'pending', '2026-07-30T00:00:00Z', NULL)")
            .execute(&pool)
            .await
            .expect("seed intent");
        sqlx::query("INSERT INTO commerce_payment_attempt VALUES ('attempt-1', 'tenant-1', 'org-1', 'user-1', 'order-1', 'intent-1', 'sandbox', 'channel-1', 'trade-1', '9900', 'CNY', 'pending', '{}', 'idem-1', NULL, '2026-07-30T00:00:00Z', NULL)")
            .execute(&pool)
            .await
            .expect("seed attempt");

        let adapter = StoreOwnerOrderPaymentReconciliationAdapter::postgres_with_credentials(
            pool.clone(),
            ProviderCredentialBundle {
                stripe: None,
                alipay: None,
                wechat_pay: None,
                webhook_base_url: None,
            },
        );
        let request = ReconcileOwnerOrderPaymentRequest {
            tenant_id: "tenant-1".to_owned(),
            organization_id: Some("org-1".to_owned()),
            owner_user_id: "user-1".to_owned(),
            order_id: "order-1".to_owned(),
        };
        let queried = adapter
            .reconcile_owner_order_payment(request.clone())
            .await
            .expect("sandbox provider query");
        assert!(!queried.replayed);

        let payment_store = PostgresCommerceOwnerOrderPaymentStore::new(pool.clone());
        let confirmed = payment_store
            .confirm_owner_order_payment(&queried.attempt)
            .await
            .expect("transactional payment confirmation");
        assert!(!confirmed.replayed);

        let replay = adapter
            .reconcile_owner_order_payment(request)
            .await
            .expect("idempotent reconciliation replay");
        assert!(replay.replayed);
        assert_eq!(
            replay.attempt.payment_attempt_id.as_deref(),
            Some("attempt-1")
        );

        let row = sqlx::query(
            "SELECT status, to_char(paid_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"') AS paid_at FROM commerce_payment_attempt WHERE id = 'attempt-1'",
        )
        .fetch_one(&pool)
        .await
        .expect("confirmed attempt");
        assert_eq!(
            Row::try_get::<String, _>(&row, "status").unwrap(),
            "succeeded"
        );
        assert!(Row::try_get::<Option<String>, _>(&row, "paid_at")
            .unwrap()
            .is_some());
    }
}
