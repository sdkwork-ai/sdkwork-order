//! Order → Deployments fulfillment adapter for app-template purchases.
//!
//! The order center owns the app-template trade: the order, its payment state
//! and the entitlement are all `commerce_order` rows, so this adapter grants
//! nothing on the deployments side. On settlement it
//!
//! 1. advances the order's `fulfillment_status` to `fulfilled`, and
//! 2. increments the purchased listing's `install_count` in
//!    `deploy_app_template`, the counter the marketplace sorts and displays.
//!
//! Both writes commit in one transaction and the counter is only touched when
//! the order row actually transitioned, so a replayed provider webhook (or the
//! payment compensation worker retrying) counts an install exactly once.
//!
//! Cross-module access is by table read/write inside the workspace database —
//! the same shape `PostgresCommerceMembershipOrderStore` uses for
//! `membership_package` — and `deploy_app_template` is the only deployments
//! table this adapter writes.

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service::{
    AppTemplatePurchaseFulfillmentFuture, AppTemplatePurchaseFulfillmentOutcome,
    AppTemplatePurchaseFulfillmentPort, FulfillPaidAppTemplateOrderRequest,
};
use sqlx::{PgPool, Row};

/// `commerce_order.subject` value this adapter fulfills.
const APP_TEMPLATE_SUBJECT: &str = "app_template";
/// Lifecycle value this adapter writes on both the order and its item.
const FULFILLED: &str = "fulfilled";

pub struct StoreAppTemplateFulfillmentAdapter {
    pool: DatabasePool,
}

impl StoreAppTemplateFulfillmentAdapter {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    async fn fulfill(
        &self,
        request: FulfillPaidAppTemplateOrderRequest,
    ) -> Result<AppTemplatePurchaseFulfillmentOutcome, CommerceServiceError> {
        // Authoritative server persistence is PostgreSQL only (DATABASE_SPEC).
        #[allow(unreachable_patterns)]
        let pool = match &self.pool {
            DatabasePool::Postgres(pool, _) => pool,
            _ => panic!("authoritative persistence requires a PostgreSQL pool (DATABASE_SPEC: authoritative-server)"),
        };
        fulfill_postgres(pool, &request).await
    }
}

impl AppTemplatePurchaseFulfillmentPort for StoreAppTemplateFulfillmentAdapter {
    fn fulfill_app_template_purchase<'a>(
        &'a self,
        request: FulfillPaidAppTemplateOrderRequest,
    ) -> AppTemplatePurchaseFulfillmentFuture<'a, AppTemplatePurchaseFulfillmentOutcome> {
        Box::pin(async move { self.fulfill(request).await })
    }
}

pub fn app_template_purchase_fulfillment_port_from_database_pool(
    pool: &DatabasePool,
) -> Result<std::sync::Arc<dyn AppTemplatePurchaseFulfillmentPort>, String> {
    let adapter = StoreAppTemplateFulfillmentAdapter::new(pool.clone());
    let port: std::sync::Arc<dyn AppTemplatePurchaseFulfillmentPort> = std::sync::Arc::new(adapter);
    Ok(port)
}

