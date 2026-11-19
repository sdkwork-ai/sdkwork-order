//! DB-backed payment compensation worker tests: claim → PSP query (sandbox)
//! → synthetic event → notify framework application.
//!
//! The sandbox provider short-circuits provider queries to `succeeded`, so
//! these tests exercise the full compensation pipeline — claim windowing,
//! synthetic event construction, idempotent ingest, terminal state
//! application, and order-side settlement — against real PostgreSQL.
//! Skipped when `SDKWORK_DATABASE_TEST_POSTGRES_URL` is not configured.

use sdkwork_order_integration_payment::{
    run_payment_compensation_pass, PaymentCompensationPassConfig,
};
use sdkwork_order_repository_sqlx::{
    order_points_recharge_e2e_postgres_pool_from_env, PostgresCommerceOrderStore,
    PostgresCommerceRechargeStore,
};
use sdkwork_order_service::{
    AccountPointsCreditFuture, AccountPointsCreditPort, FulfillPaidPhysicalOrderRequest,
    NoopAccountValueLedgerPort, NoopAppTemplatePurchaseFulfillmentPort, NoopCouponRedemptionPort,
    NoopMembershipPurchaseFulfillmentPort, NoopPromotionDiscountPort, OwnerOrderSettlementPorts,
    PhysicalGoodsFulfillmentOutcome, PhysicalGoodsFulfillmentPort, PhysicalGoodsFuture,
    PointsRechargeCreditOutcome, PointsRechargeCreditRequest,
    UnavailableVirtualGoodsFulfillmentPort,
};
use sdkwork_payment_providers::ProviderCredentialBundle;
use sdkwork_payment_repository_sqlx::PostgresCommerceOwnerOrderPaymentStore;
use sqlx::PgPool;

struct NoopAccountPointsCreditPort;

impl AccountPointsCreditPort for NoopAccountPointsCreditPort {
    fn credit_points_recharge<'a>(
        &'a self,
        _request: PointsRechargeCreditRequest,
    ) -> AccountPointsCreditFuture<'a, PointsRechargeCreditOutcome> {
        Box::pin(async move {
            Ok(PointsRechargeCreditOutcome {
                accepted: true,
                replayed: false,
            })
        })
    }

    fn reverse_points_recharge_credit<'a>(
        &'a self,
        _request: PointsRechargeCreditRequest,
    ) -> AccountPointsCreditFuture<'a, PointsRechargeCreditOutcome> {
        Box::pin(async move {
            Ok(PointsRechargeCreditOutcome {
                accepted: true,
                replayed: false,
            })
        })
    }
}

/// These DB tests seed fixed fixture ids under one tenant, and a compensation
/// pass scans every due row of that tenant, so they must not run concurrently
/// with each other.
static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn fixture() -> Option<PgPool> {
    order_points_recharge_e2e_postgres_pool_from_env().await
}

/// The three tests share one dedicated tenant. Holding `TEST_LOCK` and
/// clearing the tenant before each test keeps a compensation pass (which scans
/// every due row of the tenant) from claiming another test's rows or a
/// previous run's leftovers.
async fn reset_fixture_tenant(pool: &PgPool) {
    for sql in [
        "DELETE FROM commerce_refund_event WHERE tenant_id = 'tenant-1'",
        "DELETE FROM commerce_payment_webhook_event WHERE tenant_id = 'tenant-1'",
        "DELETE FROM commerce_refund WHERE tenant_id = 'tenant-1'",
        "DELETE FROM commerce_payment_attempt WHERE tenant_id = 'tenant-1'",
        "DELETE FROM commerce_payment_intent WHERE tenant_id = 'tenant-1'",
        "DELETE FROM commerce_order WHERE tenant_id = 'tenant-1'",
    ] {
        let _ = sqlx::query(sql).execute(pool).await;
    }
}

fn succeeded_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "succeeded" | "success" | "paid"
    )
}

fn now_iso() -> String {
    sdkwork_order_service::canonical_now_timestamp()
}

fn minutes_ago_iso(minutes: i64) -> String {
    sdkwork_order_service::canonical_timestamp_after_seconds(-minutes * 60)
}

fn hours_ago_iso(hours: i64) -> String {
    sdkwork_order_service::canonical_timestamp_after_seconds(-hours * 3_600)
}

fn default_config() -> PaymentCompensationPassConfig {
    PaymentCompensationPassConfig {
        tenant_id: Some("tenant-1".to_owned()),
        organization_id: None,
        batch_size: 100,
        min_age_seconds: 60,
    }
}

