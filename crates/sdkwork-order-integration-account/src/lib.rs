mod http_adapter;
mod store_adapter;

pub use http_adapter::HttpAccountPointsCreditAdapter;
pub use store_adapter::StoreAccountPointsCreditAdapter;

use std::sync::Arc;

use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service::{AccountPointsCreditPort, AccountValueLedgerPort};

/// Selects how the order module reaches the account ledger.
pub const ACCOUNT_LEDGER_ADAPTER_ENV: &str = "SDKWORK_ORDER_ACCOUNT_LEDGER_ADAPTER";

/// How the order module applies account ledger commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountLedgerAdapterMode {
    /// In-process: call the account repository on the shared database.
    Store,
    /// Out-of-process: HTTP POST to a separately deployed account backend.
    Http,
}

impl AccountLedgerAdapterMode {
    /// Stable wire/log spelling of the mode.
    pub fn label(self) -> &'static str {
        match self {
            Self::Store => "store",
            Self::Http => "http",
        }
    }
}

/// Resolves the account ledger adapter mode for the host connection pool.
///
/// Default is **in-process** (`store`) whenever the host pool is PostgreSQL.
/// SDKWork's cloud deployments run one gateway process against one PostgreSQL,
/// so the order module can call the account module through the shared
/// repository instead of a loopback HTTP hop that needs a service token and a
/// reachable backend — that hop is what produced `SDKWORK_ACCESS_TOKEN is
/// required for account value ledger commands` at redemption time in
/// production while no account HTTP service existed.
///
/// `http` remains an explicit opt-in for deployments that really split order
/// and account into separate services; it requires `SDKWORK_ACCESS_TOKEN` and
/// `SDKWORK_ACCOUNT_BACKEND_API_ORIGIN`. Client-local SQLite sessions keep the
/// `http` default because the store adapter is PostgreSQL-only.
pub fn account_ledger_adapter_mode_for_pool(
    pool: &DatabasePool,
) -> Result<AccountLedgerAdapterMode, String> {
    let configured = std::env::var(ACCOUNT_LEDGER_ADAPTER_ENV).ok();
    resolve_adapter_mode(configured.as_deref(), default_adapter_mode_for_pool(pool))
}

/// The host pool already identifies the deployment shape; re-reading the
/// `ACCOUNT` database configuration here would both guess wrong when that
/// variable is unset and invite a second pool.
fn default_adapter_mode_for_pool(pool: &DatabasePool) -> AccountLedgerAdapterMode {
    if pool.as_postgres().is_some() {
        AccountLedgerAdapterMode::Store
    } else {
        AccountLedgerAdapterMode::Http
    }
}

/// Pure resolution so the environment-free rules stay unit-testable.
fn resolve_adapter_mode(
    configured: Option<&str>,
    default: AccountLedgerAdapterMode,
) -> Result<AccountLedgerAdapterMode, String> {
    match configured.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(default),
        Some(raw) => match raw.to_ascii_lowercase().as_str() {
            "store" => Ok(AccountLedgerAdapterMode::Store),
            "http" => Ok(AccountLedgerAdapterMode::Http),
            other => Err(format!(
                "unsupported {ACCOUNT_LEDGER_ADAPTER_ENV} value: {other} (expected \"store\" or \"http\")"
            )),
        },
    }
}

/// Builds the account points credit port on the host's shared pool.
///
/// The adapter borrows the pool the gateway assembly already owns
/// (`MODULE_SPEC.md` §4.2.7: the in-process path reuses the host connection
/// pool; it does not open a second pool for one call).
pub fn account_points_credit_port_from_database_pool(
    pool: &DatabasePool,
) -> Result<Arc<dyn AccountPointsCreditPort>, String> {
    match account_ledger_adapter_mode_for_pool(pool)? {
        AccountLedgerAdapterMode::Store => Ok(Arc::new(
            StoreAccountPointsCreditAdapter::from_database_pool(pool)?,
        )),
        AccountLedgerAdapterMode::Http => Ok(Arc::new(HttpAccountPointsCreditAdapter::from_env()?)),
    }
}

