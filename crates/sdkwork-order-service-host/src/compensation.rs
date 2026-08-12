//! In-process payment compensation worker (embedded in the standalone
//! gateway).
//!
//! Compensation is the at-least-once safety net for provider notifications:
//! every tick the worker claims payment attempts stuck in pending/processing
//! and refunds stuck in submitted/processing, queries the PSP for the real
//! state, and re-enters the **same notify processing framework** with a
//! synthetic event (`sdkwork-order-integration-payment`:
//! `run_payment_compensation_pass`). Idempotent event ids, terminal status
//! preservation, and fulfillment keys make the query path equivalent to the
//! webhook path — a lost webhook is recovered without double success.
//!
//! Configuration (CONFIG_SPEC: env authority):
//! - `SDKWORK_ORDER_PAYMENT_COMPENSATION_WORKER_ENABLED` (default disabled —
//!   opt-in so no deployment accidentally polls providers)
//! - `SDKWORK_ORDER_PAYMENT_COMPENSATION_TENANT_ID` (default unset = all
//!   tenants)
//! - `SDKWORK_ORDER_PAYMENT_COMPENSATION_ORGANIZATION_ID` (default unset)
//! - `SDKWORK_ORDER_PAYMENT_COMPENSATION_BATCH_SIZE` (default 50, clamped
//!   1..1000)
//! - `SDKWORK_ORDER_PAYMENT_COMPENSATION_INTERVAL_MILLIS` (default 30_000,
//!   clamped 5_000..3_600_000)
//! - `SDKWORK_ORDER_PAYMENT_COMPENSATION_MIN_AGE_SECONDS` (default 60:
//!   fresh attempts keep their webhook window)
//! - `SDKWORK_ORDER_PAYMENT_COMPENSATION_MAX_AGE_SECONDS` (default 86_400:
//!   bound PSP query load)

use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Duration;

use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_integration_payment::{
    run_payment_compensation_pass, PaymentCompensationPassConfig,
};
use sdkwork_order_repository_sqlx::{PostgresCommerceOrderStore, PostgresCommerceRechargeStore};
use sdkwork_order_service::OwnerOrderSettlementPorts;
use sdkwork_payment_providers::ProviderCredentialBundle;
use sdkwork_payment_repository_sqlx::PostgresCommerceOwnerOrderPaymentStore;
use tokio::time::{interval, MissedTickBehavior};

use crate::OrderServiceHost;

const DEFAULT_INTERVAL_MILLIS: u64 = 30_000;
const MIN_INTERVAL_MILLIS: u64 = 5_000;
const MAX_INTERVAL_MILLIS: u64 = 3_600_000;
const DEFAULT_BATCH_SIZE: i64 = 50;
const MAX_BATCH_SIZE: i64 = 1_000;
const DEFAULT_MIN_AGE_SECONDS: i64 = 60;
const DEFAULT_MAX_AGE_SECONDS: i64 = 24 * 60 * 60;

static WORKER_STARTED: AtomicBool = AtomicBool::new(false);