/// Seeded order in the payable state; `created_at` can be shifted with a SQL
/// interval expression to control the claim window.
async fn insert_order(pool: &PgPool, order_id: &str, created_at: &str) -> Result<(), sqlx::Error> {
    // Re-runs against a persistent test database must not collide with the
    // previous run's fixed fixture ids.
    sqlx::query("DELETE FROM commerce_order WHERE id = $1")
        .bind(order_id)
        .execute(pool)
        .await?;
    sqlx::query(
        r#"
        INSERT INTO commerce_order
            (id, tenant_id, organization_id, owner_user_id, order_no, status,
             payment_status, fulfillment_status, refund_status, subject, currency_code,
             request_no, idempotency_key, created_at, updated_at)
        VALUES
            ($1, 'tenant-1', '0', 'user-1', $1, 'pending_payment',
             'unpaid', 'unfulfilled', NULL, 'product', 'CNY',
             $1, $1, $2::timestamptz, $2::timestamptz)
        "#,
    )
    .bind(order_id)
    .bind(created_at)
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_payment_attempt(
    pool: &PgPool,
    attempt_id: &str,
    order_id: &str,
    out_trade_no: &str,
    status: &str,
    created_at: &str,
) -> Result<(), sqlx::Error> {
    let payment_intent_id = format!("pi-{attempt_id}");
    sqlx::query("DELETE FROM commerce_payment_attempt WHERE id = $1")
        .bind(attempt_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM commerce_payment_intent WHERE id = $1")
        .bind(&payment_intent_id)
        .execute(pool)
        .await?;
    sqlx::query(
        r#"
        INSERT INTO commerce_payment_intent
            (id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
             payment_method, provider_code, amount, currency_code, status, request_no,
             idempotency_key, created_at, updated_at)
        VALUES
            ($1, 'tenant-1', '0', 'user-1', $2, $1, 'wechat', 'sandbox', 100, 'CNY',
             'pending', $1, $1, $3::timestamptz, $3::timestamptz)
        "#,
    )
    .bind(format!("pi-{attempt_id}"))
    .bind(order_id)
    .bind(created_at)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO commerce_payment_attempt
            (id, tenant_id, organization_id, owner_user_id, payment_intent_id, order_id,
             payment_method, provider_code, channel_id, out_trade_no, amount, currency_code,
             status, callback_payload, request_no, idempotency_key, expires_at, created_at,
             paid_at, updated_at)
        VALUES
            ($1, 'tenant-1', '0', 'user-1', $2, $3, 'wechat', 'sandbox', NULL, $4,
             100, 'CNY', $5, '{}'::jsonb, $1, $1, NULL, $6::timestamptz, $7::timestamptz,
             $6::timestamptz)
        "#,
    )
    .bind(attempt_id)
    .bind(format!("pi-{attempt_id}"))
    .bind(order_id)
    .bind(out_trade_no)
    .bind(status)
    .bind(created_at)
    // A terminal-success attempt always persists paid_at in production; the
    // convergence sweep fails closed on a succeeded row without it, so the
    // fixture must not seed an invariant-violating row.
    .bind(succeeded_status(status).then(|| created_at.to_owned()))
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_refund(
    pool: &PgPool,
    refund_no: &str,
    order_id: &str,
    payment_attempt_id: &str,
    status: &str,
    created_at: &str,
) -> Result<(), sqlx::Error> {
    let refund_id = format!("refund-{refund_no}");
    sqlx::query("DELETE FROM commerce_refund WHERE id = $1")
        .bind(&refund_id)
        .execute(pool)
        .await?;
    sqlx::query(
        r#"
        INSERT INTO commerce_refund
            (id, tenant_id, organization_id, order_id, payment_attempt_id, refund_no,
             amount, currency_code, status, refund_reason_code, requested_by_type,
             requested_by, request_no, idempotency_key, created_at, updated_at)
        VALUES
            ($1, 'tenant-1', '0', $2, $3, $4, 100, 'CNY', $5, 'buyer', 'buyer',
             'user-1', $1, $1, $6::timestamptz, $6::timestamptz)
        "#,
    )
    .bind(&refund_id)
    .bind(order_id)
    .bind(payment_attempt_id)
    .bind(refund_no)
    .bind(status)
    .bind(created_at)
    .execute(pool)
    .await?;
    Ok(())
}

async fn attempt_status(pool: &PgPool, attempt_id: &str) -> String {
    sqlx::query_scalar("SELECT status FROM commerce_payment_attempt WHERE id = $1")
        .bind(attempt_id)
        .fetch_one(pool)
        .await
        .unwrap_or_default()
}

async fn refund_status(pool: &PgPool, refund_no: &str) -> String {
    sqlx::query_scalar("SELECT status FROM commerce_refund WHERE refund_no = $1")
        .bind(refund_no)
        .fetch_one(pool)
        .await
        .unwrap_or_default()
}

async fn order_payment_status(pool: &PgPool, order_id: &str) -> String {
    sqlx::query_scalar("SELECT COALESCE(payment_status, '') FROM commerce_order WHERE id = $1")
        .bind(order_id)
        .fetch_one(pool)
        .await
        .unwrap_or_default()
}

async fn order_refund_status(pool: &PgPool, order_id: &str) -> String {
    sqlx::query_scalar("SELECT COALESCE(refund_status, '') FROM commerce_order WHERE id = $1")
        .bind(order_id)
        .fetch_one(pool)
        .await
        .unwrap_or_default()
}

async fn webhook_event_count(pool: &PgPool, provider_code: &str, out_trade_no: &str) -> i64 {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM commerce_payment_webhook_event
        WHERE provider_code = $1
          AND (
                payload->'normalized'->>'outTradeNo' = $2
             OR payload->'providerPayload'->>'out_trade_no' = $2
          )
        "#,
    )
    .bind(provider_code)
    .bind(out_trade_no)
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

struct TestSettlement {
    payments: PostgresCommerceOwnerOrderPaymentStore,
    orders: PostgresCommerceOrderStore,
    recharge: PostgresCommerceRechargeStore,
}

impl TestSettlement {
    fn new(pool: PgPool) -> Self {
        Self {
            payments: PostgresCommerceOwnerOrderPaymentStore::new(pool.clone()),
            orders: PostgresCommerceOrderStore::new(pool.clone()),
            recharge: PostgresCommerceRechargeStore::new(pool),
        }
    }

    fn ports(&self) -> OwnerOrderSettlementPorts<'_> {
        OwnerOrderSettlementPorts {
            payment_store: &self.payments,
            order_state_store: &self.orders,
            recharge_store: &self.recharge,
            account_value_store: &self.recharge,
            credit_port: &NoopAccountPointsCreditPort,
            account_value_ledger_port: &NoopAccountValueLedgerPort,
            coupon_redemption_port: &NoopCouponRedemptionPort,
            membership_port: &NoopMembershipPurchaseFulfillmentPort,
            app_template_port: &NoopAppTemplatePurchaseFulfillmentPort,
            physical_goods_port: &AcceptingPhysicalGoodsFulfillmentPort,
        virtual_goods_port: &UnavailableVirtualGoodsFulfillmentPort,
            promotion_discount_port: &NoopPromotionDiscountPort,
        }
    }
}

/// Accepting physical-goods port: these fixtures model a compensated payment
/// settlement, not a fulfillment outage. The unavailable port would fail the
/// settlement saga (and the test would assert compensation errors instead of
/// the settlement the test is about).
struct AcceptingPhysicalGoodsFulfillmentPort;

impl PhysicalGoodsFulfillmentPort for AcceptingPhysicalGoodsFulfillmentPort {
    fn fulfill_paid_physical_order<'a>(
        &'a self,
        _request: FulfillPaidPhysicalOrderRequest,
    ) -> PhysicalGoodsFuture<'a, PhysicalGoodsFulfillmentOutcome> {
        Box::pin(async move {
            Ok(PhysicalGoodsFulfillmentOutcome {
                accepted: true,
                replayed: false,
                fulfillment_status: "awaiting_shipment".to_owned(),
            })
        })
    }
}

