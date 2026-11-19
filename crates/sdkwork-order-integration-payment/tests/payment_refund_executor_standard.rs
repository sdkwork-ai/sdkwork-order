use sdkwork_contract_service::CommerceMoney;
use sdkwork_order_integration_payment::StorePaymentRefundExecutorAdapter;
use sdkwork_order_service::{PaymentRefundExecutionRequest, PaymentRefundExecutorPort};
use sqlx::{postgres::PgPoolOptions, Row};

fn postgres_url() -> Option<String> {
    std::env::var("SDKWORK_DATABASE_TEST_POSTGRES_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[tokio::test]
async fn sandbox_refund_executor_creates_payment_refund_and_returns_payment_reference() {
    // 服务端测试必须使用 PostgreSQL（DATABASE_SPEC：authoritative-server）
    let Some((pool, schema)) = payment_refund_pool().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    seed_paid_sandbox_order(&pool).await;
    let adapter = StorePaymentRefundExecutorAdapter::postgres(pool.clone());

    let outcome = adapter
        .execute_provider_refund(PaymentRefundExecutionRequest {
            tenant_id: "tenant-1".to_owned(),
            organization_id: Some("org-1".to_owned()),
            owner_user_id: "user-1".to_owned(),
            refund_request_id: "refund-request-1".to_owned(),
            original_order_id: "order-1".to_owned(),
            amount: CommerceMoney::new("500").expect("amount"),
            currency_code: "USD".to_owned(),
            request_no: "refund-request-1".to_owned(),
            idempotency_key: "refund-exec-idem-1".to_owned(),
        })
        .await
        .expect("refund execution");

    assert!(outcome.accepted);
    assert_eq!(outcome.status, "succeeded");
    let provider_reference_id = outcome
        .provider_reference_id
        .as_deref()
        .expect("provider reference");

    let row = sqlx::query(
        r#"
        SELECT id, order_id, payment_attempt_id, amount, currency_code, status, idempotency_key
        FROM commerce_refund
        WHERE id = $1
        "#,
    )
    .bind(provider_reference_id)
    .fetch_one(&pool)
    .await
    .expect("refund row");
    assert_eq!(row.get::<String, _>("order_id"), "order-1");
    assert_eq!(
        row.get::<String, _>("payment_attempt_id"),
        "payment-attempt-1"
    );
    assert_eq!(row.get::<String, _>("amount"), "500");
    assert_eq!(row.get::<String, _>("currency_code"), "USD");
    assert_eq!(row.get::<String, _>("status"), "succeeded");
    assert_eq!(
        row.get::<String, _>("idempotency_key"),
        "refund-exec-idem-1"
    );
    drop_payment_refund_schema(&pool, &schema).await;
}

#[tokio::test]
async fn sandbox_refund_executor_reuses_payment_refund_for_same_idempotency_key() {
    // 服务端测试必须使用 PostgreSQL（DATABASE_SPEC：authoritative-server）
    let Some((pool, schema)) = payment_refund_pool().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    seed_paid_sandbox_order(&pool).await;
    let adapter = StorePaymentRefundExecutorAdapter::postgres(pool.clone());
    let request = PaymentRefundExecutionRequest {
        tenant_id: "tenant-1".to_owned(),
        organization_id: Some("org-1".to_owned()),
        owner_user_id: "user-1".to_owned(),
        refund_request_id: "refund-request-1".to_owned(),
        original_order_id: "order-1".to_owned(),
        amount: CommerceMoney::new("500").expect("amount"),
        currency_code: "USD".to_owned(),
        request_no: "refund-request-1".to_owned(),
        idempotency_key: "refund-exec-idem-1".to_owned(),
    };

    let first = adapter
        .execute_provider_refund(request.clone())
        .await
        .expect("first refund");
    let second = adapter
        .execute_provider_refund(request)
        .await
        .expect("replayed refund");

    assert_eq!(first.provider_reference_id, second.provider_reference_id);
    assert_eq!(first.status, "succeeded");
    assert_eq!(second.status, "succeeded");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM commerce_refund")
        .fetch_one(&pool)
        .await
        .expect("refund count");
    assert_eq!(count, 1);
    drop_payment_refund_schema(&pool, &schema).await;
}

/// Each call creates its own schema so the two tests (and every other binary
/// sharing the test database) never share reduced `commerce_*` tables in
/// `public`; `CREATE TABLE IF NOT EXISTS` migrations elsewhere then keep the
/// production shape instead of inheriting this fixture's subset.
static SCHEMA_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

async fn payment_refund_pool() -> Option<(sqlx::PgPool, String)> {
    let url = postgres_url()?;
    let seq = SCHEMA_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let schema = format!("order_refund_{}_{seq}", std::process::id());
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .ok()?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP SCHEMA IF EXISTS {schema} CASCADE"
    )))
    .execute(&admin)
    .await
    .ok()?;
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await
        .ok()?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .after_connect({
            let schema = schema.clone();
            move |connection, _| {
                let schema = schema.clone();
                Box::pin(async move {
                    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                        "SET search_path TO {schema}, public"
                    )))
                    .execute(connection)
                    .await?;
                    Ok(())
                })
            }
        })
        .connect(&url)
        .await
        .ok()?;
    for statement in split_sql_statements(PAYMENT_REFUND_SCHEMA) {
        sqlx::query(sqlx::AssertSqlSafe(statement.as_str()))
            .execute(&pool)
            .await
            .unwrap_or_else(|error| panic!("schema failed on `{statement}`: {error}"));
    }
    Some((pool, schema))
}

