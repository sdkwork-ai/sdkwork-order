//! Periodic course-entitlement relay.
//!
//! Paid digital orders are projected into the course capability through the
//! course entitlement grant endpoint. The relay is intentionally an outbound
//! HTTP consumer: order storage stays authoritative for payment, and course
//! storage stays authoritative for enrollment. Delivery is idempotent because
//! the purchase reference is the course enrollment idempotency key.

use std::{sync::Arc, time::Duration};

use sdkwork_database_sqlx::DatabasePool;
use serde_json::json;
use sqlx::Row;

use crate::OrderServiceHost;

const DEFAULT_INTERVAL_SECONDS: u64 = 30;
const DELIVERY_BATCH: i64 = 50;

/// Starts the relay when both the course grant URL and the shared secret are
/// configured. Returns `None` for deployments that do not mount a course
/// capability.
pub fn spawn_course_entitlement_relay(host: Arc<OrderServiceHost>) -> Option<tokio::task::JoinHandle<()>> {
    let url = std::env::var("SDKWORK_COURSE_ENTITLEMENT_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())?;
    let secret = std::env::var("SDKWORK_COURSE_ENTITLEMENT_SECRET")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())?;
    let DatabasePool::Postgres(pool, _) = host.database_pool() else {
        return None;
    };
    let pool = pool.clone();
    let interval_seconds = std::env::var("SDKWORK_COURSE_ENTITLEMENT_RELAY_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value >= 5)
        .unwrap_or(DEFAULT_INTERVAL_SECONDS);
    Some(tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            if let Err(error) = deliver_paid_course_orders(&pool, &client, &url, &secret).await {
                tracing::warn!(%error, "course entitlement relay pass failed");
            }
            tokio::time::sleep(Duration::from_secs(interval_seconds)).await;
        }
    }))
}

async fn deliver_paid_course_orders(
    pool: &sqlx::PgPool,
    client: &reqwest::Client,
    url: &str,
    secret: &str,
) -> Result<(), String> {
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT o.id AS purchase_ref, o.owner_user_id, co.id AS offering_id
        FROM commerce_order o
        JOIN commerce_order_item i
          ON i.order_id = o.id AND i.tenant_id = o.tenant_id
        JOIN course_offering co
          ON co.tenant_id = o.tenant_id
         AND co.access_policy_json::jsonb ->> 'commerceSkuId' = i.sku_id
         AND co.deleted_at IS NULL
        WHERE (LOWER(o.status) IN ('paid', 'fulfilled', 'completed')
               OR LOWER(o.payment_status) IN ('success', 'paid', 'succeeded'))
        ORDER BY o.id
        LIMIT $1
        "#,
    )
    .bind(DELIVERY_BATCH)
    .fetch_all(pool)
    .await
    .map_err(|error| format!("load paid course orders: {error}"))?;

    for row in rows {
        let purchase_ref: String = row.try_get("purchase_ref").map_err(|error| error.to_string())?;
        let learner_user_id: String = row
            .try_get("owner_user_id")
            .map_err(|error| error.to_string())?;
        let offering_id: String = row.try_get("offering_id").map_err(|error| error.to_string())?;
        let response = client
            .post(url)
            .header("x-sdkwork-entitlement-secret", secret)
            .json(&json!({
                "offeringId": offering_id,
                "learnerUserId": learner_user_id,
                "purchaseRef": purchase_ref,
            }))
            .send()
            .await
            .map_err(|error| format!("grant course entitlement: {error}"))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "course entitlement grant returned {status}: {}",
                body.chars().take(300).collect::<String>()
            ));
        }
    }
    Ok(())
}
