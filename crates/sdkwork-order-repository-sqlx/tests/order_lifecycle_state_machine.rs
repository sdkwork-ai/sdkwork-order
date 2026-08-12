//! DB-backed order lifecycle state machine tests: expiration sweep, receipt
//! confirmation, and merchant shipment advance. Skipped when
//! `SDKWORK_DATABASE_TEST_POSTGRES_URL` is not configured.

use sdkwork_order_repository_sqlx::{
    order_points_recharge_e2e_postgres_pool_from_env,
    postgres_expiration::{expire_due_order, list_due_expiring_orders},
    PostgresCommerceOrderStore, PostgresCommerceRechargeStore,
};
use sdkwork_order_service::{
    OrderOwnerDetailQuery, RefundNotifyStatePort, UpdateShipmentPackageCommand,
};
use sqlx::{PgPool, Row};

async fn fixture() -> Option<PgPool> {
    order_points_recharge_e2e_postgres_pool_from_env().await
}

#[allow(clippy::too_many_arguments)]
async fn insert_order(
    pool: &PgPool,
    order_id: &str,
    status: &str,
    payment_status: &str,
    fulfillment_status: Option<&str>,
    expired_at: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO commerce_order
            (id, tenant_id, organization_id, owner_user_id, order_no, status,
             payment_status, fulfillment_status, subject, currency_code,
             request_no, idempotency_key, created_at, updated_at, expired_at)
        VALUES
            ($1, 'tenant-1', '0', 'user-1', $1, $2, $3, $4, 'product', 'CNY',
             $1, $1, '1000', '1000', $5)
        "#,
    )
    .bind(order_id)
    .bind(status)
    .bind(payment_status)
    .bind(fulfillment_status)
    .bind(expired_at)
    .execute(pool)
    .await?;
    Ok(())
}

async fn event_count(pool: &PgPool, order_id: &str, event_type: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM commerce_order_event WHERE order_id = $1 AND event_type = $2",
    )
    .bind(order_id)
    .bind(event_type)
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

