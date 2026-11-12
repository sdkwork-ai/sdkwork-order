//! Order repricing after a promotion discount, against the order PostgreSQL
//! schema. Skipped unless `SDKWORK_DATABASE_TEST_POSTGRES_URL` is configured.

use sdkwork_order_repository_sqlx::{
    order_points_recharge_e2e_postgres_pool_from_env, PostgresCommerceOrderStore,
};
use sdkwork_order_service::ApplyOrderDiscountCommand;
use uuid::Uuid;

const TENANT: &str = "100001";
const ORGANIZATION: &str = "0";
const OWNER: &str = "900123";

#[tokio::test]
async fn applying_discount_reprices_the_order_total_breakdown() {
    let Ok(url) = std::env::var("SDKWORK_DATABASE_TEST_POSTGRES_URL") else {
        return;
    };
    // The harness only runs `CREATE TABLE IF NOT EXISTS`; a previous minimal
    // `commerce_order` shape would leave it without the e2e columns. Rebuild the
    // two tables this test owns on the disposable test database.
    let bootstrap = sqlx::PgPool::connect(&url).await.expect("connect test db");
    sqlx::query(
        "DROP TABLE IF EXISTS commerce_order_amount_breakdown, commerce_order CASCADE",
    )
    .execute(&bootstrap)
    .await
    .expect("reset order tables");
    drop(bootstrap);

    let Some(pool) = order_points_recharge_e2e_postgres_pool_from_env().await else {
        return;
    };
    let order_id = format!("discount-order-{}", Uuid::new_v4());
    let breakdown_id = format!("discount-breakdown-{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO commerce_order
            (id, tenant_id, organization_id, owner_user_id, order_no, status, subject,
             currency_code, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, 'pending_payment', 'membership', 'CNY',
                NOW(), NOW())
        "#,
    )
    .bind(&order_id)
    .bind(TENANT)
    .bind(ORGANIZATION)
    .bind(OWNER)
    .bind(format!("NO-{order_id}"))
    .execute(&pool)
    .await
    .expect("seed order");
    sqlx::query(
        r#"
        INSERT INTO commerce_order_amount_breakdown
            (id, tenant_id, organization_id, order_id, allocation_type, original_amount,
             discount_amount, payable_amount, currency_code, created_at)
        VALUES ($1, $2, $3, $4, 'order_total', '10000', '0', '10000', 'CNY', NOW())
        "#,
    )
    .bind(&breakdown_id)
    .bind(TENANT)
    .bind(ORGANIZATION)
    .bind(&order_id)
    .execute(&pool)
    .await
    .expect("seed breakdown");

    let store = PostgresCommerceOrderStore::new(pool.clone());
    let first = store
        .apply_order_discount(
            ApplyOrderDiscountCommand::new(TENANT, Some(ORGANIZATION), OWNER, &order_id, 3000)
                .expect("first discount command"),
        )
        .await
        .expect("apply first discount");
    assert_eq!(first.original_amount_minor, "10000");
    assert_eq!(first.discount_amount_minor, "3000");
    assert_eq!(first.payable_amount_minor, "7000");

    let second = store
        .apply_order_discount(
            ApplyOrderDiscountCommand::new(TENANT, Some(ORGANIZATION), OWNER, &order_id, 500)
                .expect("second discount command"),
        )
        .await
        .expect("apply second discount");
    assert_eq!(
        second.discount_amount_minor, "3500",
        "combinable coupons stack on the same breakdown"
    );
    assert_eq!(second.payable_amount_minor, "6500");

    let over = store
        .apply_order_discount(
            ApplyOrderDiscountCommand::new(TENANT, Some(ORGANIZATION), OWNER, &order_id, 10000)
                .expect("over discount command"),
        )
        .await
        .expect_err("discount must not exceed the order amount");
    assert!(over.message().contains("exceeds the order amount"));

    sqlx::query("UPDATE commerce_order SET status = 'paid' WHERE id = $1")
        .bind(&order_id)
        .execute(&pool)
        .await
        .expect("mark order paid");
    let paid = store
        .apply_order_discount(
            ApplyOrderDiscountCommand::new(TENANT, Some(ORGANIZATION), OWNER, &order_id, 1)
                .expect("paid discount command"),
        )
        .await
        .expect_err("a paid order must not be repriced");
    assert!(paid.message().contains("not open for a discount"));
}
