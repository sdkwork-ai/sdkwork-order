//! DB-backed app-template purchase tests: catalog resolution, free-listing
//! settlement, paid-listing idempotency, entitlement reuse, and the
//! install-counter fulfillment. Skipped when
//! `SDKWORK_DATABASE_TEST_POSTGRES_URL` is not configured.

use sdkwork_database_config::DatabaseConfig;
use sdkwork_database_sqlx::{DatabasePool, PoolContext};
use sdkwork_order_integration_deployments::StoreAppTemplateFulfillmentAdapter;
use sdkwork_order_repository_sqlx::{
    order_points_recharge_e2e_postgres_pool_from_env, PostgresCommerceAppTemplateOrderStore,
};
use sdkwork_order_service::{
    AppTemplateOrderListQuery, AppTemplatePurchaseFulfillmentPort, CreateAppTemplateOrderCommand,
    FulfillPaidAppTemplateOrderRequest,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

const TENANT: &str = "100001";
const ORGANIZATION: &str = "0";
/// IAM user subjects are decimal snowflake ids, like every other buyer id the
/// order center stores.
const BUYER: &str = "900001";
const AUTHOR: i64 = 42;

async fn fixture() -> Option<PgPool> {
    order_points_recharge_e2e_postgres_pool_from_env().await
}

/// Derives a stable positive BIGINT primary key from a UUID. Tests run in
/// parallel against one schema, so `MAX(id) + 1` seeding would race; a
/// 60-bit slice of the identifier cannot collide in practice.
fn id_from_uuid(value: &str) -> i64 {
    let hex: String = value
        .chars()
        .filter(|character| *character != '-')
        .collect();
    i64::from_str_radix(&hex[..15], 16).expect("uuid hex prefix")
}

async fn seed_template(
    pool: &PgPool,
    template_uuid: &str,
    pricing_model: &str,
    price_minor: i64,
    status: &str,
    visibility: &str,
    author_user_id: i64,
) -> i64 {
    sqlx::query(
        "INSERT INTO deploy_app_template_category (id, uuid, tenant_id, organization_id, category_key, display_name)
         VALUES (1, 'category-1', CAST($1 AS BIGINT), 0, 'general', 'General')
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(TENANT)
    .execute(pool)
    .await
    .expect("seed template category");

    let template_id = id_from_uuid(template_uuid);

    sqlx::query(
        r#"
        INSERT INTO deploy_app_template
            (id, uuid, tenant_id, organization_id, category_id, author_user_id, app_uuid,
             template_key, display_name, summary, visibility, pricing_model, price_minor,
             currency, status)
        VALUES
            ($1, $2, CAST($3 AS BIGINT), $4, 1, $5, $6, $7, 'Delivery console',
             'Ship an app from a template', $8, $9, $10, 'CNY', $11)
        "#,
    )
    .bind(template_id)
    .bind(template_uuid)
    .bind(TENANT)
    .bind(ORGANIZATION.parse::<i64>().expect("organization id"))
    .bind(author_user_id)
    .bind(Uuid::new_v4().to_string())
    .bind(format!("template-{}", &template_uuid[..8]))
    .bind(visibility)
    .bind(pricing_model)
    .bind(price_minor)
    .bind(status)
    .execute(pool)
    .await
    .expect("seed template");

    template_id
}

async fn seed_version(pool: &PgPool, template_id: i64, tenant: &str) -> String {
    let version_uuid = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO deploy_app_template_version
             (id, uuid, tenant_id, organization_id, template_id, template_version, status)
         VALUES ($1, $2, CAST($3 AS BIGINT), 0, $4, '1.0.0', 'PUBLISHED')",
    )
    .bind(id_from_uuid(&version_uuid))
    .bind(&version_uuid)
    .bind(tenant)
    .bind(template_id)
    .execute(pool)
    .await
    .expect("seed template version");
    version_uuid
}

fn command(template_uuid: &str, idempotency_key: &str) -> CreateAppTemplateOrderCommand {
    command_for_buyer(template_uuid, idempotency_key, BUYER)
}

fn command_for_buyer(
    template_uuid: &str,
    idempotency_key: &str,
    buyer_user_id: &str,
) -> CreateAppTemplateOrderCommand {
    let order_id = Uuid::new_v4().to_string();
    CreateAppTemplateOrderCommand::new(
        TENANT,
        Some(ORGANIZATION),
        buyer_user_id,
        template_uuid,
        "wechat_pay",
        "wechat_native",
        &order_id,
        &Uuid::new_v4().to_string(),
        &format!("AT{}", &order_id[..8]),
        &format!("APPTEMPLATE{}", &order_id[..8]),
        "2026-10-03T00:00:00Z",
        "2026-10-03T00:30:00Z",
        idempotency_key,
        None,
        None,
    )
    .expect("app template order command")
}

async fn order_row(pool: &PgPool, order_id: &str) -> (String, String, String) {
    let row = sqlx::query(
        "SELECT status, COALESCE(payment_status, '') AS payment_status,
                COALESCE(fulfillment_status, '') AS fulfillment_status
           FROM commerce_order WHERE id = $1",
    )
    .bind(order_id)
    .fetch_one(pool)
    .await
    .expect("order row");
    (
        row.get::<String, _>("status"),
        row.get::<String, _>("payment_status"),
        row.get::<String, _>("fulfillment_status"),
    )
}

async fn install_count(pool: &PgPool, template_uuid: &str) -> i64 {
    sqlx::query_scalar("SELECT install_count FROM deploy_app_template WHERE uuid = $1")
        .bind(template_uuid)
        .fetch_one(pool)
        .await
        .expect("install count")
}

/// Counts this buyer's orders for one listing. Scoped to the listing because the
/// test schema is shared across tests and runs.
async fn app_template_order_count(pool: &PgPool, buyer_user_id: &str, template_uuid: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM commerce_order o
           JOIN commerce_order_item oi
             ON oi.tenant_id = o.tenant_id AND oi.order_id = o.id
          WHERE o.tenant_id = $1
            AND o.owner_user_id = $2
            AND o.subject = 'app_template'
            AND COALESCE(
                  NULLIF(COALESCE(NULLIF(to_jsonb(oi) ->> 'sku_snapshot_json', ''), '{}')::jsonb ->> 'templateUuid', ''),
                  NULLIF(to_jsonb(oi) ->> 'sku_id', '')
                ) = $3",
    )
    .bind(TENANT)
    .bind(buyer_user_id)
    .bind(template_uuid)
    .fetch_one(pool)
    .await
    .expect("order count")
}

