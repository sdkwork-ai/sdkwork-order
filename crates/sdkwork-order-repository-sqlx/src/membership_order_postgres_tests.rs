use sdkwork_order_service::CreateMembershipOrderCommand;

use crate::test_postgres_pool::order_points_recharge_e2e_postgres_pool_from_env;
use crate::PostgresCommerceMembershipOrderStore;

#[tokio::test]
async fn postgres_concurrent_membership_purchase_intent_has_one_active_order() {
    let Some(pool) = order_points_recharge_e2e_postgres_pool_from_env().await else {
        eprintln!(
            "SDKWORK_DATABASE_TEST_POSTGRES_URL is unset; skipping postgres membership concurrency test"
        );
        return;
    };
    let owner_user_id = "membership-concurrency-user";
    sqlx::query(
        "DELETE FROM commerce_order WHERE tenant_id = $1 AND organization_id = $2 AND owner_user_id = $3 AND subject = 'membership'",
    )
    .bind("100001")
    .bind("0")
    .bind(owner_user_id)
    .execute(&pool)
    .await
    .expect("clear membership concurrency fixture");

    let mut tasks = Vec::with_capacity(20);
    for index in 0..20 {
        let store = PostgresCommerceMembershipOrderStore::new(pool.clone());
        let command = CreateMembershipOrderCommand::new(
            "100001",
            Some("0"),
            owner_user_id,
            "201",
            "purchase",
            "wechat_pay",
            "wechat_native",
            &format!("membership-postgres-order-{index}"),
            &format!("membership-postgres-item-{index}"),
            &format!("MB-PG-{index}"),
            &format!("MEMBERSHIP-PG-{index}"),
            "2026-07-26T14:00:00Z",
            "2099-07-26T14:30:00Z",
            &format!("membership-postgres-idempotency-{index}"),
            Some(&format!("membership-postgres-client-{index}")),
            Some("postgres-concurrency-test"),
            None,
            None,
            1,
        )
        .expect("membership concurrency command");
        tasks.push(tokio::spawn(async move {
            store.create_membership_order(command).await
        }));
    }

    let mut winner_id = None;
    for task in tasks {
        let outcome = task
            .await
            .expect("membership concurrency task")
            .expect("membership concurrency create");
        match winner_id.as_ref() {
            Some(expected) => assert_eq!(&outcome.order_id, expected),
            None => winner_id = Some(outcome.order_id),
        }
    }

    let active_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM commerce_order
        WHERE tenant_id = $1
          AND organization_id = $2
          AND owner_user_id = $3
          AND subject = 'membership'
          AND status IN ('draft', 'pending', 'pending_payment', 'unpaid', 'wait_pay', 'created')
        "#,
    )
    .bind("100001")
    .bind("0")
    .bind(owner_user_id)
    .fetch_one(&pool)
    .await
    .expect("count active membership purchase intents");
    assert_eq!(active_count, 1);
}
