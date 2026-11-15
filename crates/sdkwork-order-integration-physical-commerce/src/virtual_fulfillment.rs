use sdkwork_contract_service::CommerceServiceError;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service::{
    FulfillPaidVirtualOrderRequest, VirtualGoodsFulfillmentOutcome, VirtualGoodsFulfillmentPort,
    VirtualGoodsFuture,
};
use sqlx::Row;

/// Virtual-goods fulfillment: payment success IS delivery.
///
/// The adapter performs one idempotent transaction per order:
///
/// 1. claim `commerce_fulfillment_order` for `virtual_delivery` (a replay
///    short-circuits to `replayed = true`);
/// 2. move the order + its items to `fulfilled`.
///
/// It never touches inventory (a virtual order has no stock reservation) and
/// never re-writes a terminal negative order state, so a replayed provider
/// webhook, the compensation sweep, and an operator re-confirmation all
/// converge on exactly one delivery.
pub struct VirtualFulfillmentAdapter {
    order_pool: DatabasePool,
}

impl VirtualFulfillmentAdapter {
    pub fn new(order_pool: DatabasePool) -> Self {
        Self { order_pool }
    }

    async fn fulfill(
        &self,
        request: FulfillPaidVirtualOrderRequest,
    ) -> Result<VirtualGoodsFulfillmentOutcome, CommerceServiceError> {
        // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
        #[allow(unreachable_patterns)]
        let pool = match &self.order_pool {
            DatabasePool::Postgres(pool, _) => pool,
            _ => panic!("authoritative persistence requires a PostgreSQL pool (DATABASE_SPEC: authoritative-server)"),
        };
        let replayed = fulfill_postgres(pool, &request).await?;
        Ok(VirtualGoodsFulfillmentOutcome {
            accepted: true,
            replayed,
            fulfillment_status: "fulfilled".to_owned(),
        })
    }
}

impl VirtualGoodsFulfillmentPort for VirtualFulfillmentAdapter {
    fn fulfill_paid_virtual_order<'a>(
        &'a self,
        request: FulfillPaidVirtualOrderRequest,
    ) -> VirtualGoodsFuture<'a, VirtualGoodsFulfillmentOutcome> {
        Box::pin(async move { self.fulfill(request).await })
    }
}

async fn fulfill_postgres(
    pool: &sqlx::PgPool,
    request: &FulfillPaidVirtualOrderRequest,
) -> Result<bool, CommerceServiceError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(store_error("begin virtual fulfillment"))?;
    let fulfillment_id = fulfillment_id(&request.order_id);
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM commerce_fulfillment_order WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&request.tenant_id)
    .bind(&fulfillment_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(store_error("load virtual fulfillment replay"))?;
    if existing > 0 {
        tx.commit()
            .await
            .map_err(store_error("commit virtual fulfillment replay"))?;
        return Ok(true);
    }
    let order = sqlx::query(
        "SELECT organization_id, owner_user_id, payment_status, status \
         FROM commerce_order WHERE tenant_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(&request.tenant_id)
    .bind(&request.order_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(store_error("load paid virtual order"))?
    .ok_or_else(|| CommerceServiceError::not_found("virtual order was not found"))?;
    let stored_owner_user_id = order
        .try_get::<Option<String>, _>("owner_user_id")
        .ok()
        .flatten()
        .unwrap_or_default();
    let payment_status = order
        .try_get::<Option<String>, _>("payment_status")
        .ok()
        .flatten()
        .unwrap_or_default();
    validate_paid_order(&stored_owner_user_id, &request.owner_user_id, &payment_status)?;
    let order_status = order
        .try_get::<Option<String>, _>("status")
        .ok()
        .flatten()
        .unwrap_or_default();
    if matches!(
        order_status.trim().to_ascii_lowercase().as_str(),
        "cancelled" | "canceled" | "closed" | "expired"
    ) {
        // A negative terminal order must never be turned into a delivery: the
        // settlement layer reports `late_payment_requires_recovery` for that
        // case and the entitlement is issued by the recovery flow.
        return Err(CommerceServiceError::invalid_state(
            "virtual order reached a negative terminal state before delivery",
        ));
    }
    let order_organization_id = order
        .try_get::<Option<String>, _>("organization_id")
        .ok()
        .flatten()
        .unwrap_or_else(|| "0".to_string());
    let now = sdkwork_order_service::canonical_now_timestamp();
    sqlx::query(
        "INSERT INTO commerce_fulfillment_order \
         (id, tenant_id, organization_id, fulfillment_no, order_id, fulfillment_type, status, \
          address_snapshot_id, provider_code, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, 'virtual_delivery', 'fulfilled', NULL, 'merchant', $6, $6)",
    )
    .bind(&fulfillment_id)
    .bind(&request.tenant_id)
    .bind(&order_organization_id)
    .bind(&fulfillment_id)
    .bind(&request.order_id)
    .bind(&now)
    .execute(&mut *tx)
    .await
    .map_err(store_error("create virtual fulfillment"))?;
    sqlx::query(
        "UPDATE commerce_order SET status = 'fulfilled', fulfillment_status = 'fulfilled', \
         updated_at = CAST($1 AS TIMESTAMPTZ) \
         WHERE tenant_id = $2 AND id = $3 AND organization_id = $4 \
           AND payment_status IN ('success', 'succeeded', 'paid')",
    )
    .bind(&now)
    .bind(&request.tenant_id)
    .bind(&request.order_id)
    .bind(&order_organization_id)
    .execute(&mut *tx)
    .await
    .map_err(store_error("advance virtual order fulfillment"))?;
    sqlx::query(
        "UPDATE commerce_order_item SET fulfillment_status = 'fulfilled' \
         WHERE tenant_id = $1 AND order_id = $2 AND organization_id = $3",
    )
    .bind(&request.tenant_id)
    .bind(&request.order_id)
    .bind(&order_organization_id)
    .execute(&mut *tx)
    .await
    .map_err(store_error("advance virtual order items"))?;
    tx.commit()
        .await
        .map_err(store_error("commit virtual fulfillment"))?;
    Ok(false)
}

fn validate_paid_order(
    stored_owner: &str,
    expected_owner: &str,
    payment_status: &str,
) -> Result<(), CommerceServiceError> {
    if stored_owner != expected_owner {
        return Err(CommerceServiceError::not_found(
            "virtual order was not found",
        ));
    }
    if !matches!(
        payment_status.trim().to_ascii_lowercase().as_str(),
        "success" | "succeeded" | "paid"
    ) {
        return Err(CommerceServiceError::invalid_state(
            "virtual order payment is not successful",
        ));
    }
    Ok(())
}

fn fulfillment_id(order_id: &str) -> String {
    format!("virtual-fulfillment-{order_id}")
}

fn store_error(message: &'static str) -> impl FnOnce(sqlx::Error) -> CommerceServiceError {
    move |error| CommerceServiceError::storage(format!("{message}: {error}"))
}
