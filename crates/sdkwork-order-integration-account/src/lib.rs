mod http_adapter;
mod store_adapter;

pub use http_adapter::HttpAccountPointsCreditAdapter;
pub use store_adapter::StoreAccountPointsCreditAdapter;

use std::sync::Arc;

use sdkwork_database_config::DatabaseConfig;
use sdkwork_database_sqlx::{create_pool_from_config, DatabasePool};
use sdkwork_order_service::{AccountPointsCreditPort, AccountValueLedgerPort};

/// Builds the account points credit port from environment.
///
/// - `SDKWORK_ORDER_ACCOUNT_LEDGER_ADAPTER=store` — in-process ledger via ACCOUNT database pool.
/// - `SDKWORK_ORDER_ACCOUNT_LEDGER_ADAPTER=http` (default) — HTTP POST to account backend adjustments.
pub async fn account_points_credit_port_from_env(
) -> Result<Arc<dyn AccountPointsCreditPort>, String> {
    let mode = std::env::var("SDKWORK_ORDER_ACCOUNT_LEDGER_ADAPTER")
        .unwrap_or_else(|_| "http".to_owned())
        .trim()
        .to_ascii_lowercase();

    match mode.as_str() {
        "store" => build_store_adapter().await,
        "http" => Ok(Arc::new(HttpAccountPointsCreditAdapter::from_env()?)),
        other => Err(format!(
            "unsupported SDKWORK_ORDER_ACCOUNT_LEDGER_ADAPTER value: {other}"
        )),
    }
}

pub async fn account_value_ledger_port_from_env() -> Result<Arc<dyn AccountValueLedgerPort>, String>
{
    let mode = std::env::var("SDKWORK_ORDER_ACCOUNT_LEDGER_ADAPTER")
        .unwrap_or_else(|_| "http".to_owned())
        .trim()
        .to_ascii_lowercase();

    match mode.as_str() {
        "store" => build_store_account_value_adapter().await,
        "http" => Ok(Arc::new(HttpAccountPointsCreditAdapter::from_env()?)),
        other => Err(format!(
            "unsupported SDKWORK_ORDER_ACCOUNT_LEDGER_ADAPTER value: {other}"
        )),
    }
}

async fn build_store_adapter() -> Result<Arc<dyn AccountPointsCreditPort>, String> {
    let _ = dotenvy::dotenv();
    let config = DatabaseConfig::from_env("ACCOUNT")
        .map_err(|error| format!("read account database config failed: {error}"))?;
    let pool = create_pool_from_config(config)
        .await
        .map_err(|error| format!("create account database pool failed: {error}"))?;

    // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
    #[allow(unreachable_patterns)]
    let pool = match pool {
        DatabasePool::Postgres(pool, _) => pool,
        _ => panic!("authoritative persistence requires a PostgreSQL pool (DATABASE_SPEC: authoritative-server)"),
    };
    Ok(Arc::new(StoreAccountPointsCreditAdapter::postgres(pool)))
}

async fn build_store_account_value_adapter() -> Result<Arc<dyn AccountValueLedgerPort>, String> {
    let _ = dotenvy::dotenv();
    let config = DatabaseConfig::from_env("ACCOUNT")
        .map_err(|error| format!("read account database config failed: {error}"))?;
    let pool = create_pool_from_config(config)
        .await
        .map_err(|error| format!("create account database pool failed: {error}"))?;

    // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
    #[allow(unreachable_patterns)]
    let pool = match pool {
        DatabasePool::Postgres(pool, _) => pool,
        _ => panic!("authoritative persistence requires a PostgreSQL pool (DATABASE_SPEC: authoritative-server)"),
    };
    Ok(Arc::new(StoreAccountPointsCreditAdapter::postgres(pool)))
}