/// Spawns the payment compensation worker loop on the host's pools.
///
/// Returns `None` when the worker is disabled by env (default) or already
/// running.
pub fn spawn_payment_compensation_worker(
    host: Arc<OrderServiceHost>,
) -> Option<tokio::task::JoinHandle<()>> {
    if !worker_enabled_from_env() {
        tracing::info!(
            target = "order.payment_compensation",
            "payment compensation worker is disabled by SDKWORK_ORDER_PAYMENT_COMPENSATION_WORKER_ENABLED"
        );
        return None;
    }
    if WORKER_STARTED.swap(true, AtomicOrdering::SeqCst) {
        tracing::warn!(
            target = "order.payment_compensation",
            "payment compensation worker is already running"
        );
        return None;
    }
    let config = PaymentCompensationPassConfig {
        tenant_id: env_var_optional("SDKWORK_ORDER_PAYMENT_COMPENSATION_TENANT_ID"),
        organization_id: env_var_optional("SDKWORK_ORDER_PAYMENT_COMPENSATION_ORGANIZATION_ID"),
        batch_size: env_i64_clamped(
            "SDKWORK_ORDER_PAYMENT_COMPENSATION_BATCH_SIZE",
            DEFAULT_BATCH_SIZE,
            1,
            MAX_BATCH_SIZE,
        ),
        min_age_seconds: env_i64_clamped(
            "SDKWORK_ORDER_PAYMENT_COMPENSATION_MIN_AGE_SECONDS",
            DEFAULT_MIN_AGE_SECONDS,
            0,
            DEFAULT_MAX_AGE_SECONDS,
        ),
        max_age_seconds: env_i64_clamped(
            "SDKWORK_ORDER_PAYMENT_COMPENSATION_MAX_AGE_SECONDS",
            DEFAULT_MAX_AGE_SECONDS,
            DEFAULT_MIN_AGE_SECONDS,
            30 * 24 * 60 * 60,
        ),
    };
    let interval_millis = env_u64_clamped(
        "SDKWORK_ORDER_PAYMENT_COMPENSATION_INTERVAL_MILLIS",
        DEFAULT_INTERVAL_MILLIS,
        MIN_INTERVAL_MILLIS,
        MAX_INTERVAL_MILLIS,
    );
    tracing::info!(
        target = "order.payment_compensation",
        interval_millis,
        batch_size = config.batch_size,
        tenant_id = config.tenant_id.as_deref(),
        organization_id = config.organization_id.as_deref(),
        min_age_seconds = config.min_age_seconds,
        max_age_seconds = config.max_age_seconds,
        "payment compensation worker started"
    );
    Some(tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(interval_millis));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(error) = run_compensation_pass(&host, &config).await {
                tracing::error!(
                    target = "order.payment_compensation",
                    error = ?error,
                    "payment compensation pass failed"
                );
            }
        }
    }))
}

async fn run_compensation_pass(
    host: &OrderServiceHost,
    config: &PaymentCompensationPassConfig,
) -> Result<(), String> {
    let DatabasePool::Postgres(pool, _) = host.database_pool() else {
        return Err("payment compensation worker requires a PostgreSQL pool".to_owned());
    };
    let credentials = ProviderCredentialBundle::from_env();
    let payments = PostgresCommerceOwnerOrderPaymentStore::new(pool.clone());
    let orders = PostgresCommerceOrderStore::new(pool.clone());
    let recharge = PostgresCommerceRechargeStore::new(pool.clone());
    let credit_port = host.account_credit_port();
    let account_value_ledger_port = host.account_value_ledger_port();
    let coupon_redemption_port = host.coupon_redemption_port();
    let membership_port = host.membership_fulfillment_port();
    let physical_goods_port = host.physical_goods_fulfillment_port();
    let settlement_ports = OwnerOrderSettlementPorts {
        payment_store: &payments,
        order_state_store: &orders,
        recharge_store: &recharge,
        account_value_store: &recharge,
        credit_port: credit_port.as_ref(),
        account_value_ledger_port: account_value_ledger_port.as_ref(),
        coupon_redemption_port: coupon_redemption_port.as_ref(),
        membership_port: membership_port.as_ref(),
        physical_goods_port: physical_goods_port.as_ref(),
    };
    let summary = run_payment_compensation_pass(pool, &credentials, settlement_ports, config)
        .await
        .map_err(|error| format!("payment compensation pass failed: {error:?}"))?;
    tracing::info!(
        target = "order.payment_compensation",
        claimed_payment_attempts = summary.claimed_payment_attempts,
        claimed_refunds = summary.claimed_refunds,
        payment_events_applied = summary.payment_events_applied,
        refund_events_applied = summary.refund_events_applied,
        payments_still_pending = summary.payments_still_pending,
        payments_not_found = summary.payments_not_found,
        refunds_still_processing = summary.refunds_still_processing,
        refunds_not_found = summary.refunds_not_found,
        skipped_unrepresentable = summary.skipped_unrepresentable,
        errors = summary.errors,
        "payment compensation pass finished"
    );
    Ok(())
}

fn worker_enabled_from_env() -> bool {
    match std::env::var("SDKWORK_ORDER_PAYMENT_COMPENSATION_WORKER_ENABLED") {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    }
}

fn env_var_optional(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn env_i64_clamped(name: &str, default: i64, min: i64, max: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn env_u64_clamped(name: &str, default: u64, min: u64, max: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}