#[tokio::test]
async fn sandbox_compensation_pass_settles_due_payment_attempt() {
    let _guard = TEST_LOCK.lock().await;
    let Some(pool) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    reset_fixture_tenant(&pool).await;
    insert_order(&pool, "order-comp-1", &minutes_ago_iso(5))
        .await
        .expect("seed order");
    insert_payment_attempt(
        &pool,
        "attempt-comp-1",
        "order-comp-1",
        "trade-comp-1",
        "pending",
        &minutes_ago_iso(5),
    )
    .await
    .expect("seed due payment attempt");

    let settlement = TestSettlement::new(pool.clone());
    let summary = run_payment_compensation_pass(
        &pool,
        &ProviderCredentialBundle::from_env(),
        settlement.ports(),
        &default_config(),
    )
    .await
    .expect("compensation pass must run");
    assert_eq!(1, summary.claimed_payment_attempts);
    assert_eq!(1, summary.payment_events_applied);
    assert_eq!(0, summary.errors);

    // The synthetic event ran through the full notify pipeline: attempt
    // succeeded, order paid, webhook audit row persisted.
    assert_eq!("succeeded", attempt_status(&pool, "attempt-comp-1").await);
    assert_eq!("success", order_payment_status(&pool, "order-comp-1").await);
    assert_eq!(
        1,
        webhook_event_count(&pool, "sandbox", "trade-comp-1").await
    );

    // A second pass claims nothing: the attempt left the scan window states.
    let again = run_payment_compensation_pass(
        &pool,
        &ProviderCredentialBundle::from_env(),
        settlement.ports(),
        &default_config(),
    )
    .await
    .expect("second compensation pass must run");
    assert_eq!(0, again.claimed_payment_attempts);
    assert_eq!(0, again.payment_events_applied);
}

