//! Order runtime environment resolution.
//!
//! Every posture decision in this application — Snowflake node allocation above all — must derive
//! from **one** resolution of "which environment am I running in". Resolving it twice with two
//! different key lists is how a process ends up leasing a node id because it believed it was in
//! production while the gateway applies the development CORS policy.
//!
//! The platform owns the resolution rules; this module only names the key list Order uses and
//! exposes the production-like predicate on top of [`sdkwork_web_bootstrap::web_environment_from_env`].
//!
//! # Why two keys
//!
//! `SDKWORK_ORDER_ENVIRONMENT` is the Order-specific key and wins when both are set.
//! `SDKWORK_ENVIRONMENT` is the workspace-wide key every deployment profile also writes, so it is
//! honoured as a fallback: resolving to `Dev` because only the shared key was set would silently
//! relax production postures, and `/etc/topology/*.env` sets both consistently.

use sdkwork_web_bootstrap::web_environment_from_env;
use sdkwork_web_core::WebEnvironment;

/// Environment keys consulted, most specific first.
pub const ORDER_ENVIRONMENT_KEYS: &[&str] = &["SDKWORK_ORDER_ENVIRONMENT", "SDKWORK_ENVIRONMENT"];

/// Resolve the Order runtime environment.
///
/// Unknown values fail closed onto [`WebEnvironment::Prod`] — the platform resolver's own rule, so
/// a typo can never buy a relaxed posture.
#[must_use]
pub fn order_environment() -> WebEnvironment {
    web_environment_from_env(ORDER_ENVIRONMENT_KEYS)
}

/// Canonical environment name for logs and diagnostics.
#[must_use]
pub fn order_environment_name() -> &'static str {
    match order_environment() {
        WebEnvironment::Dev => "dev",
        WebEnvironment::Test => "test",
        WebEnvironment::Prod => "prod",
    }
}

/// Whether this process must run with production posture.
///
/// Staging and production both resolve to [`WebEnvironment::Prod`], so this is the single predicate
/// to gate fail-closed behaviour on.
#[must_use]
pub fn order_is_production_like() -> bool {
    order_environment() == WebEnvironment::Prod
}
