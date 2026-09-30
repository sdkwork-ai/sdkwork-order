use sdkwork_contract_service::CommerceServiceError;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service::{
    FulfillPaidPhysicalOrderRequest, PhysicalGoodsFulfillmentOutcome, PhysicalGoodsFulfillmentPort,
    PhysicalGoodsFuture,
};
use sqlx::Row;

use crate::inventory::consume_order_inventory;

pub struct PhysicalFulfillmentAdapter {
    inventory_pool: DatabasePool,
    order_pool: DatabasePool,
}

impl PhysicalFulfillmentAdapter {
    pub fn new(inventory_pool: DatabasePool, order_pool: DatabasePool) -> Self {
        Self {
            inventory_pool,
            order_pool,
        }
    }

    async fn fulfill(
        &self,
        request: FulfillPaidPhysicalOrderRequest,
    ) -> Result<PhysicalGoodsFulfillmentOutcome, CommerceServiceError> {
        let inventory = consume_order_inventory(
            &self.inventory_pool,
            &request.tenant_id,
            &request.order_id,
            &request.idempotency_key,
        )
        .await?;

        // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
        let DatabasePool::Postgres(pool, _) = &self.order_pool;
        let order_replayed = fulfill_postgres(pool, &request).await?;

        Ok(PhysicalGoodsFulfillmentOutcome {
            accepted: true,
            replayed: inventory.replayed && order_replayed,
            fulfillment_status: "awaiting_shipment".to_owned(),
        })
    }
}

impl PhysicalGoodsFulfillmentPort for PhysicalFulfillmentAdapter {
    fn fulfill_paid_physical_order<'a>(
        &'a self,
        request: FulfillPaidPhysicalOrderRequest,
    ) -> PhysicalGoodsFuture<'a, PhysicalGoodsFulfillmentOutcome> {
        Box::pin(async move { self.fulfill(request).await })
    }
}
async fn fulfill_postgres(
    pool: &sqlx::PgPool,
    request: &FulfillPaidPhysicalOrderRequest,
) -> Result<bool, CommerceServiceError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(store_error("begin physical fulfillment"))?;
    let fulfillment_id = fulfillment_id(&request.order_id);
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM commerce_fulfillment_order WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&request.tenant_id)
    .bind(&fulfillment_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(store_error("load physical fulfillment replay"))?;
    if existing > 0 {
        tx.commit()
            .await
            .map_err(store_error("commit physical fulfillment replay"))?;
        return Ok(true);
    }
    let order = sqlx::query("SELECT organization_id, owner_user_id, payment_status, shipping_address_snapshot_json FROM commerce_order WHERE tenant_id = $1 AND id = $2 FOR UPDATE")
        .bind(&request.tenant_id).bind(&request.order_id).fetch_optional(&mut *tx).await
        .map_err(store_error("load paid physical order"))?
        .ok_or_else(|| CommerceServiceError::not_found("physical order was not found"))?;
    validate_paid_order(
        &order
            .try_get::<Option<String>, _>("owner_user_id")
            .ok()
            .flatten()
            .unwrap_or_default(),
        &request.owner_user_id,
        &order
            .try_get::<Option<String>, _>("payment_status")
            .ok()
            .flatten()
            .unwrap_or_default(),
    )?;
    let now = now_string();
    sqlx::query("INSERT INTO commerce_fulfillment_order (id, tenant_id, organization_id, fulfillment_no, order_id, fulfillment_type, status, address_snapshot_id, provider_code, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, 'physical_shipment', 'awaiting_shipment', $6, 'merchant', $7, $7)")
        .bind(&fulfillment_id).bind(&request.tenant_id).bind(order.try_get::<Option<String>, _>("organization_id").ok().flatten())
        .bind(&fulfillment_id).bind(&request.order_id).bind(format!("address-{}", request.order_id)).bind(&now)
        .execute(&mut *tx).await.map_err(store_error("create physical fulfillment"))?;
    sqlx::query("UPDATE commerce_order SET fulfillment_status = 'awaiting_shipment', updated_at = CAST($1 AS TIMESTAMPTZ) WHERE tenant_id = $2 AND id = $3 AND payment_status IN ('success', 'succeeded', 'paid')")
        .bind(&now).bind(&request.tenant_id).bind(&request.order_id).execute(&mut *tx).await.map_err(store_error("advance physical order fulfillment"))?;
    sqlx::query("UPDATE commerce_order_item SET fulfillment_status = 'awaiting_shipment' WHERE tenant_id = $1 AND order_id = $2")
        .bind(&request.tenant_id).bind(&request.order_id).execute(&mut *tx).await.map_err(store_error("advance physical order items"))?;
    tx.commit()
        .await
        .map_err(store_error("commit physical fulfillment"))?;
    Ok(false)
}
fn validate_paid_order(
    stored_owner: &str,
    expected_owner: &str,
    payment_status: &str,
) -> Result<(), CommerceServiceError> {
    if stored_owner != expected_owner {
        return Err(CommerceServiceError::not_found(
            "physical order was not found",
        ));
    }
    if !matches!(
        payment_status.trim().to_ascii_lowercase().as_str(),
        "success" | "succeeded" | "paid"
    ) {
        return Err(CommerceServiceError::invalid_state(
            "physical order payment is not successful",
        ));
    }
    Ok(())
}

fn fulfillment_id(order_id: &str) -> String {
    format!("physical-fulfillment-{order_id}")
}
fn now_string() -> String {
    sdkwork_order_service::canonical_now_timestamp()
}
fn store_error(message: &'static str) -> impl FnOnce(sqlx::Error) -> CommerceServiceError {
    move |error| CommerceServiceError::storage(format!("{message}: {error}"))
}