async fn fulfill_postgres(
    pool: &PgPool,
    request: &FulfillPaidAppTemplateOrderRequest,
) -> Result<AppTemplatePurchaseFulfillmentOutcome, CommerceServiceError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(store_error("begin app template fulfillment"))?;

    let order = sqlx::query(
        "SELECT owner_user_id, COALESCE(subject, '') AS subject,
                COALESCE(payment_status, '') AS payment_status,
                COALESCE(fulfillment_status, '') AS fulfillment_status
           FROM commerce_order
          WHERE tenant_id = CAST($1 AS TEXT) AND id = CAST($2 AS TEXT)
          FOR UPDATE",
    )
    .bind(&request.tenant_id)
    .bind(&request.order_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(store_error("load app template order"))?
    .ok_or_else(|| CommerceServiceError::not_found("app template order was not found"))?;

    let stored_owner = order
        .try_get::<Option<String>, _>("owner_user_id")
        .ok()
        .flatten()
        .unwrap_or_default();
    if stored_owner != request.owner_user_id {
        return Err(CommerceServiceError::not_found(
            "app template order was not found",
        ));
    }
    let subject = order
        .try_get::<Option<String>, _>("subject")
        .ok()
        .flatten()
        .unwrap_or_default();
    if !subject.eq_ignore_ascii_case(APP_TEMPLATE_SUBJECT) {
        return Err(CommerceServiceError::invalid_state(
            "order is not an app template order",
        ));
    }
    let payment_status = order
        .try_get::<Option<String>, _>("payment_status")
        .ok()
        .flatten()
        .unwrap_or_default();
    if !matches!(
        payment_status.trim().to_ascii_lowercase().as_str(),
        "success" | "succeeded" | "paid"
    ) {
        return Err(CommerceServiceError::invalid_state(
            "app template order payment is not successful",
        ));
    }
    let fulfillment_status = order
        .try_get::<Option<String>, _>("fulfillment_status")
        .ok()
        .flatten()
        .unwrap_or_default();

    let template_uuid = load_order_template_uuid(&mut tx, request).await?;
    let template_uuid = template_uuid.ok_or_else(|| {
        CommerceServiceError::invalid_state("app template order carries no template reference")
    })?;

    if fulfillment_status.eq_ignore_ascii_case(FULFILLED) {
        let install_count = read_install_count(&mut tx, &template_uuid).await?;
        tx.commit()
            .await
            .map_err(store_error("commit app template fulfillment replay"))?;
        return Ok(AppTemplatePurchaseFulfillmentOutcome {
            accepted: true,
            replayed: true,
            fulfillment_status: FULFILLED.to_owned(),
            install_count,
            template_uuid: Some(template_uuid),
        });
    }

    let now = sdkwork_order_service::canonical_now_timestamp();
    let transitioned = sqlx::query(
        "UPDATE commerce_order
            SET fulfillment_status = $1, updated_at = CAST($2 AS TIMESTAMPTZ)
          WHERE tenant_id = CAST($3 AS TEXT)
            AND id = CAST($4 AS TEXT)
            AND COALESCE(fulfillment_status, '') IS DISTINCT FROM $1",
    )
    .bind(FULFILLED)
    .bind(&now)
    .bind(&request.tenant_id)
    .bind(&request.order_id)
    .execute(&mut *tx)
    .await
    .map_err(store_error("advance app template order fulfillment"))?
    .rows_affected()
        == 1;

    sqlx::query(
        "UPDATE commerce_order_item
            SET fulfillment_status = $1
          WHERE tenant_id = CAST($2 AS TEXT) AND order_id = CAST($3 AS TEXT)",
    )
    .bind(FULFILLED)
    .bind(&request.tenant_id)
    .bind(&request.order_id)
    .execute(&mut *tx)
    .await
    .map_err(store_error("advance app template order items"))?;

    // Only the transition that actually moved the order counts the install.
    let install_count = if transitioned {
        sqlx::query(
            "UPDATE deploy_app_template
                SET install_count = install_count + 1, updated_at = CAST($1 AS TIMESTAMPTZ)
              WHERE uuid = $2 AND deleted_at IS NULL
              RETURNING install_count",
        )
        .bind(&now)
        .bind(&template_uuid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(store_error("count app template install"))?
        .and_then(|row| row.try_get::<i64, _>("install_count").ok())
    } else {
        read_install_count(&mut tx, &template_uuid).await?
    };

    tx.commit()
        .await
        .map_err(store_error("commit app template fulfillment"))?;

    Ok(AppTemplatePurchaseFulfillmentOutcome {
        accepted: true,
        replayed: !transitioned,
        fulfillment_status: FULFILLED.to_owned(),
        install_count,
        template_uuid: Some(template_uuid),
    })
}

/// Reads the template this order entitles. The snapshot is rendered through the
/// row's JSON form because the published baseline declares
/// `sku_snapshot_json` as TEXT while test fixtures declare JSONB.
async fn load_order_template_uuid(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &FulfillPaidAppTemplateOrderRequest,
) -> Result<Option<String>, CommerceServiceError> {
    let row = sqlx::query(
        "SELECT COALESCE(
                    NULLIF(COALESCE(NULLIF(to_jsonb(commerce_order_item) ->> 'sku_snapshot_json', ''), '{}')::jsonb ->> 'templateUuid', ''),
                    NULLIF(to_jsonb(commerce_order_item) ->> 'sku_id', '')
                ) AS template_uuid
           FROM commerce_order_item
          WHERE tenant_id = CAST($1 AS TEXT) AND order_id = CAST($2 AS TEXT)
          ORDER BY created_at ASC, id ASC
          LIMIT 1",
    )
    .bind(&request.tenant_id)
    .bind(&request.order_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(store_error("load app template order item"))?;

    Ok(row
        .and_then(|row| {
            row.try_get::<Option<String>, _>("template_uuid")
                .ok()
                .flatten()
        })
        .filter(|value| !value.trim().is_empty()))
}

async fn read_install_count(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    template_uuid: &str,
) -> Result<Option<i64>, CommerceServiceError> {
    let row = sqlx::query(
        "SELECT install_count FROM deploy_app_template WHERE uuid = $1 AND deleted_at IS NULL",
    )
    .bind(template_uuid)
    .fetch_optional(&mut **tx)
    .await
    .map_err(store_error("read app template install count"))?;
    Ok(row.and_then(|row| row.try_get::<i64, _>("install_count").ok()))
}

fn store_error(message: &'static str) -> impl FnOnce(sqlx::Error) -> CommerceServiceError {
    move |error| CommerceServiceError::storage(format!("{message}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{APP_TEMPLATE_SUBJECT, FULFILLED};

    /// The adapter's SQL is built inline; these literals are the contract with
    /// `PostgresCommerceAppTemplateOrderStore` and must not drift from it.
    #[test]
    fn adapter_uses_the_order_center_subject_and_fulfilled_status() {
        assert_eq!(APP_TEMPLATE_SUBJECT, "app_template");
        assert_eq!(FULFILLED, "fulfilled");
    }

    #[test]
    fn adapter_sql_only_writes_the_install_counter_on_the_catalog_side() {
        let source = include_str!("lib.rs");
        assert!(source.contains("UPDATE deploy_app_template"));
        assert!(source.contains("SET install_count = install_count + 1"));
        assert!(source.contains("IS DISTINCT FROM $1"));
        assert!(source.contains("to_jsonb(commerce_order_item) ->> 'sku_snapshot_json'"));
        // No deployments-owned purchase table may come back through this seam.
        // The needle is assembled so this assertion does not match its own text.
        let retired_table = ["deploy", "app", "template", "purchase"].join("_");
        assert!(!source.contains(&retired_table));
    }
}
