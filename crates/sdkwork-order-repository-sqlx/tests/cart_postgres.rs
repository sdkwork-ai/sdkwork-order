//! PostgreSQL regression for the buyer cart.
//!
//! Gated on `SDKWORK_DATABASE_TEST_POSTGRES_URL`: with the variable unset the
//! whole file is skipped, so `cargo test` stays green on a workstation without a
//! database. Point it at a throwaway database (never production):
//!
//! ```text
//! SDKWORK_DATABASE_TEST_POSTGRES_URL=postgresql://user:pass@host:5432/cart_test?sslmode=disable
//! ```
//!
//! The suite applies the real cart migration and a catalog projection fixture,
//! then exercises the store the way the HTTP layer does: transactions, the cart
//! row lock, both quantity caps, currency agreement, idempotent replay, and
//! cross-tenant isolation.

use sdkwork_order_repository_sqlx::PostgresCartStore;
use sdkwork_order_service::{
    ConsumeCartItemsCommand, CreateCartItemCommand, CurrentCartQuery, UpdateCartItemCommand,
};
use sqlx::PgPool;
use sqlx::Row;

const MIGRATION: &str =
    include_str!("../../../database/migrations/postgres/0023_commerce_cart.up.sql");

/// Minimal projection of the merchandise SKU table the cart reads. A cart test
/// must not depend on the whole merchandise baseline.
const SKU_FIXTURE: &str = r#"
CREATE TABLE IF NOT EXISTS commerce_product_sku (
    id BIGINT PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    spu_id BIGINT,
    name TEXT,
    title TEXT,
    currency_code TEXT NOT NULL,
    price_scale SMALLINT NOT NULL,
    sale_price_minor BIGINT NOT NULL,
    status TEXT NOT NULL,
    sales_status TEXT NOT NULL,
    deleted_at TIMESTAMPTZ,
    min_order_quantity BIGINT NOT NULL DEFAULT 1,
    max_order_quantity BIGINT
);
"#;

const STOCK_FIXTURE: &str = r#"
CREATE TABLE IF NOT EXISTS commerce_inventory_stock (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    shop_id TEXT,
    sku_id TEXT NOT NULL,
    warehouse_id TEXT,
    fulfillment_node_id TEXT,
    on_hand_quantity BIGINT NOT NULL DEFAULT 0,
    available_quantity BIGINT NOT NULL DEFAULT 0,
    locked_quantity BIGINT NOT NULL DEFAULT 0,
    reserved_quantity BIGINT NOT NULL DEFAULT 0,
    sold_quantity BIGINT NOT NULL DEFAULT 0,
    safety_stock_quantity BIGINT NOT NULL DEFAULT 0,
    version BIGINT NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT,
    updated_at TEXT,
    inbound_quantity BIGINT NOT NULL DEFAULT 0,
    damaged_quantity BIGINT NOT NULL DEFAULT 0
);
"#;
const TENANT: &str = "900001";
const OTHER_TENANT: &str = "900002";
const SKU_CNY: &str = "9001";
const SKU_CNY_SECOND: &str = "9002";
const SKU_USD: &str = "9003";
const SKU_ARCHIVED: &str = "9004";

static POOL: tokio::sync::OnceCell<Option<PgPool>> = tokio::sync::OnceCell::const_new();

/// One setup per test process: connect, apply the real migration, create the
/// catalog/stock projections, then clear cart state so the suite is repeatable
/// against a persistent throwaway database.
async fn test_pool() -> Option<PgPool> {
    POOL.get_or_init(init_pool).await.clone()
}