/// Builds the account value ledger port on the host's shared pool.
pub fn account_value_ledger_port_from_database_pool(
    pool: &DatabasePool,
) -> Result<Arc<dyn AccountValueLedgerPort>, String> {
    match account_ledger_adapter_mode_for_pool(pool)? {
        AccountLedgerAdapterMode::Store => Ok(Arc::new(
            StoreAccountPointsCreditAdapter::from_database_pool(pool)?,
        )),
        AccountLedgerAdapterMode::Http => Ok(Arc::new(HttpAccountPointsCreditAdapter::from_env()?)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        account_ledger_adapter_mode_for_pool, account_points_credit_port_from_database_pool,
        account_value_ledger_port_from_database_pool, default_adapter_mode_for_pool,
        resolve_adapter_mode, AccountLedgerAdapterMode, ACCOUNT_LEDGER_ADAPTER_ENV,
    };
    use sdkwork_database_config::DatabaseConfig;
    use sdkwork_database_sqlx::{DatabasePool, PoolContext};

    /// A lazily connected PostgreSQL pool: no server is required, and the pool
    /// reports size 0 until something actually dials, so a test can prove the
    /// builder reused it instead of opening its own.
    fn lazy_postgres_pool() -> DatabasePool {
        DatabasePool::Postgres(
            sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://order:order@127.0.0.1:1/order")
                .expect("a lazy postgres pool must build without a server"),
            PoolContext {
                config: DatabaseConfig::default(),
            },
        )
    }

    #[test]
    fn unset_uses_the_engine_default() {
        // The cloud/integrated deployment shape: PostgreSQL → in-process store.
        assert_eq!(
            resolve_adapter_mode(None, AccountLedgerAdapterMode::Store),
            Ok(AccountLedgerAdapterMode::Store)
        );
        // Client-local SQLite has no store adapter, so it keeps the HTTP default.
        assert_eq!(
            resolve_adapter_mode(None, AccountLedgerAdapterMode::Http),
            Ok(AccountLedgerAdapterMode::Http)
        );
    }

    #[test]
    fn explicit_values_win_and_are_case_and_whitespace_tolerant() {
        for value in ["store", "STORE", " Store ", "store\n"] {
            assert_eq!(
                resolve_adapter_mode(Some(value), AccountLedgerAdapterMode::Http),
                Ok(AccountLedgerAdapterMode::Store),
                "value {value:?} must select the store adapter"
            );
        }
        for value in ["http", "HTTP", " http "] {
            assert_eq!(
                resolve_adapter_mode(Some(value), AccountLedgerAdapterMode::Store),
                Ok(AccountLedgerAdapterMode::Http),
                "value {value:?} must select the http adapter"
            );
        }
        // A blank value is treated as unset rather than as an unknown mode.
        for value in ["", "   "] {
            assert_eq!(
                resolve_adapter_mode(Some(value), AccountLedgerAdapterMode::Store),
                Ok(AccountLedgerAdapterMode::Store)
            );
        }
    }

    #[test]
    fn an_unknown_mode_names_the_variable_and_the_accepted_values() {
        let error = resolve_adapter_mode(Some("grpc"), AccountLedgerAdapterMode::Store)
            .expect_err("an unknown adapter must not silently fall back");
        assert!(error.contains(ACCOUNT_LEDGER_ADAPTER_ENV), "{error}");
        assert!(error.contains("\"store\""), "{error}");
        assert!(error.contains("\"http\""), "{error}");
    }

    #[test]
    fn the_mode_labels_are_the_documented_wire_spellings() {
        assert_eq!(AccountLedgerAdapterMode::Store.label(), "store");
        assert_eq!(AccountLedgerAdapterMode::Http.label(), "http");
        assert_eq!(
            ACCOUNT_LEDGER_ADAPTER_ENV,
            "SDKWORK_ORDER_ACCOUNT_LEDGER_ADAPTER"
        );
    }

    #[test]
    fn the_default_is_in_process_so_a_missing_service_token_cannot_break_redemption() {
        // Regression guard for the production incident: with no explicit
        // adapter, a PostgreSQL deployment must not require
        // `SDKWORK_ACCESS_TOKEN` (the http adapter's precondition).
        let source = include_str!("lib.rs");
        assert!(
            !source.contains("unwrap_or_else(|_| \"http\".to_owned())"),
            "the adapter must no longer default to http unconditionally"
        );
        // The default comes from the host pool engine, not from a second
        // `ACCOUNT` database configuration lookup.
        assert!(source.contains("pool.as_postgres().is_some()"));
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("the module has a production section");
        assert!(
            !production.contains("create_pool_from_config"),
            "the account adapter must reuse the host pool, never create a second one"
        );
        assert!(
            !production.contains("DatabaseConfig::from_env"),
            "the host pool decides the adapter mode; reading ACCOUNT_* again implies a second pool"
        );
        assert!(
            !production.contains("StoreAccountPointsCreditAdapter::postgres"),
            "the store adapter must be built from the host DatabasePool"
        );
        // The resolver must be the single decision point for both ports.
        assert_eq!(
            production
                .matches("account_ledger_adapter_mode_for_pool(pool)?")
                .count(),
            2,
            "both port builders must resolve the mode through one helper"
        );
    }

    #[tokio::test]
    async fn a_postgres_host_pool_defaults_to_the_in_process_adapter() {
        let pool = lazy_postgres_pool();
        assert_eq!(
            default_adapter_mode_for_pool(&pool),
            AccountLedgerAdapterMode::Store
        );
    }

    #[tokio::test]
    async fn the_account_ports_reuse_the_host_pool_without_dialling_or_duplicating_it() {
        if std::env::var_os(ACCOUNT_LEDGER_ADAPTER_ENV).is_some() {
            // The explicit opt-in is covered by the resolution tests; it is an
            // operator setting, not something a unit test may override.
            return;
        }
        let pool = lazy_postgres_pool();
        let credit = account_points_credit_port_from_database_pool(&pool)
            .expect("the account credit port must reuse the host pool");
        let ledger = account_value_ledger_port_from_database_pool(&pool)
            .expect("the account ledger port must reuse the host pool");
        // Building both ports must not dial the server or construct a second
        // pool: `create_pool_from_config` would have tried to connect here.
        assert_eq!(pool.size(), 0, "no connection may be opened at build time");
        drop((credit, ledger));
    }

    #[tokio::test]
    async fn the_mode_resolution_is_reachable_from_a_host_pool() {
        // Resolving against a host pool must not panic even when the explicit
        // variable is absent; the concrete value follows the pool engine.
        let mode = account_ledger_adapter_mode_for_pool(&lazy_postgres_pool())
            .expect("the default must resolve");
        assert!(matches!(
            mode,
            AccountLedgerAdapterMode::Store | AccountLedgerAdapterMode::Http
        ));
    }
}
