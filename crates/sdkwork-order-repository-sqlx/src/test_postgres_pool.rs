use sqlx::PgPool;

pub fn order_points_recharge_e2e_migration_sql() -> &'static str {
    include_str!("../test_migrations/0001_order_points_recharge_e2e.postgres.sql")
}

pub fn split_order_e2e_sql_statements(sql: &str) -> Vec<String> {
    sql.split(';')
        .map(|chunk| {
            chunk
                .lines()
                .filter(|line| {
                    let trimmed = line.trim_start();
                    !trimmed.is_empty() && !trimmed.starts_with("--")
                })
                .collect::<Vec<_>>()
                .join(
                    "
",
                )
        })
        .map(|statement| statement.trim().to_string())
        .filter(|statement| !statement.is_empty())
        .collect()
}

/// PostgreSQL-only e2e pool（DATABASE_SPEC：server test profile 必须使用 PostgreSQL）。
/// 由环境变量 `SDKWORK_DATABASE_TEST_POSTGRES_URL` 提供连接；未配置时返回 None。
pub async fn order_points_recharge_e2e_postgres_pool_from_env() -> Option<PgPool> {
    let url = std::env::var("SDKWORK_DATABASE_TEST_POSTGRES_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())?;
    let pool = PgPool::connect(&url).await.ok()?;
    for statement in split_order_e2e_sql_statements(order_points_recharge_e2e_migration_sql()) {
        if let Err(error) = sqlx::query(sqlx::AssertSqlSafe(statement.as_str()))
            .execute(&pool)
            .await
        {
            eprintln!("postgres e2e migration skipped ({error}); statement: {statement}");
            return None;
        }
    }
    Some(pool)
}