#[tokio::test]
async fn recovery_success_notify_on_failed_order_preserves_cancelled_with_audit_and_replay_idempotency(
) {
    let Some(pool) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    // Order previously marked failed/cancelled by a failure webhook.
    insert_order(
        &pool,
        "order-recover-1",
        "cancelled",
        "failed",
        Some("unfulfilled"),
        None,
    )
    .await
    .expect("seed cancelled order");

    let store = PostgresCommerceOrderStore::new(pool.clone());
    let attempt = sdkwork_order_service::OrderPaymentSettlementAttempt {
        tenant_id: "tenant-1".to_owned(),
        organization_id: Some("0".to_owned()),
        owner_user_id: "user-1".to_owned(),
        order_id: "order-recover-1".to_owned(),
        payment_attempt_id: Some("attempt-recv1".to_owned()),
        out_trade_no: Some("trade-recv1".to_owned()),
    };

    // Recovery success notify: the order stays cancelled (terminal preserved)
    // but the money-collected fact is recorded.
    let outcome = store
        .mark_owner_order_payment_succeeded(&attempt, "2026-08-12T00:00:00Z")
        .await
        .expect("recovery must not error");
    assert!(outcome.terminal_order_preserved);
    assert_eq!("cancelled", outcome.order_status);

    let row = sqlx::query(
        "SELECT status, payment_status FROM commerce_order WHERE id = 'order-recover-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("reload order");
    let status: String = row.try_get("status").unwrap_or_default();
    let payment_status: String = row.try_get("payment_status").unwrap_or_default();
    assert_eq!("cancelled", status);
    assert_eq!("success", payment_status);

    // Late-payment audit recorded exactly once.
    assert_eq!(
        1,
        event_count(&pool, "order-recover-1", "payment_succeeded_after_terminal").await
    );

    // Redelivery replay writes no additional audit rows (ON CONFLICT no-op).
    let replay = store
        .mark_owner_order_payment_succeeded(&attempt, "2026-08-12T00:00:05Z")
        .await
        .expect("replay must not error");
    assert!(replay.terminal_order_preserved);
    assert_eq!(
        1,
        event_count(&pool, "order-recover-1", "payment_succeeded_after_terminal").await
    );
    assert_eq!(
        1,
        event_count(&pool, "order-recover-1", "duplicate_payment_success").await
    );
    let second_replay = store
        .mark_owner_order_payment_succeeded(&attempt, "2026-08-12T00:00:10Z")
        .await
        .expect("second replay must not error");
    assert!(second_replay.terminal_order_preserved);
    assert_eq!(
        1,
        event_count(&pool, "order-recover-1", "duplicate_payment_success").await,
        "duplicate audit must stay bounded across replays"
    );
}

#[tokio::test]
async fn refund_notify_advances_order_refund_status_with_terminal_guard() {
    let Some(pool) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    insert_order(
        &pool,
        "order-refund-1",
        "paid",
        "success",
        Some("unfulfilled"),
        None,
    )
    .await
    .expect("seed refunded order");

    let store = PostgresCommerceOrderStore::new(pool.clone());
    let attempt = sdkwork_order_service::OrderPaymentSettlementAttempt {
        tenant_id: "tenant-1".to_owned(),
        organization_id: Some("0".to_owned()),
        owner_user_id: "user-1".to_owned(),
        order_id: "order-refund-1".to_owned(),
        payment_attempt_id: Some("attempt-r1".to_owned()),
        out_trade_no: Some("trade-r1".to_owned()),
    };

    let marked = store
        .mark_owner_order_refund_status(&attempt, "refunded")
        .await
        .expect("order must be marked refunded");
    assert!(!marked.terminal_preserved);
    assert_eq!("refunded", marked.refund_status);

    // A late refund_failed notify must never overwrite the refunded terminal.
    let preserved = store
        .mark_owner_order_refund_status(&attempt, "refund_failed")
        .await
        .expect("terminal refunded must be preserved");
    assert!(preserved.terminal_preserved);
    assert_eq!("refunded", preserved.refund_status);

    // Replay of the same state is idempotent.
    let replay = store
        .mark_owner_order_refund_status(&attempt, "refunded")
        .await
        .expect("refunded replay must be idempotent");
    assert_eq!("refunded", replay.refund_status);
}

#[tokio::test]
async fn payment_failure_webhook_preserves_paid_and_fulfilled_orders() {
    let Some(pool) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    insert_order(
        &pool,
        "order-fail-paid",
        "paid",
        "success",
        Some("unfulfilled"),
        None,
    )
    .await
    .expect("seed paid order");
    insert_order(
        &pool,
        "order-fail-fulfilled",
        "fulfilled",
        "success",
        Some("fulfilled"),
        None,
    )
    .await
    .expect("seed fulfilled order");

    let store = PostgresCommerceOrderStore::new(pool.clone());
    let attempt = sdkwork_order_service::OrderPaymentSettlementAttempt {
        tenant_id: "tenant-1".to_owned(),
        organization_id: Some("0".to_owned()),
        owner_user_id: "user-1".to_owned(),
        order_id: "order-fail-paid".to_owned(),
        payment_attempt_id: Some("attempt-rp1".to_owned()),
        out_trade_no: Some("trade-rp1".to_owned()),
    };

    // A late failure notify on a paid order must preserve the order and ack
    // (no cancelled flip, no error) — even cross-provider.
    let preserved = store
        .mark_owner_order_payment_failed(&attempt, "failed")
        .await
        .expect("paid order must be preserved without error");
    assert!(preserved.terminal_order_preserved);
    assert_eq!("paid", preserved.order_status);

    let fulfilled_attempt = sdkwork_order_service::OrderPaymentSettlementAttempt {
        order_id: "order-fail-fulfilled".to_owned(),
        ..attempt
    };
    let fulfilled = store
        .mark_owner_order_payment_failed(&fulfilled_attempt, "failed")
        .await
        .expect("fulfilled order must be preserved without error");
    assert!(fulfilled.terminal_order_preserved);
    assert_eq!("fulfilled", fulfilled.order_status);

    // The paid order must still read paid/success afterwards.
    let row = sqlx::query(
        "SELECT status, payment_status FROM commerce_order WHERE id = 'order-fail-paid'",
    )
    .fetch_one(&pool)
    .await
    .expect("reload order");
    let status: String = row.try_get("status").unwrap_or_default();
    let payment_status: String = row.try_get("payment_status").unwrap_or_default();
    assert_eq!("paid", status);
    assert_eq!("success", payment_status);
}

#[tokio::test]
async fn payment_failure_webhook_marks_pending_order_cancelled_but_preserves_terminal() {
    let Some(pool) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    insert_order(
        &pool,
        "order-fail-1",
        "pending_payment",
        "pending",
        None,
        None,
    )
    .await
    .expect("seed pending order");
    insert_order(
        &pool,
        "order-fail-2",
        "paid",
        "success",
        Some("unfulfilled"),
        None,
    )
    .await
    .expect("seed paid order");

    let store = PostgresCommerceOrderStore::new(pool.clone());
    let attempt = sdkwork_order_service::OrderPaymentSettlementAttempt {
        tenant_id: "tenant-1".to_owned(),
        organization_id: Some("0".to_owned()),
        owner_user_id: "user-1".to_owned(),
        order_id: "order-fail-1".to_owned(),
        payment_attempt_id: Some("attempt-1".to_owned()),
        out_trade_no: Some("trade-1".to_owned()),
    };
    let outcome = store
        .mark_owner_order_payment_failed(&attempt, "failed")
        .await
        .expect("pending order must be marked cancelled");
    assert!(!outcome.terminal_order_preserved);
    assert_eq!("cancelled", outcome.order_status);

    let row = sqlx::query(
        "SELECT payment_status, cancelled_at FROM commerce_order WHERE id = 'order-fail-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("reload order");
    let payment_status: String = row.try_get("payment_status").unwrap_or_default();
    let cancelled_at: Option<String> = row.try_get("cancelled_at").ok().flatten();
    assert_eq!("failed", payment_status);
    assert!(cancelled_at
        .as_deref()
        .is_some_and(|value| !value.is_empty()));

    // A late failure callback must never overwrite a confirmed success.
    let terminal_attempt = sdkwork_order_service::OrderPaymentSettlementAttempt {
        order_id: "order-fail-2".to_owned(),
        ..attempt
    };
    let preserved = store
        .mark_owner_order_payment_failed(&terminal_attempt, "closed")
        .await
        .expect("terminal order must be preserved");
    assert!(preserved.terminal_order_preserved);
    assert_eq!("paid", preserved.order_status);
}

#[tokio::test]
async fn expiration_sweep_transitions_due_orders_with_events() {
    let Some(pool) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    insert_order(
        &pool,
        "order-expire-1",
        "pending_payment",
        "pending",
        None,
        Some("1"),
    )
    .await
    .expect("insert expiring order");

    let records = list_due_expiring_orders(&pool, 10)
        .await
        .expect("list due orders");
    let record = records
        .iter()
        .find(|record| record.order_id == "order-expire-1")
        .expect("due order listed");

    let expired = expire_due_order(&pool, record, "2000")
        .await
        .expect("expire order");
    assert!(expired);

    let (status, payment_status): (String, String) = sqlx::query_as(
        "SELECT status, COALESCE(payment_status, '') FROM commerce_order WHERE id = 'order-expire-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("expired order state");
    assert_eq!(status, "expired");
    assert_eq!(payment_status, "expired");
    assert_eq!(event_count(&pool, "order-expire-1", "expired").await, 1);

    // Idempotent replay.
    let replay = expire_due_order(&pool, record, "2000")
        .await
        .expect("expire replay");
    assert!(!replay);
    assert_eq!(event_count(&pool, "order-expire-1", "expired").await, 1);
}

#[tokio::test]
async fn receipt_confirmation_completes_paid_physical_orders() {
    let Some(pool) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    insert_order(
        &pool,
        "order-receipt-1",
        "paid",
        "success",
        Some("awaiting_shipment"),
        None,
    )
    .await
    .expect("insert shipped order");

    let store = PostgresCommerceOrderStore::new(pool.clone());
    store
        .confirm_owner_order_receipt(
            OrderOwnerDetailQuery {
                tenant_id: "tenant-1".to_owned(),
                organization_id: Some("0".to_owned()),
                owner_user_id: "user-1".to_owned(),
                order_id: "order-receipt-1".to_owned(),
            },
            "receipt-request-1",
        )
        .await
        .expect("confirm receipt");

    let (status, fulfillment_status): (String, String) = sqlx::query_as(
        "SELECT status, COALESCE(fulfillment_status, '') FROM commerce_order WHERE id = 'order-receipt-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("completed order state");
    assert_eq!(status, "completed");
    assert_eq!(fulfillment_status, "delivered");
    assert_eq!(event_count(&pool, "order-receipt-1", "completed").await, 1);

    // Idempotent replay.
    store
        .confirm_owner_order_receipt(
            OrderOwnerDetailQuery {
                tenant_id: "tenant-1".to_owned(),
                organization_id: Some("0".to_owned()),
                owner_user_id: "user-1".to_owned(),
                order_id: "order-receipt-1".to_owned(),
            },
            "receipt-request-1",
        )
        .await
        .expect("receipt replay");
    assert_eq!(event_count(&pool, "order-receipt-1", "completed").await, 1);
}

#[tokio::test]
async fn merchant_shipment_advance_marks_orders_shipped() {
    let Some(pool) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    insert_order(
        &pool,
        "order-ship-1",
        "paid",
        "success",
        Some("inventory_reserved"),
        None,
    )
    .await
    .expect("insert paid order");
    sqlx::query(
        r#"
        INSERT INTO commerce_fulfillment_order
            (id, tenant_id, organization_id, fulfillment_no, order_id, fulfillment_type,
             status, provider_code, created_at, updated_at)
        VALUES
            ('physical-fulfillment-order-ship-1', 'tenant-1', '0', 'f-1', 'order-ship-1',
             'physical_shipment', 'awaiting_shipment', 'merchant', '1000', '1000')
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert fulfillment");
    sqlx::query(
        r#"
        INSERT INTO commerce_shipment
            (id, tenant_id, organization_id, shipment_no, fulfillment_id, carrier_code, status, created_at, updated_at)
        VALUES
            ('shipment-1', 'tenant-1', '0', 's-1', 'physical-fulfillment-order-ship-1', 'sf', 'created', '1000', '1000')
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert shipment");
    sqlx::query(
        r#"
        INSERT INTO commerce_shipment_package
            (id, tenant_id, organization_id, shipment_id, package_no, package_type, status, created_at)
        VALUES
            ('package-1', 'tenant-1', '0', 'shipment-1', 'p-1', 'standard', 'created', '1000')
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert package");

    let store = PostgresCommerceOrderStore::new(pool.clone());
    store
        .update_management_shipment_package(UpdateShipmentPackageCommand {
            tenant_id: "tenant-1".to_owned(),
            organization_id: Some("0".to_owned()),
            shipment_id: "shipment-1".to_owned(),
            package_id: "package-1".to_owned(),
            status: Some("shipped".to_owned()),
            package_type: None,
            tracking_no: Some("SF123456".to_owned()),
            request_no: "ship-request-1".to_owned(),
            idempotency_key: "ship-request-1".to_owned(),
        })
        .await
        .expect("mark package shipped");

    let (status, fulfillment_status): (String, String) = sqlx::query_as(
        "SELECT status, COALESCE(fulfillment_status, '') FROM commerce_order WHERE id = 'order-ship-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("shipped order state");
    assert_eq!(status, "shipped");
    assert_eq!(fulfillment_status, "shipped");
    assert_eq!(event_count(&pool, "order-ship-1", "shipped").await, 1);

    let shipment_status: String =
        sqlx::query_scalar("SELECT status FROM commerce_shipment WHERE id = 'shipment-1'")
            .fetch_one(&pool)
            .await
            .expect("shipment state");
    assert_eq!(shipment_status, "shipped");
}

#[tokio::test]
async fn refund_request_rejects_unpaid_orders() {
    let Some(pool) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    insert_order(
        &pool,
        "order-refund-1",
        "pending_payment",
        "pending",
        None,
        None,
    )
    .await
    .expect("insert unpaid order");

    let store = PostgresCommerceRechargeStore::new(pool.clone());
    let outcome = store
        .create_order_refund_request(
            sdkwork_order_service::CreateOrderRefundRequestCommand::new(
                "tenant-1",
                Some("0"),
                "user-1",
                "refund-request-1",
                "order-refund-1",
                sdkwork_order_service::AccountValueAssetCode::Points,
                sdkwork_contract_service::CommerceMoney::new("100").expect("money"),
                "CNY",
                "idem-1",
            )
            .expect("command"),
        )
        .await;
    assert!(
        outcome.is_err(),
        "refund request for an unpaid order must be rejected"
    );
    assert_eq!(outcome.err().map(|error| error.code()), Some("conflict"));
}