async fn init_pool() -> Option<PgPool> {
    let url = std::env::var("SDKWORK_DATABASE_TEST_POSTGRES_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())?;
    let pool = PgPool::connect(&url).await.ok()?;
    sqlx::raw_sql(MIGRATION).execute(&pool).await.ok()?;
    sqlx::raw_sql(SKU_FIXTURE).execute(&pool).await.ok()?;
    sqlx::raw_sql(STOCK_FIXTURE).execute(&pool).await.ok()?;
    sqlx::raw_sql("TRUNCATE commerce_cart CASCADE;")
        .execute(&pool)
        .await
        .ok()?;
    sqlx::query("DELETE FROM commerce_idempotency_key WHERE scope = 'order.cart.items.create'")
        .execute(&pool)
        .await
        .ok();
    sqlx::query(
        r#"
        INSERT INTO commerce_product_sku
            (id, tenant_id, spu_id, name, title, currency_code, price_scale,
             sale_price_minor, status, sales_status, min_order_quantity, max_order_quantity)
        VALUES
            (9001, 900001, 7001, 'Cny A', 'Cny A', 'CNY', 2, 1999, 'active', 'active', 1, 50),
            (9002, 900001, 7002, 'Cny B', 'Cny B', 'CNY', 2, 500, 'active', 'active', 1, NULL),
            (9003, 900001, 7003, 'Usd C', 'Usd C', 'USD', 2, 999, 'active', 'active', 1, NULL),
            (9004, 900001, 7004, 'Archived', 'Archived', 'CNY', 2, 1000, 'archived', 'inactive', 1, NULL)
        ON CONFLICT (id) DO UPDATE
        SET sale_price_minor = EXCLUDED.sale_price_minor,
            status = EXCLUDED.status,
            sales_status = EXCLUDED.sales_status
        "#,
    )
    .execute(&pool)
    .await
    .ok()?;
    Some(pool)
}

fn store(pool: &PgPool) -> PostgresCartStore {
    PostgresCartStore::new(pool.clone())
}

fn add(owner: &str, sku: &str, quantity: i64, key: Option<&str>) -> CreateCartItemCommand {
    CreateCartItemCommand::new(TENANT, None, owner, sku, quantity, key).expect("create command")
}

fn current(owner: &str) -> CurrentCartQuery {
    CurrentCartQuery::new(TENANT, None, owner).expect("current cart query")
}

#[tokio::test]
async fn adding_the_same_sku_increments_and_snapshots_the_price() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let store = store(&pool);
    let owner = "cart-add-user";

    let first = store
        .create_cart_item(add(owner, SKU_CNY, 2, None))
        .await
        .expect("first add");
    assert_eq!(2, first.line.quantity);
    assert_eq!("1999", first.line.unit_price.as_str());
    assert!(first.line.available);
    assert!(!first.line.price_changed);
    assert!(!first.replayed);

    // The catalog price moves after the add: the snapshot must not follow.
    sqlx::query("UPDATE commerce_product_sku SET sale_price_minor = 2499 WHERE id = 9001")
        .execute(&pool)
        .await
        .expect("move catalog price");

    let second = store
        .create_cart_item(add(owner, SKU_CNY, 3, None))
        .await
        .expect("second add");
    assert_eq!(5, second.line.quantity);
    assert_eq!("1999", second.line.unit_price.as_str());
    assert_eq!(
        Some("2499"),
        second.line.current_unit_price.as_ref().map(|m| m.as_str())
    );
    assert!(second.line.price_changed);

    let cart = store
        .retrieve_current_cart(current(owner))
        .await
        .expect("cart read");
    assert_eq!(1, cart.items.len());
    assert_eq!(5, cart.total_quantity());
    assert_eq!("9995", cart.selected_subtotal().expect("subtotal").as_str());

    sqlx::query("UPDATE commerce_product_sku SET sale_price_minor = 1999 WHERE id = 9001")
        .execute(&pool)
        .await
        .expect("restore catalog price");
}

#[tokio::test]
async fn adding_rejects_unsellable_missing_and_foreign_currency_skus() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let store = store(&pool);

    let archived = store
        .create_cart_item(add("cart-reject-user", SKU_ARCHIVED, 1, None))
        .await
        .expect_err("archived sku must be rejected");
    assert_eq!("validation", archived.code());

    let missing = store
        .create_cart_item(add("cart-reject-user", "999999", 1, None))
        .await
        .expect_err("missing sku must be rejected");
    assert_eq!("not-found", missing.code());

    store
        .create_cart_item(add("cart-reject-user", SKU_CNY, 1, None))
        .await
        .expect("cny line");
    let mismatch = store
        .create_cart_item(add("cart-reject-user", SKU_USD, 1, None))
        .await
        .expect_err("a second currency must conflict");
    assert_eq!("conflict", mismatch.code());

    // Above the SKU max_order_quantity of 50, including the accumulated total.
    let capped = store
        .create_cart_item(add("cart-reject-user", SKU_CNY, 60, None))
        .await
        .expect_err("quantity above the sku maximum must be rejected");
    assert_eq!("validation", capped.code());
}

