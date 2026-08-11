//! In-process order expiration scheduler (embedded in the standalone gateway).
//!
//! Every tick scans orders whose payment window (`expired_at`) has elapsed
//! and transitions them to `expired` with a system lifecycle event, then:
//! - closes open payment attempts through the payment provider store, and
//! - releases physical inventory reservations for physical orders.
//!
//! Configuration (CONFIG_SPEC: env authority):
//! - `SDKWORK_ORDER_EXPIRATION_SCHEDULER_ENABLED` (default enabled)
//! - `SDKWORK_ORDER_EXPIRATION_SCHEDULER_INTERVAL_SECONDS` (default 60,
//!   clamped 10..3600)
//! - `SDKWORK_ORDER_EXPIRATION_BATCH_SIZE` (default 200, clamped 1..2000)

use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Duration;

use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_repository_sqlx::postgres_expiration::{
    expire_due_order, list_due_expiring_orders, ExpiringOrderRecord,
};
use sdkwork_order_service::{
    physical_inventory_release_idempotency_key, physical_order_fulfillment_requires_release,
    ReleasePhysicalOrderInventoryRequest,
};
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_payment_repository_sqlx::close_expired_owner_order_provider_attempts_postgres;
use tokio::time::{interval, MissedTickBehavior};

use crate::OrderServiceHost;

const DEFAULT_INTERVAL_SECONDS: u64 = 60;
const MIN_INTERVAL_SECONDS: u64 = 10;
const MAX_INTERVAL_SECONDS: u64 = 3_600;
const DEFAULT_BATCH_SIZE: i64 = 200;
const MAX_BATCH_SIZE: i64 = 2_000;

static SCHEDULER_STARTED: AtomicBool = AtomicBool::new(false);

/// Spawns the order expiration scheduler loop on the host's pools.
///
/// Returns `None` when the scheduler is disabled by env or already running.
pub fn spawn_order_expiration_scheduler(
    host: Arc<OrderServiceHost>,
) -> Option<tokio::task::JoinHandle<()>> {
    if !scheduler_enabled_from_env() {
        tracing::info!(
            target = "order.expiration",
            "order expiration scheduler is disabled by SDKWORK_ORDER_EXPIRATION_SCHEDULER_ENABLED"
        );
        return None;
    }
    if SCHEDULER_STARTED.swap(true, AtomicOrdering::SeqCst) {
        tracing::warn!(
            target = "order.expiration",
            "order expiration scheduler is already running"
        );
        return None;
    }
    let interval_seconds = scheduler_interval_seconds();
    let batch_size = scheduler_batch_size();
    tracing::info!(
        target = "order.expiration",
        interval_seconds,
        batch_size,
        "order expiration scheduler started"
    );
    Some(tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(interval_seconds));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(error) = run_expiration_pass(&host, batch_size).await {
                tracing::error!(
                    target = "order.expiration",
                    error = ?error,
                    "order expiration pass failed"
                );
            }
        }
    }))
}

async fn run_expiration_pass(host: &OrderServiceHost, batch_size: i64) -> Result<(), String> {
    let DatabasePool::Postgres(pool, _) = host.database_pool() else {
        return Err("order expiration scheduler requires a PostgreSQL pool".to_owned());
    };
    let credentials = ProviderCredentialBundle::from_env();
    let registry = Arc::new(PaymentProviderRegistry::from_credentials(
        credentials.clone(),
    ));
    let records = list_due_expiring_orders(pool, batch_size)
        .await
        .map_err(|error| format!("list expiring orders failed: {error:?}"))?;
    for record in records {
        let now = current_command_timestamp();
        let expired = expire_due_order(pool, &record, &now)
            .await
            .map_err(|error| format!("expire order {} failed: {error:?}", record.order_id))?;
        if !expired {
            continue;
        }
        close_expired_payment_attempts(pool, &registry, &credentials, &record).await;
        release_physical_inventory(host, &record).await;
        tracing::info!(
            target = "order.expiration",
            order_id = %record.order_id,
            "order expired automatically after payment window elapsed"
        );
    }
    sweep_expired_inventory_reservations(host, batch_size).await;
    Ok(())
}

/// Consistency backstop for physical inventory: releases any reservation
/// whose own expiry window has elapsed while still `reserved`. Covers
/// failed releases, legacy orders without `expired_at`, and abandoned
/// payment windows — independent of the order's current status.
async fn sweep_expired_inventory_reservations(host: &OrderServiceHost, batch_size: i64) {
    match host
        .physical_inventory_reservation_port()
        .sweep_expired_inventory_reservations(batch_size)
        .await
    {
        Ok(swept) if swept > 0 => {
            tracing::info!(
                target = "order.expiration",
                swept,
                "released expired physical inventory reservations"
            );
        }
        Ok(_) => {}
        Err(error) => {
            tracing::warn!(
                target = "order.expiration",
                error = ?error,
                "failed to sweep expired physical inventory reservations"
            );
        }
    }
}