async fn mark_paid(pool: &PgPool, order_id: &str) {
    sqlx::query(
        "UPDATE commerce_order
            SET status = 'paid', payment_status = 'success', paid_at = NOW(), updated_at = NOW()
          WHERE id = $1",
    )
    .bind(order_id)
    .execute(pool)
    .await
    .expect("mark order paid");
}

fn fulfillment_port(pool: &PgPool) -> StoreAppTemplateFulfillmentAdapter {
    StoreAppTemplateFulfillmentAdapter::new(DatabasePool::Postgres(
        pool.clone(),
        PoolContext {
            config: DatabaseConfig::default(),
        },
    ))
}

fn fulfillment_request(order_id: &str) -> FulfillPaidAppTemplateOrderRequest {
    FulfillPaidAppTemplateOrderRequest {
        tenant_id: TENANT.to_owned(),
        organization_id: Some(ORGANIZATION.to_owned()),
        owner_user_id: BUYER.to_owned(),
        order_id: order_id.to_owned(),
        paid_at: "2026-10-03T00:05:00Z".to_owned(),
        request_no: format!("webhook-{order_id}"),
        idempotency_key: format!("app-template-purchase:fulfill:{order_id}"),
    }
}

#[tokio::test]
async fn free_listing_settles_immediately_and_counts_the_install() {
    let Some(pool) = fixture().await else {
        return;
    };
    let store = PostgresCommerceAppTemplateOrderStore::new(pool.clone());
    let template_uuid = Uuid::new_v4().to_string();
    let template_id = seed_template(
        &pool,
        &template_uuid,
        "FREE",
        0,
        "PUBLISHED",
        "PUBLIC",
        AUTHOR,
    )
    .await;
    let version_uuid = seed_version(&pool, template_id, TENANT).await;

    let outcome = store
        .create_app_template_order(command(&template_uuid, &Uuid::new_v4().to_string()))
        .await
        .expect("free listing purchase");

    assert_eq!(outcome.status, "paid");
    assert_eq!(outcome.amount.as_str(), "0");
    assert_eq!(outcome.template_uuid, template_uuid);
    assert_eq!(outcome.version_uuid.as_deref(), Some(version_uuid.as_str()));
    assert!(!outcome.reused);
    assert!(!outcome.cashier_url.is_empty());

    let (status, payment_status, fulfillment_status) = order_row(&pool, &outcome.order_id).await;
    assert_eq!(status, "paid");
    assert_eq!(payment_status, "success");
    assert_eq!(fulfillment_status, "fulfilled");
    assert_eq!(install_count(&pool, &template_uuid).await, 1);

    let page = store
        .list_app_template_orders(
            &AppTemplateOrderListQuery::new(TENANT, Some(ORGANIZATION), BUYER, None, None)
                .expect("list query"),
        )
        .await
        .expect("list orders");
    let item = page
        .items
        .iter()
        .find(|item| item.template_uuid == template_uuid)
        .expect("purchased template is listed");
    assert!(item.is_entitled());
    assert_eq!(item.status, "paid");
    assert_eq!(item.fulfillment_status, "fulfilled");
    assert_eq!(item.version_uuid.as_deref(), Some(version_uuid.as_str()));
}

