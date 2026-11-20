//! Refund-request webhook settlement mirroring onto the after-sales case.
//!
//! An approved after-sales refund creates the order refund request and links
//! it on `commerce_after_sales_request.refund_request_id`. The asynchronous
//! provider outcome arrives later through the refund webhook; the settlement
//! store terminalizes the refund request and must move the after-sales
//! `refund_status` in the same transaction.
//!
//! Requires `SDKWORK_DATABASE_TEST_POSTGRES_URL`; skips cleanly when unset.

use sdkwork_order_repository_sqlx::{
    order_points_recharge_e2e_postgres_pool_from_env, PostgresRefundRequestSettlementStore,
};
use sdkwork_order_service::RefundRequestSettlementPort;
use sqlx::Row;

const TENANT: &str = "tenant-after-sales-sync";
const ORGANIZATION: &str = "org-1";
const REQUEST_ID: &str = "refund-after-sales-sync-1";

async fn cleanup(pool: &sqlx::PgPool) {
    for statement in [
        "DELETE FROM commerce_after_sales_event WHERE tenant_id = 'tenant-after-sales-sync'",
        "DELETE FROM commerce_after_sales_request WHERE tenant_id = 'tenant-after-sales-sync'",
        "DELETE FROM commerce_order_refund_request WHERE tenant_id = 'tenant-after-sales-sync'",
    ] {
        let _ = sqlx::query(statement).execute(pool).await;
    }
}

#[tokio::test]
async fn webhook_terminalization_mirrors_the_refund_status_onto_the_after_sales_case() {
    let Some(pool) = order_points_recharge_e2e_postgres_pool_from_env().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    cleanup(&pool).await;
    sqlx::query(
        r#"
        INSERT INTO commerce_order_refund_request
            (id, tenant_id, organization_id, request_no, original_order_id, owner_user_id,
             target_asset, amount, currency_code, status, idempotency_key, created_at, updated_at)
        VALUES
            ($1, $2, $3, 'AS-REFUND-REQ-1', 'order-after-sales-sync-1', '200001',
             'token_bank', 500, 'CNY', 'provider_refund_processing',
             'after-sales-refund-sync-key', NOW(), NOW())
        "#,
    )
    .bind(REQUEST_ID)
    .bind(TENANT)
    .bind(ORGANIZATION)
    .execute(&pool)
    .await
    .expect("seed refund request");
    sqlx::query(
        r#"
        INSERT INTO commerce_after_sales_request
            (id, tenant_id, organization_id, after_sales_no, order_id, owner_user_id,
             after_sales_type, status, refund_status, return_status, exchange_status,
             reason_code, description, requested_amount, approved_amount, refund_request_id,
             currency_code, requested_by_type, requested_by, request_no, idempotency_key,
             created_at, updated_at)
        VALUES
            ('after-sales-sync-1', $1, $2, 'AS-SYNC-1', 'order-after-sales-sync-1', '200001',
             'refund', 'approved', 'processing', 'none', 'none', 'quality', '',
             500, 500, $3, 'CNY', 'buyer', '200001', 'AS-SYNC-REQ-1', 'as-sync-idem-1',
             NOW(), NOW())
        "#,
    )
    .bind(TENANT)
    .bind(ORGANIZATION)
    .bind(REQUEST_ID)
    .execute(&pool)
    .await
    .expect("seed after-sales case");

    let store = PostgresRefundRequestSettlementStore::new(pool.clone());
    let settled = store
        .mark_refund_request_settled(TENANT, REQUEST_ID, "refunded", Some("RF-SYNC-1"))
        .await
        .expect("settle refund request");
    assert!(settled);

    let request = sqlx::query("SELECT status, provider_reference_id FROM commerce_order_refund_request WHERE id = $1")
        .bind(REQUEST_ID)
        .fetch_one(&pool)
        .await
        .expect("refund request row");
    assert_eq!(
        request.try_get::<String, _>("status").unwrap(),
        "refunded"
    );
    assert_eq!(
        request
            .try_get::<Option<String>, _>("provider_reference_id")
            .unwrap()
            .as_deref(),
        Some("RF-SYNC-1")
    );

    let after_sales = sqlx::query("SELECT refund_status FROM commerce_after_sales_request WHERE id = 'after-sales-sync-1'")
        .fetch_one(&pool)
        .await
        .expect("after sales row");
    assert_eq!(
        after_sales.try_get::<String, _>("refund_status").unwrap(),
        "refunded"
    );

    // A replay of the same webhook is a no-op: the in-flight guard keeps the
    // request immutable and never lets a second delivery re-run the sync.
    let replayed = store
        .mark_refund_request_settled(TENANT, REQUEST_ID, "provider_refund_failed", None)
        .await
        .expect("replay settlement");
    assert!(!replayed);
    let after_sales = sqlx::query("SELECT refund_status FROM commerce_after_sales_request WHERE id = 'after-sales-sync-1'")
        .fetch_one(&pool)
        .await
        .expect("after sales row after replay");
    assert_eq!(
        after_sales.try_get::<String, _>("refund_status").unwrap(),
        "refunded"
    );
    cleanup(&pool).await;
}
