//! Platform commerce catalog tenant resolution.
//!
//! Public recharge and membership packages fall back to a platform-owned tenant when
//! the caller tenant has no scoped catalog. The tenant id is configurable via
//! `SDKWORK_ORDER_PLATFORM_CATALOG_TENANT_ID` (default `100001`).

pub const ENV_PLATFORM_CATALOG_TENANT_ID: &str = "SDKWORK_ORDER_PLATFORM_CATALOG_TENANT_ID";
pub const DEFAULT_PLATFORM_CATALOG_TENANT_ID: &str = "100001";
const PLATFORM_TENANT_PLACEHOLDER: &str = "__PLATFORM_TENANT__";

/// Resolves the platform catalog tenant id from environment with validation.
pub fn platform_catalog_tenant_id() -> String {
    std::env::var(ENV_PLATFORM_CATALOG_TENANT_ID)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| is_valid_platform_catalog_tenant_id(value))
        .unwrap_or_else(|| DEFAULT_PLATFORM_CATALOG_TENANT_ID.to_owned())
}

/// Materializes SQL that contains `__PLATFORM_TENANT__` with the resolved platform tenant.
pub fn materialize_platform_catalog_sql(template: &str) -> String {
    template.replace(PLATFORM_TENANT_PLACEHOLDER, &platform_catalog_tenant_id())
}

fn is_valid_platform_catalog_tenant_id(value: &str) -> bool {
    // Digits only, because this id is materialized into comparisons against the merchandise
    // catalog, whose `tenant_id` is `BIGINT`. An alphanumeric value used to be usable when the
    // catalog keyed tenants by text; today it would produce `bigint = 'acme-01'` and fail inside
    // PostgreSQL. Rejecting it here means a misconfigured environment falls back to the default
    // instead of failing the query it was meant to serve.
    !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_platform_catalog_tenant_is_stable() {
        let _guard = EnvGuard::unset(ENV_PLATFORM_CATALOG_TENANT_ID);
        assert_eq!(
            platform_catalog_tenant_id(),
            DEFAULT_PLATFORM_CATALOG_TENANT_ID
        );
    }

    #[test]
    fn invalid_platform_catalog_tenant_falls_back_to_default() {
        let _guard = EnvGuard::set(ENV_PLATFORM_CATALOG_TENANT_ID, "'; DROP TABLE--");
        assert_eq!(
            platform_catalog_tenant_id(),
            DEFAULT_PLATFORM_CATALOG_TENANT_ID
        );
    }

    #[test]
    fn platform_catalog_tenant_must_be_a_decimal_id() {
        // The predicate is exercised directly rather than through the environment: the catalog
        // keys tenants on `BIGINT`, so only a decimal id can select a row, while an alphanumeric
        // value would materialize as `bigint = 'acme-01'` and fail inside PostgreSQL. Testing the
        // predicate keeps this assertion off the process environment that the tests above mutate.
        assert!(is_valid_platform_catalog_tenant_id("200002"));
        assert!(!is_valid_platform_catalog_tenant_id("acme-01"));
        assert!(!is_valid_platform_catalog_tenant_id("100001-1"));
        assert!(!is_valid_platform_catalog_tenant_id(""));
    }

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn unset(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            // SAFETY: test-only serial mutation of process environment.
            unsafe { std::env::remove_var(key) };
            Self { key, previous }
        }

        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            // SAFETY: test-only serial mutation of process environment.
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }
}