#[tokio::test]
async fn paid_listing_replays_idempotently_and_reuses_one_open_order() {
    let Some(pool) = fixture().await else {
        return;
    };
    let store = PostgresCommerceAppTemplateOrderStore::new(pool.clone());
    let template_uuid = Uuid::new_v4().to_string();
    let template_id = seed_template(
        &pool,
        &template_uuid,
        "PAID",
        9900,
        "PUBLISHED",
        "PUBLIC",
        AUTHOR,
    )
    .await;
    seed_version(&pool, template_id, TENANT).await;

    let idempotency_key = Uuid::new_v4().to_string();
    let first = store
        .create_app_template_order(command(&template_uuid, &idempotency_key))
        .await
        .expect("paid listing purchase");
    assert_eq!(first.status, "pending_payment");
    assert_eq!(first.amount.as_str(), "9900");
    assert!(!first.reused);

    // Same key: the stored order is replayed.
    let replay = store
        .create_app_template_order(command(&template_uuid, &idempotency_key))
        .await
        .expect("idempotency replay");
    assert_eq!(replay.order_id, first.order_id);
    assert!(replay.reused);

    // New key while the order is still unpaid: the open purchase intent is
    // reused instead of opening a second order.
    let intent_reuse = store
        .create_app_template_order(command(&template_uuid, &Uuid::new_v4().to_string()))
        .await
        .expect("open intent reuse");
    assert_eq!(intent_reuse.order_id, first.order_id);
    assert!(intent_reuse.reused);
    assert_eq!(
        app_template_order_count(&pool, BUYER, &template_uuid).await,
        1
    );

    // Once settled, a further purchase answers with the settled order: the
    // buyer already owns the listing.
    mark_paid(&pool, &first.order_id).await;
    let settled = store
        .create_app_template_order(command(&template_uuid, &Uuid::new_v4().to_string()))
        .await
        .expect("settled order reuse");
    assert_eq!(settled.order_id, first.order_id);
    assert!(settled.reused);
    assert_eq!(settled.status, "paid");
    assert_eq!(
        app_template_order_count(&pool, BUYER, &template_uuid).await,
        1
    );
}

