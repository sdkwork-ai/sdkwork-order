use super::*;
use sdkwork_database_config::DatabaseConfig;
use sdkwork_database_sqlx::{DatabasePool, PoolContext};
use sdkwork_order_service::{
    PhysicalInventoryLine, ReleasePhysicalOrderInventoryRequest,
    ReservePhysicalOrderInventoryRequest,
};
use sqlx::postgres::PgPoolOptions;

/// 服务端测试必须使用 PostgreSQL（DATABASE_SPEC：authoritative-server）。
/// 由 `SDKWORK_DATABASE_TEST_POSTGRES_URL` 提供连接；未配置时跳过。
fn postgres_url() -> Option<String> {
    std::env::var("SDKWORK_DATABASE_TEST_POSTGRES_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// 共享 PostgreSQL 实例下，串行化 DROP/CREATE schema 的初始化阶段。
static SCHEMA_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

async fn fixture() -> Option<(sqlx::PgPool, PhysicalInventoryAdapter)> {
    let url = postgres_url()?;
    let _guard = SCHEMA_LOCK.lock().expect("schema lock");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .ok()?;
    sqlx::query(
        r#"
        DROP TABLE IF EXISTS commerce_inventory_reservation;
        DROP TABLE IF EXISTS commerce_inventory_stock;
        CREATE TABLE commerce_inventory_stock (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            organization_id TEXT NOT NULL DEFAULT '0',
            shop_id TEXT,
            sku_id TEXT NOT NULL,
            warehouse_id TEXT,
            fulfillment_node_id TEXT,
            available_quantity INTEGER NOT NULL,
            reserved_quantity INTEGER NOT NULL,
            sold_quantity INTEGER NOT NULL,
            safety_stock_quantity INTEGER NOT NULL,
            version INTEGER NOT NULL,
            status TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE commerce_inventory_reservation (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            organization_id TEXT NOT NULL DEFAULT '0',
            reservation_no TEXT NOT NULL,
            order_id TEXT NOT NULL,
            reservation_source_type TEXT NOT NULL,
            reservation_source_id TEXT NOT NULL,
            reservation_type TEXT NOT NULL,
            sku_id TEXT NOT NULL,
            warehouse_id TEXT,
            fulfillment_node_id TEXT,
            quantity INTEGER NOT NULL,
            reserved_quantity INTEGER NOT NULL,
            consumed_quantity INTEGER NOT NULL,
            released_quantity INTEGER NOT NULL,
            status TEXT NOT NULL,
            release_reason_code TEXT,
            request_no TEXT NOT NULL,
            idempotency_key TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            consumed_at TEXT,
            released_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        INSERT INTO commerce_inventory_stock (
            id, tenant_id, organization_id, shop_id, sku_id, warehouse_id,
            fulfillment_node_id, available_quantity, reserved_quantity,
            sold_quantity, safety_stock_quantity, version, status, updated_at
        ) VALUES ('stock-1', 'tenant-1', 'merchant-1', 'shop-1', 'sku-1', 'wh-1',
                  'node-1', 10, 0, 0, 0, 1, 'active', 'now');
        "#,
    )
    .execute(&pool)
    .await
    .ok()?;
    drop(_guard);

    let context = PoolContext {
        config: DatabaseConfig::default(),
    };
    let database_pool = DatabasePool::Postgres(pool.clone(), context);
    Some((pool, PhysicalInventoryAdapter::new(database_pool)))
}

fn reserve_request(order_id: &str, key: &str) -> ReservePhysicalOrderInventoryRequest {
    ReservePhysicalOrderInventoryRequest {
        tenant_id: "tenant-1".to_owned(),
        merchant_organization_id: "merchant-1".to_owned(),
        order_id: order_id.to_owned(),
        request_no: format!("request-{order_id}"),
        idempotency_key: key.to_owned(),
        lines: vec![PhysicalInventoryLine {
            sku_id: "sku-1".to_owned(),
            shop_id: "shop-1".to_owned(),
            quantity: 3,
        }],
    }
}

#[tokio::test]
async fn reserve_replay_only_decrements_stock_once_and_rejects_changed_payload() {
    let Some((pool, adapter)) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    let request = reserve_request("order-1", "reserve-key-1");

    let first = adapter
        .reserve_physical_order_inventory(request.clone())
        .await
        .expect("first reservation");
    let replay = adapter
        .reserve_physical_order_inventory(request.clone())
        .await
        .expect("reservation replay");
    assert!(!first.replayed);
    assert!(replay.replayed);

    let changed = ReservePhysicalOrderInventoryRequest {
        lines: vec![PhysicalInventoryLine {
            quantity: 4,
            ..request.lines[0].clone()
        }],
        ..request
    };
    assert!(adapter
        .reserve_physical_order_inventory(changed)
        .await
        .is_err());

    let (available, reserved, count): (i64, i64, i64) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, (SELECT COUNT(*) FROM commerce_inventory_reservation) FROM commerce_inventory_stock WHERE id = 'stock-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("stock state");
    assert_eq!((available, reserved, count), (7, 3, 1));
}

#[tokio::test]
async fn consume_replay_is_idempotent_and_release_skips_consumed_reservations() {
    let Some((pool, adapter)) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    let request = reserve_request("order-2", "reserve-key-2");
    adapter
        .reserve_physical_order_inventory(request)
        .await
        .expect("reservation");

    let database_pool = DatabasePool::Postgres(
        pool.clone(),
        PoolContext {
            config: DatabaseConfig::default(),
        },
    );
    let first = consume_order_inventory(&database_pool, "tenant-1", "order-2", "consume-key-2")
        .await
        .expect("first consume");
    let replay = consume_order_inventory(&database_pool, "tenant-1", "order-2", "consume-key-2")
        .await
        .expect("consume replay");
    assert!(!first.replayed);
    assert!(replay.replayed);

    // A release after consume is a no-op: consumed stock is already out of
    // the warehouse and must not be returned to available stock.
    let release = adapter
        .release_physical_order_inventory(ReleasePhysicalOrderInventoryRequest {
            tenant_id: "tenant-1".to_owned(),
            order_id: "order-2".to_owned(),
            reason_code: "buyer_cancelled".to_owned(),
            request_no: "cancel-order-2".to_owned(),
            idempotency_key: "release-key-2".to_owned(),
        })
        .await
        .expect("release after consume is idempotent");
    assert!(release.replayed);

    let (available, reserved, sold, status): (i64, i64, i64, String) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, sold_quantity, (SELECT status FROM commerce_inventory_reservation WHERE order_id = 'order-2') FROM commerce_inventory_stock WHERE id = 'stock-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("consumed stock state");
    assert_eq!(
        (available, reserved, sold, status),
        (7, 0, 3, "consumed".to_owned())
    );
}

#[tokio::test]
async fn release_replay_restores_stock_only_once() {
    let Some((pool, adapter)) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    let request = reserve_request("order-3", "reserve-key-3");
    adapter
        .reserve_physical_order_inventory(request)
        .await
        .expect("reservation");
    let request = ReleasePhysicalOrderInventoryRequest {
        tenant_id: "tenant-1".to_owned(),
        order_id: "order-3".to_owned(),
        reason_code: "buyer_cancelled".to_owned(),
        request_no: "cancel-order-3".to_owned(),
        idempotency_key: "release-key-3".to_owned(),
    };
    let first = adapter
        .release_physical_order_inventory(request.clone())
        .await
        .expect("first release");
    let replay = adapter
        .release_physical_order_inventory(request)
        .await
        .expect("release replay");
    assert!(!first.replayed);
    assert!(replay.replayed);

    let (available, reserved, version): (i64, i64, i64) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, version FROM commerce_inventory_stock WHERE id = 'stock-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("released stock state");
    assert_eq!((available, reserved, version), (10, 0, 3));
}

#[tokio::test]
async fn restock_returns_consumed_stock_to_available_only_once() {
    let Some((pool, adapter)) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    let request = reserve_request("order-4", "reserve-key-4");
    adapter
        .reserve_physical_order_inventory(request)
        .await
        .expect("reservation");

    let database_pool = DatabasePool::Postgres(
        pool.clone(),
        PoolContext {
            config: DatabaseConfig::default(),
        },
    );
    consume_order_inventory(&database_pool, "tenant-1", "order-4", "consume-key-4")
        .await
        .expect("consume");

    let request = ReleasePhysicalOrderInventoryRequest {
        tenant_id: "tenant-1".to_owned(),
        order_id: "order-4".to_owned(),
        reason_code: "buyer_return".to_owned(),
        request_no: "restock-order-4".to_owned(),
        idempotency_key: "restock-key-4".to_owned(),
    };
    let first = adapter
        .restock_consumed_order_inventory(request.clone())
        .await
        .expect("first restock");
    let replay = adapter
        .restock_consumed_order_inventory(request)
        .await
        .expect("restock replay");
    assert!(!first.replayed);
    assert!(replay.replayed);

    let (available, reserved, sold, status): (i64, i64, i64, String) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, sold_quantity, (SELECT status FROM commerce_inventory_reservation WHERE order_id = 'order-4') FROM commerce_inventory_stock WHERE id = 'stock-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("restocked stock state");
    assert_eq!(
        (available, reserved, sold, status),
        (10, 0, 0, "restocked".to_owned())
    );
}

#[tokio::test]
async fn sweep_releases_expired_reservations_back_to_stock() {
    let Some((pool, adapter)) = fixture().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    let request = reserve_request("order-5", "reserve-key-5");
    adapter
        .reserve_physical_order_inventory(request)
        .await
        .expect("reservation");

    // Age the reservation beyond its expiry window.
    sqlx::query("UPDATE commerce_inventory_reservation SET expires_at = '1' WHERE order_id = 'order-5'")
        .execute(&pool)
        .await
        .expect("age reservation");

    let swept = adapter
        .sweep_expired_inventory_reservations(10)
        .await
        .expect("sweep");
    assert_eq!(swept, 1);

    let (available, reserved, status): (i64, i64, String) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, (SELECT status FROM commerce_inventory_reservation WHERE order_id = 'order-5') FROM commerce_inventory_stock WHERE id = 'stock-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("swept stock state");
    assert_eq!((available, reserved, status), (10, 0, "released".to_owned()));

    // A second sweep is a no-op (idempotent).
    let replay = adapter
        .sweep_expired_inventory_reservations(10)
        .await
        .expect("sweep replay");
    assert_eq!(replay, 0);
}
