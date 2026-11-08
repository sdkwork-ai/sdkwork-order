//! Points-recharge order creation against the production schema shape.
//!
//! Regression for the 50001 `POST /app/v3/api/recharges/orders` failure where
//! `commerce_order_item.created_at` was written from a Rust `String` bind
//! without an explicit `TIMESTAMPTZ` cast, so PostgreSQL rejected the INSERT
//! with `column "created_at" is of type timestamp with time zone but expression
//! is of type text`. Every order-owned instant column became native
//! `TIMESTAMPTZ` in migration 0017, so text binds must be cast at the SQL
//! boundary.
//!
//! Skipped when `SDKWORK_DATABASE_TEST_POSTGRES_URL` is not configured (the
//! standard order PG harness behavior).

use chrono::{Duration, SecondsFormat, Utc};
use sdkwork_contract_service::CommerceMoney;
use sdkwork_order_repository_sqlx::{
    order_points_recharge_e2e_postgres_pool_from_env, PostgresCommerceRechargeStore,
};
use sdkwork_order_service::{
    CreatePointsRechargeOrderCommand, MarkPointsRechargePaymentSucceededCommand,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

const TENANT: &str = "100001";
const ORGANIZATION: &str = "0";
const OWNER: &str = "900123";
const METHOD: &str = "wechat_pay";

fn timestamp(offset_minutes: i64) -> String {
    let instant = Utc::now() + Duration::minutes(offset_minutes);
    instant.to_rfc3339_opts(SecondsFormat::Micros, true)
}

async fn seed_recharge_package(pool: &PgPool, package_id: &str, sku_id: &str) {
    sqlx::query(
        r#"
        INSERT INTO commerce_recharge_package
            (id, tenant_id, organization_id, external_id, package_no, sku_id, name,
             price_amount, currency_code, bonus_points, discount, status,
             sort_weight, request_no, idempotency_key, created_at, updated_at)
        VALUES
            ($1, $2, $3, 1234, $4, $5, 'Probe points package',
             CAST($6 AS NUMERIC), 'CNY', 0, 100, 'active',
             1, $7, $8, CAST($9 AS TIMESTAMPTZ), CAST($9 AS TIMESTAMPTZ))
        "#,
    )
    .bind(package_id)
    .bind(TENANT)
    .bind(ORGANIZATION)
    .bind(format!("pkg-{package_id}"))
    .bind(sku_id)
    .bind("2999")
    .bind(format!("request-{package_id}"))
    .bind(format!("idem-{package_id}"))
    .bind(timestamp(0))
    .execute(pool)
    .await
    .expect("seed commerce_recharge_package");
}

async fn cleanup(pool: &PgPool, order_id: &str, package_id: &str) {
    for statement in [
        "DELETE FROM commerce_order_item WHERE order_id = $1",
        "DELETE FROM commerce_order_amount_breakdown WHERE order_id = $1",
        "DELETE FROM commerce_order WHERE id = $1",
    ] {
        let _ = sqlx::query(statement).bind(order_id).execute(pool).await;
    }
    let _ = sqlx::query("DELETE FROM commerce_recharge_package WHERE id = $1")
        .bind(package_id)
        .execute(pool)
        .await;
}

#[tokio::test]
async fn creates_points_recharge_order_with_native_instant_columns() {
    let Some(pool) = order_points_recharge_e2e_postgres_pool_from_env().await else {
        eprintln!("SDKWORK_DATABASE_TEST_POSTGRES_URL is unset; skipping points recharge PG test");
        return;
    };

    let suffix = Uuid::new_v4().simple().to_string();
    let package_id = format!("pkg-{suffix}");
    let sku_id = format!("sku-{suffix}");
    let order_id = format!("order-{suffix}");
    let order_item_id = format!("order-item-{suffix}");
    let requested_at = timestamp(0);
    let expire_at = timestamp(30);

    seed_recharge_package(&pool, &package_id, &sku_id).await;
    let store = PostgresCommerceRechargeStore::new(pool.clone());

    let command = CreatePointsRechargeOrderCommand::new(
        TENANT,
        Some(ORGANIZATION),
        OWNER,
        CommerceMoney::new("2999").expect("amount"),
        "CNY",
        METHOD,
        &order_id,
        &order_item_id,
        &format!("intent-{suffix}"),
        &format!("attempt-{suffix}"),
        &format!("RCH-{suffix}"),
        &format!("TRADE-{suffix}"),
        &requested_at,
        &expire_at,
        &format!("idem-{suffix}"),
        Some(&package_id),
        None,
        Some("app"),
    )
    .expect("create recharge command");

    let outcome = store
        .create_points_recharge_order(command.clone())
        .await
        .expect("recharge order must be created against native TIMESTAMPTZ columns");
    assert!(outcome.success);
    assert_eq!(outcome.order_id, order_id);

    let item = sqlx::query(
        "SELECT created_at, CAST(total_amount AS TEXT) AS total_amount \
         FROM commerce_order_item WHERE order_id = $1",
    )
    .bind(&order_id)
    .fetch_one(&pool)
    .await
    .expect("recharge order item row");
    let created_at: chrono::DateTime<Utc> = item.get("created_at");
    assert!(created_at.timestamp() > 0);
    assert_eq!(item.get::<String, _>("total_amount"), "2999");

    let breakdown: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM commerce_order_amount_breakdown WHERE order_id = $1",
    )
    .bind(&order_id)
    .fetch_one(&pool)
    .await
    .expect("recharge amount breakdown count");
    assert_eq!(breakdown, 1);

    // The payment-settlement UPDATEs share the same native instant columns and
    // must accept text RFC 3339 binds through explicit casts.
    let payment_command = MarkPointsRechargePaymentSucceededCommand::new(
        TENANT,
        Some(ORGANIZATION),
        OWNER,
        &order_id,
        &timestamp(1),
        &format!("pay-{suffix}"),
        &format!("pay-idem-{suffix}"),
    )
    .expect("mark succeeded command");
    store
        .mark_points_recharge_payment_succeeded(payment_command)
        .await
        .expect("recharge payment settlement must accept native instant columns");

    cleanup(&pool, &order_id, &package_id).await;
}
