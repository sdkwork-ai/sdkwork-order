//! Order service configuration helpers.

/// Payment expiry window for pending orders, in seconds.
///
/// Defaults to 30 minutes (`SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS`), clamped to
/// a 60s..24h range. The same window backs checkout sessions, checkout orders,
/// account-value orders, and the recharge/membership routers so every order
/// type shares one configurable payment countdown (CONFIG_SPEC: env authority).
pub fn payment_expire_seconds() -> i64 {
    std::env::var("SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(1_800)
        .clamp(60, 86_400)
}

#[cfg(test)]
mod tests {
    use super::payment_expire_seconds;
    use std::sync::Mutex;

    /// Environment-backed config tests mutate process env; serialize them so
    /// parallel test threads cannot race on the same variable.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn payment_expire_seconds_defaults_to_thirty_minutes() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let previous = std::env::var("SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS").ok();
        std::env::remove_var("SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS");
        assert_eq!(payment_expire_seconds(), 1_800);
        match previous {
            Some(value) => std::env::set_var("SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS", value),
            None => std::env::remove_var("SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS"),
        }
    }

    #[test]
    fn payment_expire_seconds_reads_env_and_clamps() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let previous = std::env::var("SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS").ok();
        std::env::set_var("SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS", "120");
        assert_eq!(payment_expire_seconds(), 120);
        std::env::set_var("SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS", "5");
        assert_eq!(payment_expire_seconds(), 60);
        std::env::set_var("SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS", "999999");
        assert_eq!(payment_expire_seconds(), 86_400);
        std::env::set_var("SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS", "not-a-number");
        assert_eq!(payment_expire_seconds(), 1_800);
        match previous {
            Some(value) => std::env::set_var("SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS", value),
            None => std::env::remove_var("SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS"),
        }
    }
}