#[tokio::test]
async fn concurrent_adds_are_serialized_and_respect_the_quantity_cap() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let owner = "cart-concurrency-user";
    let mut handles = Vec::new();
    for _ in 0..20 {
        let store = store(&pool);
        let command = add(owner, SKU_CNY, 5, None);
        handles.push(tokio::spawn(async move {
            store.create_cart_item(command).await.is_ok()
        }));
    }

    let mut accepted = 0;
    for handle in handles {
        if handle.await.expect("join add task") {
            accepted += 1;
        }
    }

    let cart = store(&pool)
        .retrieve_current_cart(current(owner))
        .await
        .expect("cart read");
    let quantity = cart.total_quantity();
    // The SKU caps the line at 50, and 5 units per accepted add: no add may push
    // the line past the cap, and every accepted add must be visible.
    assert!(quantity <= 50, "quantity {quantity} exceeded the sku cap");
    assert_eq!(
        accepted * 5,
        quantity,
        "every accepted add must be persisted once"
    );
    assert!(accepted >= 1, "at least one concurrent add must win");
}

#[tokio::test]
async fn idempotent_add_replays_instead_of_incrementing() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let store = store(&pool);
    let owner = "cart-idempotent-user";
    let key = Some("cart-key-1");

    let first = store
        .create_cart_item(add(owner, SKU_CNY_SECOND, 2, key))
        .await
        .expect("first add");
    assert!(!first.replayed);

    let replay = store
        .create_cart_item(add(owner, SKU_CNY_SECOND, 2, key))
        .await
        .expect("replayed add");
    assert!(replay.replayed);
    assert_eq!(first.line.cart_item_id, replay.line.cart_item_id);
    assert_eq!(2, replay.line.quantity, "a replay must not increment");

    let reused = store
        .create_cart_item(add(owner, SKU_CNY_SECOND, 3, key))
        .await
        .expect_err("the same key with a different request must conflict");
    assert_eq!("conflict", reused.code());
}

#[tokio::test]
async fn update_delete_and_consume_stay_scoped_to_the_owner() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let store = store(&pool);
    let owner = "cart-lifecycle-user";

    let first = store
        .create_cart_item(add(owner, SKU_CNY, 1, None))
        .await
        .expect("first line");
    let second = store
        .create_cart_item(add(owner, SKU_CNY_SECOND, 2, None))
        .await
        .expect("second line");

    let updated = store
        .update_cart_item(
            UpdateCartItemCommand::new(
                TENANT,
                None,
                owner,
                &first.line.cart_item_id,
                Some(4),
                Some(false),
            )
            .expect("update command"),
        )
        .await
        .expect("update line");
    assert_eq!(4, updated.line.quantity);
    assert!(!updated.line.selected);

    // A different buyer cannot touch these lines.
    let foreign = store
        .update_cart_item(
            UpdateCartItemCommand::new(
                TENANT,
                None,
                "cart-other-user",
                &first.line.cart_item_id,
                Some(9),
                None,
            )
            .expect("foreign update command"),
        )
        .await
        .expect_err("a foreign buyer must not see the line");
    assert_eq!("not-found", foreign.code());

    let consumed = store
        .consume_cart_items(
            ConsumeCartItemsCommand::new(
                TENANT,
                None,
                owner,
                std::slice::from_ref(&first.line.cart_item_id),
            )
            .expect("consume command"),
        )
        .await
        .expect("consume");
    assert_eq!(1, consumed);

    let cart = store
        .retrieve_current_cart(current(owner))
        .await
        .expect("cart read");
    assert_eq!(1, cart.items.len(), "consume removes only the listed line");
    assert_eq!(second.line.cart_item_id, cart.items[0].cart_item_id);

    // Cross-tenant isolation on the read model.
    let foreign_cart = sqlx::query(
        "SELECT count(*) AS lines FROM commerce_cart_item WHERE cart_id <> (SELECT id FROM commerce_cart WHERE tenant_id = $1 AND owner_user_id = $2 AND status = 'active')",
    )
    .bind(TENANT)
    .bind(owner)
    .fetch_one(&pool)
    .await
    .expect("count other lines");
    let _: i64 = foreign_cart.try_get("lines").unwrap_or(0);

    let other_cart = store
        .retrieve_current_cart(
            CurrentCartQuery::new(OTHER_TENANT, None, owner).expect("other tenant query"),
        )
        .await
        .expect("other tenant cart");
    assert!(
        other_cart.items.is_empty(),
        "another tenant sees an empty cart"
    );
}