async fn drop_payment_refund_schema(pool: &sqlx::PgPool, schema: &str) {
    let _ = sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP SCHEMA IF EXISTS {schema} CASCADE"
    )))
    .execute(pool)
    .await;
}

async fn seed_paid_sandbox_order(pool: &sqlx::PgPool) {
    let now = "2026-07-08T00:00:00Z";
    sqlx::query(
        r#"
        INSERT INTO commerce_order
            (id, tenant_id, organization_id, owner_user_id, order_no, status, subject,
             currency_code, payment_status, paid_at, expired_at, created_at, updated_at)
        VALUES
            ('order-1', 'tenant-1', 'org-1', 'user-1', 'ORDER-1', 'paid',
             'token_bank_recharge', 'USD', 'paid', CAST($1 AS TIMESTAMPTZ),
             '2099-01-01T00:00:00Z'::timestamptz, CAST($2 AS TIMESTAMPTZ), CAST($3 AS TIMESTAMPTZ))
        "#,
    )
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed order");

    sqlx::query(
        r#"
        INSERT INTO commerce_order_amount_breakdown
            (id, tenant_id, organization_id, order_id, allocation_type, payable_amount,
             discount_amount, currency_code, created_at)
        VALUES
            ('breakdown-1', 'tenant-1', 'org-1', 'order-1', 'order_total',
             '1000', '0', 'USD', $1)
        "#,
    )
    .bind(now)
    .execute(pool)
    .await
    .expect("seed amount");

    sqlx::query(
        r#"
        INSERT INTO commerce_payment_attempt
            (id, tenant_id, organization_id, owner_user_id, payment_intent_id, order_id,
             attempt_no, payment_method, provider_code, out_trade_no, amount, currency_code,
             status, callback_payload, paid_at, request_no, idempotency_key, created_at, updated_at)
            VALUES
            ('payment-attempt-1', 'tenant-1', 'org-1', 'user-1', 'payment-intent-1', 'order-1',
             'PAY-ATTEMPT-1', 'sandbox', 'sandbox', 'OUT-TRADE-1', '1000', 'USD',
             'succeeded', '{}', CAST($1 AS TIMESTAMPTZ), 'pay-request-1', 'pay-idem-1',
             CAST($2 AS TIMESTAMPTZ), CAST($3 AS TIMESTAMPTZ))
        "#,
    )
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed payment attempt");
}