#[tokio::test]
async fn sandbox_compensation_pass_marks_due_refund_succeeded_and_order_refunded() {
    let _guard = TEST_LOCK.lock().await;
    let Some(pool) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    reset_fixture_tenant(&pool).await;
    insert_order(&pool, "order-comp-refund", &minutes_ago_iso(10))
        .await
        .expect("seed order");
    insert_payment_attempt(
        &pool,
        "attempt-comp-refund",
        "order-comp-refund",
        "trade-comp-refund",
        "succeeded",
        &minutes_ago_iso(10),
    )
    .await
    .expect("seed settled payment attempt");
    insert_refund(
        &pool,
        "refund-comp-1",
        "order-comp-refund",
        "attempt-comp-refund",
        "processing",
        &minutes_ago_iso(5),
    )
    .await
    .expect("seed due refund");

    let settlement = TestSettlement::new(pool.clone());
    let summary = run_payment_compensation_pass(
        &pool,
        &ProviderCredentialBundle::from_env(),
        settlement.ports(),
        &default_config(),
    )
    .await
    .expect("compensation pass must run");
    assert_eq!(1, summary.claimed_refunds);
    assert_eq!(1, summary.refund_events_applied);
    assert_eq!(0, summary.errors);

    assert_eq!("succeeded", refund_status(&pool, "refund-comp-1").await);
    assert_eq!(
        "refunded",
        order_refund_status(&pool, "order-comp-refund").await
    );
}

#[tokio::test]
async fn compensation_pass_respects_scan_window_and_terminal_states() {
    let _guard = TEST_LOCK.lock().await;
    let Some(pool) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    reset_fixture_tenant(&pool).await;
    // Fresh attempt (younger than min_age) must not be claimed yet.
    insert_order(&pool, "order-comp-fresh", &now_iso())
        .await
        .expect("seed fresh order");
    insert_payment_attempt(
        &pool,
        "attempt-comp-fresh",
        "order-comp-fresh",
        "trade-comp-fresh",
        "pending",
        &now_iso(),
    )
    .await
    .expect("seed fresh attempt");
    // Aged attempt (well past the webhook window) must be claimed: the
    // compensation sweep is the at-least-once safety net and the claim set is
    // bounded by the attempt's own expiry, not by age.
    insert_order(&pool, "order-comp-aged", &hours_ago_iso(25))
        .await
        .expect("seed aged order");
    insert_payment_attempt(
        &pool,
        "attempt-comp-aged",
        "order-comp-aged",
        "trade-comp-aged",
        "pending",
        &hours_ago_iso(25),
    )
    .await
    .expect("seed aged attempt");
    // Terminal attempt (already succeeded) must never be claimed.
    insert_order(&pool, "order-comp-terminal", &minutes_ago_iso(10))
        .await
        .expect("seed terminal order");
    insert_payment_attempt(
        &pool,
        "attempt-comp-terminal",
        "order-comp-terminal",
        "trade-comp-terminal",
        "succeeded",
        &minutes_ago_iso(10),
    )
    .await
    .expect("seed terminal attempt");

    let settlement = TestSettlement::new(pool.clone());
    let summary = run_payment_compensation_pass(
        &pool,
        &ProviderCredentialBundle::from_env(),
        settlement.ports(),
        &default_config(),
    )
    .await
    .expect("compensation pass must run");
    // Fresh attempt: inside the webhook window, never claimed. Aged attempt:
    // claimed exactly once (idempotent replay makes repeats safe). Terminal
    // attempt: never claimed. PSP query outcomes are provider-dependent, so
    // the pass contract asserts on claims and error-free execution.
    assert_eq!(1, summary.claimed_payment_attempts);
    assert_eq!(0, summary.claimed_refunds);
    assert_eq!(0, summary.errors);
}
