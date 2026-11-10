//! Optional PostgreSQL evidence for coupon-redemption failure/retry semantics.
//! Skipped when `SDKWORK_DATABASE_TEST_POSTGRES_URL` is not configured.

use sdkwork_contract_service::CommerceMoney;
use sdkwork_order_repository_sqlx::{
    order_points_recharge_e2e_postgres_pool_from_env, PostgresCommerceRechargeStore,
};
use sdkwork_order_service::{
    AccountValueAssetCode, AccountValueFulfillmentContext, AccountValueOrderSubject,
    FulfillAccountValueOrderCommand,
};
use sqlx::PgPool;

const TENANT_ID: &str = "tenant-coupon-retry";
const OWNER_USER_ID: &str = "user-coupon-retry";
const ORDER_ID: &str = "order-coupon-retry-1";

fn coupon_fulfillment_context(fulfillment_status: &str) -> AccountValueFulfillmentContext {
    AccountValueFulfillmentContext {
        order_id: ORDER_ID.to_owned(),
        order_no: "CP-RETRY-1".to_owned(),
        subject: AccountValueOrderSubject::CouponRecharge,
        target_asset: AccountValueAssetCode::TokenBank,
        order_status: "pending_payment".to_owned(),
        fulfillment_status: fulfillment_status.to_owned(),
        payment_status: "success".to_owned(),
        payment_attempt_status: "success".to_owned(),
        grant_amount: CommerceMoney::new("5000").expect("token bank grant amount"),
        asset_unit_code: "TOKEN_BANK".to_owned(),
        coupon_code: Some("COUPON-RETRY".to_owned()),
        coupon_benefit: None,
    }
}

async fn seed_order(pool: &PgPool, fulfillment_status: &str) {
    sqlx::query("DELETE FROM commerce_order WHERE id = $1")
        .bind(ORDER_ID)
        .execute(pool)
        .await
        .expect("clear coupon retry fixture");
    sqlx::query(
        r#"
        INSERT INTO commerce_order
            (id, tenant_id, organization_id, owner_user_id, order_no, status,
             subject, currency_code, payment_status, fulfillment_status,
             request_no, idempotency_key, created_at, updated_at)
        VALUES
            ($1, $2, '0', $3, $1, 'pending_payment',
             'coupon_recharge', 'CNY', 'success', $4,
             'REQ-COUPON-RETRY', 'IDEMP-COUPON-RETRY',
             '1971-01-01T00:00:00.000Z', '1971-01-01T00:00:00.000Z')
        "#,
    )
    .bind(ORDER_ID)
    .bind(TENANT_ID)
    .bind(OWNER_USER_ID)
    .bind(fulfillment_status)
    .execute(pool)
    .await
    .expect("seed coupon retry order");
}

async fn fulfillment_status(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT fulfillment_status FROM commerce_order WHERE id = $1")
        .bind(ORDER_ID)
        .fetch_one(pool)
        .await
        .expect("reload coupon retry order")
}

#[tokio::test]
async fn postgres_reserve_accepts_failed_coupon_fulfillment_and_retries_to_processing() {
    let Some(pool) = order_points_recharge_e2e_postgres_pool_from_env().await else {
        eprintln!(
            "SDKWORK_DATABASE_TEST_POSTGRES_URL is unset; skipping coupon failure retry PG test"
        );
        return;
    };
    let store = PostgresCommerceRechargeStore::new(pool.clone());
    let command = FulfillAccountValueOrderCommand::new(
        TENANT_ID,
        None,
        OWNER_USER_ID,
        ORDER_ID,
        "REQ-COUPON-RETRY",
        "IDEMP-COUPON-RETRY",
    )
    .expect("coupon retry command");

    // A failed fulfilment attempt must stay reservable: the reservation claim
    // only excludes fulfilled|completed|processing.
    seed_order(&pool, "failed").await;
    store
        .reserve_account_value_fulfillment(&command, &coupon_fulfillment_context("failed"))
        .await
        .expect("reserve must accept an order marked failed");
    assert_eq!("processing", fulfillment_status(&pool).await);

    // The same idempotency key must be able to retry after another failure.
    seed_order(&pool, "failed").await;
    store
        .reserve_account_value_fulfillment(&command, &coupon_fulfillment_context("failed"))
        .await
        .expect("a retry must claim the failed order again");
    assert_eq!("processing", fulfillment_status(&pool).await);
}