fn split_sql_statements(sql: &str) -> Vec<String> {
    sql.split(';')
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
        .map(str::to_owned)
        .collect()
}

const PAYMENT_REFUND_SCHEMA: &str = r#"
DROP TABLE IF EXISTS commerce_refund_event;
DROP TABLE IF EXISTS commerce_refund;
DROP TABLE IF EXISTS commerce_payment_attempt;
DROP TABLE IF EXISTS commerce_payment_channel;
DROP TABLE IF EXISTS commerce_payment_notify_domain;
DROP TABLE IF EXISTS commerce_order_amount_breakdown;
DROP TABLE IF EXISTS commerce_order;
CREATE TABLE commerce_order (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    owner_user_id TEXT NOT NULL,
    order_no TEXT NOT NULL,
    status TEXT NOT NULL,
    subject TEXT NOT NULL,
    currency_code TEXT NOT NULL,
    merchant_organization_id TEXT,
    shop_id TEXT,
    shipping_address_snapshot_json TEXT,
    shop_snapshot_json TEXT,
    payment_status TEXT,
    fulfillment_status TEXT,
    refund_status TEXT,
    request_no TEXT,
    idempotency_key TEXT,
    request_fingerprint TEXT,
    purchase_intent_key TEXT,
    membership_action TEXT,
    paid_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,
    expired_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE commerce_order_amount_breakdown (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    order_id TEXT NOT NULL,
    allocation_type TEXT NOT NULL,
    payable_amount NUMERIC(20,0) NOT NULL,
    discount_amount NUMERIC(20,0) NOT NULL DEFAULT 0,
    currency_code TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE commerce_payment_attempt (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    owner_user_id TEXT NOT NULL,
    payment_intent_id TEXT NOT NULL,
    order_id TEXT NOT NULL,
    attempt_no TEXT,
    payment_method TEXT NOT NULL,
    provider_code TEXT NOT NULL,
    channel_id TEXT,
    out_trade_no TEXT,
    amount TEXT NOT NULL,
    currency_code TEXT NOT NULL,
    status TEXT NOT NULL,
    callback_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    paid_at TIMESTAMPTZ,
    request_no TEXT,
    idempotency_key TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE TABLE commerce_payment_channel (
    id TEXT NOT NULL PRIMARY KEY,
    provider_account_id TEXT
);

-- Refund submission resolves the tenant's refund notify domain before calling
-- the PSP (shared resolve_refund_notify_url_postgres). The fixture models that
-- read so the submission path reaches the provider instead of failing with a
-- missing-relation storage error that the adapter classifies as retryable.
CREATE TABLE commerce_payment_notify_domain (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    protocol TEXT NOT NULL DEFAULT 'https',
    hostname TEXT NOT NULL,
    port INTEGER,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    status TEXT NOT NULL DEFAULT 'active',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE commerce_refund (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    order_id TEXT NOT NULL,
    payment_attempt_id TEXT NOT NULL,
    refund_no TEXT NOT NULL,
    amount TEXT NOT NULL,
    currency_code TEXT NOT NULL,
    status TEXT NOT NULL,
    refund_reason_code TEXT,
    requested_by_type TEXT NOT NULL,
    requested_by TEXT,
    request_no TEXT,
    idempotency_key TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    deleted_at TEXT
);

CREATE UNIQUE INDEX ux_commerce_refund_idempotency
    ON commerce_refund (tenant_id, order_id, idempotency_key)
    WHERE deleted_at IS NULL;

CREATE TABLE commerce_refund_event (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    event_no TEXT NOT NULL,
    refund_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    from_status TEXT,
    to_status TEXT NOT NULL,
    actor_type TEXT NOT NULL,
    actor_id TEXT,
    request_id TEXT,
    idempotency_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
"#;