#[tokio::test]
async fn fulfillment_adapter_counts_an_install_exactly_once() {
    let Some(pool) = fixture().await else {
        return;
    };
    let store = PostgresCommerceAppTemplateOrderStore::new(pool.clone());
    let template_uuid = Uuid::new_v4().to_string();
    let template_id = seed_template(
        &pool,
        &template_uuid,
        "PAID",
        9900,
        "PUBLISHED",
        "PUBLIC",
        AUTHOR,
    )
    .await;
    seed_version(&pool, template_id, TENANT).await;

    let outcome = store
        .create_app_template_order(command(&template_uuid, &Uuid::new_v4().to_string()))
        .await
        .expect("paid listing purchase");
    mark_paid(&pool, &outcome.order_id).await;

    let port = fulfillment_port(&pool);
    let first = port
        .fulfill_app_template_purchase(fulfillment_request(&outcome.order_id))
        .await
        .expect("fulfill settled order");
    assert!(first.accepted);
    assert!(!first.replayed);
    assert_eq!(first.fulfillment_status, "fulfilled");
    assert_eq!(first.template_uuid.as_deref(), Some(template_uuid.as_str()));
    assert_eq!(first.install_count, Some(1));

    // Provider redelivery / compensation retry must not count twice.
    let replay = port
        .fulfill_app_template_purchase(fulfillment_request(&outcome.order_id))
        .await
        .expect("fulfillment replay");
    assert!(replay.accepted);
    assert!(replay.replayed);
    assert_eq!(replay.install_count, Some(1));
    assert_eq!(install_count(&pool, &template_uuid).await, 1);

    let (status, _, fulfillment_status) = order_row(&pool, &outcome.order_id).await;
    assert_eq!(status, "paid");
    assert_eq!(fulfillment_status, "fulfilled");
    let item_status: String = sqlx::query_scalar(
        "SELECT COALESCE(fulfillment_status, '') FROM commerce_order_item WHERE order_id = $1",
    )
    .bind(&outcome.order_id)
    .fetch_one(&pool)
    .await
    .expect("order item");
    assert_eq!(item_status, "fulfilled");
}

#[tokio::test]
async fn listing_guards_reject_unpublished_foreign_and_self_authored_templates() {
    let Some(pool) = fixture().await else {
        return;
    };
    let store = PostgresCommerceAppTemplateOrderStore::new(pool.clone());

    let draft_uuid = Uuid::new_v4().to_string();
    seed_template(&pool, &draft_uuid, "FREE", 0, "DRAFT", "PUBLIC", AUTHOR).await;
    let error = store
        .create_app_template_order(command(&draft_uuid, &Uuid::new_v4().to_string()))
        .await
        .expect_err("draft listing is not purchasable");
    assert_eq!(error.code(), "conflict");

    let private_uuid = Uuid::new_v4().to_string();
    seed_template(
        &pool,
        &private_uuid,
        "FREE",
        0,
        "PUBLISHED",
        "PRIVATE",
        AUTHOR,
    )
    .await;
    let error = store
        .create_app_template_order(command(&private_uuid, &Uuid::new_v4().to_string()))
        .await
        .expect_err("private listing is not purchasable");
    assert_eq!(error.code(), "conflict");

    let own_uuid = Uuid::new_v4().to_string();
    seed_template(&pool, &own_uuid, "FREE", 0, "PUBLISHED", "PUBLIC", 7).await;
    let error = store
        .create_app_template_order(command_for_buyer(
            &own_uuid,
            &Uuid::new_v4().to_string(),
            "7",
        ))
        .await
        .expect_err("the author already holds the template");
    assert_eq!(error.code(), "conflict");

    let missing = Uuid::new_v4().to_string();
    let error = store
        .create_app_template_order(command(&missing, &Uuid::new_v4().to_string()))
        .await
        .expect_err("unknown listing");
    assert_eq!(error.code(), "not-found");
}

#[tokio::test]
async fn fulfillment_adapter_refuses_orders_outside_the_app_template_subject() {
    let Some(pool) = fixture().await else {
        return;
    };
    let order_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO commerce_order
             (id, tenant_id, organization_id, owner_user_id, order_no, status, subject,
              currency_code, payment_status, fulfillment_status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $1, 'paid', 'product', 'CNY', 'success', 'unfulfilled', NOW(), NOW())",
    )
    .bind(&order_id)
    .bind(TENANT)
    .bind(ORGANIZATION)
    .bind(BUYER)
    .execute(&pool)
    .await
    .expect("seed foreign-subject order");

    let error = fulfillment_port(&pool)
        .fulfill_app_template_purchase(fulfillment_request(&order_id))
        .await
        .expect_err("non-template orders are refused");
    assert_eq!(error.code(), "invalid-state");
}