/// Best-effort close of open payment attempts for the expired order; a
/// provider failure must not roll back the order expiration itself.
async fn close_expired_payment_attempts(
    pool: &sqlx::PgPool,
    registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    record: &ExpiringOrderRecord,
) {
    if let Err(error) = close_expired_owner_order_provider_attempts_postgres(
        pool,
        registry,
        credentials,
        &record.tenant_id,
        record.organization_id.as_deref(),
        &record.owner_user_id,
    )
    .await
    {
        tracing::warn!(
            target = "order.expiration",
            order_id = %record.order_id,
            error = ?error,
            "failed to close expired payment attempts"
        );
    }
}

/// Best-effort release of physical inventory for physical orders; the
/// reservation release is independent of the order state transition.
async fn release_physical_inventory(host: &OrderServiceHost, record: &ExpiringOrderRecord) {
    if !physical_order_fulfillment_requires_release(record.fulfillment_status.as_deref()) {
        return;
    }
    let release = ReleasePhysicalOrderInventoryRequest {
        tenant_id: record.tenant_id.clone(),
        order_id: record.order_id.clone(),
        reason_code: "payment_timeout".to_owned(),
        request_no: format!("system-expire-{}", record.order_id),
        idempotency_key: physical_inventory_release_idempotency_key(&record.order_id),
    };
    if let Err(error) = host
        .physical_inventory_reservation_port()
        .release_physical_order_inventory(release)
        .await
    {
        tracing::warn!(
            target = "order.expiration",
            order_id = %record.order_id,
            error = ?error,
            "failed to release expired order inventory"
        );
    }
}

fn scheduler_enabled_from_env() -> bool {
    match std::env::var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_ENABLED") {
        Ok(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "" | "0" | "false" | "off" | "no" | "disabled"
        ),
        Err(_) => true,
    }
}

fn scheduler_interval_seconds() -> u64 {
    std::env::var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_INTERVAL_SECONDS)
        .clamp(MIN_INTERVAL_SECONDS, MAX_INTERVAL_SECONDS)
}

fn scheduler_batch_size() -> i64 {
    std::env::var("SDKWORK_ORDER_EXPIRATION_BATCH_SIZE")
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(DEFAULT_BATCH_SIZE)
        .clamp(1, MAX_BATCH_SIZE)
}

fn current_command_timestamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{seconds}")
}

#[cfg(test)]
mod tests {
    use super::{scheduler_batch_size, scheduler_enabled_from_env, scheduler_interval_seconds};

    #[test]
    fn scheduler_config_defaults_and_clamps() {
        let previous_interval =
            std::env::var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_INTERVAL_SECONDS").ok();
        let previous_batch = std::env::var("SDKWORK_ORDER_EXPIRATION_BATCH_SIZE").ok();
        let previous_enabled = std::env::var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_ENABLED").ok();
        std::env::remove_var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_INTERVAL_SECONDS");
        std::env::remove_var("SDKWORK_ORDER_EXPIRATION_BATCH_SIZE");
        std::env::remove_var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_ENABLED");
        assert_eq!(scheduler_interval_seconds(), 60);
        assert_eq!(scheduler_batch_size(), 200);
        assert!(scheduler_enabled_from_env());

        std::env::set_var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_INTERVAL_SECONDS", "5");
        std::env::set_var("SDKWORK_ORDER_EXPIRATION_BATCH_SIZE", "99999");
        std::env::set_var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_ENABLED", "0");
        assert_eq!(scheduler_interval_seconds(), 10);
        assert_eq!(scheduler_batch_size(), 2_000);
        assert!(!scheduler_enabled_from_env());

        std::env::set_var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_ENABLED", "true");
        assert!(scheduler_enabled_from_env());

        match previous_interval {
            Some(value) => {
                std::env::set_var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_INTERVAL_SECONDS", value)
            }
            None => std::env::remove_var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_INTERVAL_SECONDS"),
        }
        match previous_batch {
            Some(value) => std::env::set_var("SDKWORK_ORDER_EXPIRATION_BATCH_SIZE", value),
            None => std::env::remove_var("SDKWORK_ORDER_EXPIRATION_BATCH_SIZE"),
        }
        match previous_enabled {
            Some(value) => std::env::set_var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_ENABLED", value),
            None => std::env::remove_var("SDKWORK_ORDER_EXPIRATION_SCHEDULER_ENABLED"),
        }
    }
}