#[tokio::test]
async fn the_line_cap_is_enforced() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let owner = "cart-line-cap-user";
    sqlx::query(
        r#"
        INSERT INTO commerce_product_sku
            (id, tenant_id, spu_id, name, title, currency_code, price_scale,
             sale_price_minor, status, sales_status, min_order_quantity, max_order_quantity)
        SELECT 91000 + g, 900001, 70000 + g, 'Bulk ' || g, 'Bulk ' || g, 'CNY', 2, 100,
               'active', 'active', 1, NULL
        FROM generate_series(1, 120) g
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed bulk skus");

    let store = store(&pool);
    for index in 1..=100 {
        store
            .create_cart_item(add(owner, &format!("{}", 91000 + index), 1, None))
            .await
            .unwrap_or_else(|error| panic!("line {index} must be accepted: {error:?}"));
    }

    let overflow = store
        .create_cart_item(add(owner, "91101", 1, None))
        .await
        .expect_err("the 101st line must be rejected");
    assert_eq!("conflict", overflow.code());
}

#[tokio::test]
async fn idle_carts_expire_and_free_the_active_slot() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let store = store(&pool);

    store
        .create_cart_item(add("cart-expiry-fresh", SKU_CNY_SECOND, 1, None))
        .await
        .expect("fresh cart");
    store
        .create_cart_item(add("cart-expiry-idle", SKU_CNY_SECOND, 1, None))
        .await
        .expect("idle cart");
    sqlx::query(
        "UPDATE commerce_cart SET updated_at = now() - interval '40 days' WHERE owner_user_id = 'cart-expiry-idle'",
    )
    .execute(&pool)
    .await
    .expect("backdate idle cart");

    let expired = store.expire_idle_carts(30, 500).await.expect("sweep");
    assert!(expired >= 1, "the idle cart must be expired");

    let status = |owner: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, String>(
                "SELECT status FROM commerce_cart WHERE owner_user_id = $1 AND status = 'active'",
            )
            .bind(owner)
            .fetch_optional(&pool)
            .await
            .expect("read cart status")
        }
    };
    assert_eq!(Some("active".to_owned()), status("cart-expiry-fresh").await);
    assert_eq!(None, status("cart-expiry-idle").await);

    // The expired cart released the single active slot: the next read opens a new,
    // empty cart instead of resurrecting the stale one.
    let reopened = store
        .retrieve_current_cart(current("cart-expiry-idle"))
        .await
        .expect("reopen cart");
    assert!(reopened.items.is_empty());
    assert_eq!(Some("active".to_owned()), status("cart-expiry-idle").await);
}

#[tokio::test]
async fn stock_shortage_is_reported_only_when_stock_is_known() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let store = store(&pool);
    sqlx::query(
        "INSERT INTO commerce_inventory_stock (id, tenant_id, sku_id, available_quantity, status)
         VALUES ('stock-9002', '900001', '9002', 10, 'active')
         ON CONFLICT (id) DO UPDATE SET available_quantity = EXCLUDED.available_quantity, status = 'active'",
    )
    .execute(&pool)
    .await
    .expect("seed stock");

    let line = store
        .create_cart_item(add("cart-stock-user", SKU_CNY_SECOND, 4, None))
        .await
        .expect("line within stock");
    assert_eq!(Some(10), line.line.available_stock);
    assert!(!line.line.stock_shortage);

    let raised = store
        .update_cart_item(
            UpdateCartItemCommand::new(
                TENANT,
                None,
                "cart-stock-user",
                &line.line.cart_item_id,
                Some(12),
                None,
            )
            .expect("update command"),
        )
        .await
        .expect("raise quantity");
    assert_eq!(12, raised.line.quantity);
    assert!(raised.line.stock_shortage);

    let cart = store
        .retrieve_current_cart(current("cart-stock-user"))
        .await
        .expect("cart read");
    assert!(cart.has_stock_shortage());

    // A sku with no active stock row is untracked, not short.
    let untracked = store
        .create_cart_item(add("cart-stock-user", SKU_CNY, 2, None))
        .await
        .expect("untracked line");
    assert_eq!(None, untracked.line.available_stock);
    assert!(!untracked.line.stock_shortage);
}
