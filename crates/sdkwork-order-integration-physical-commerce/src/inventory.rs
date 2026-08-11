use std::collections::HashSet;

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service::{
    PhysicalInventoryMutationOutcome, PhysicalInventoryReservationPort, PhysicalPurchaseFuture,
    ReleasePhysicalOrderInventoryRequest, ReservePhysicalOrderInventoryRequest,
};
use sqlx::{Postgres, Row, Transaction};

#[derive(Clone)]
pub struct PhysicalInventoryAdapter {
    pool: DatabasePool,
}

impl PhysicalInventoryAdapter {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

impl PhysicalInventoryReservationPort for PhysicalInventoryAdapter {
    fn reserve_physical_order_inventory<'a>(
        &'a self,
        request: ReservePhysicalOrderInventoryRequest,
    ) -> PhysicalPurchaseFuture<'a, PhysicalInventoryMutationOutcome> {
        Box::pin(async move {
            // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
            let DatabasePool::Postgres(pool, _) = &self.pool else {
                panic!("physical inventory reservation requires a PostgreSQL pool");
            };
            reserve_postgres(pool, &request).await
        })
    }

    fn release_physical_order_inventory<'a>(
        &'a self,
        request: ReleasePhysicalOrderInventoryRequest,
    ) -> PhysicalPurchaseFuture<'a, PhysicalInventoryMutationOutcome> {
        Box::pin(async move {
            // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
            let DatabasePool::Postgres(pool, _) = &self.pool else {
                panic!("physical inventory release requires a PostgreSQL pool");
            };
            release_postgres(pool, &request).await
        })
    }

    fn restock_consumed_order_inventory<'a>(
        &'a self,
        request: ReleasePhysicalOrderInventoryRequest,
    ) -> PhysicalPurchaseFuture<'a, PhysicalInventoryMutationOutcome> {
        Box::pin(async move {
            // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
            let DatabasePool::Postgres(pool, _) = &self.pool else {
                panic!("physical inventory restock requires a PostgreSQL pool");
            };
            restock_postgres(pool, &request).await
        })
    }

    fn sweep_expired_inventory_reservations<'a>(
        &'a self,
        limit: i64,
    ) -> PhysicalPurchaseFuture<'a, i64> {
        let pool = self.pool.clone();
        Box::pin(async move {
            // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
            let DatabasePool::Postgres(pool, _) = &pool else {
                panic!("physical inventory sweep requires a PostgreSQL pool");
            };
            sweep_expired_reservations_postgres(pool, limit).await
        })
    }
}

pub(crate) async fn consume_order_inventory(
    pool: &DatabasePool,
    tenant_id: &str,
    order_id: &str,
    idempotency_key: &str,
) -> Result<PhysicalInventoryMutationOutcome, CommerceServiceError> {
    // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
    let DatabasePool::Postgres(pool, _) = pool else {
        panic!("physical inventory consume requires a PostgreSQL pool");
    };
    let mut tx = pool
            .begin()
            .await
            .map_err(store_error("begin inventory consume"))?;
        let rows = sqlx::query(
            "SELECT id, tenant_id, organization_id, sku_id, warehouse_id, fulfillment_node_id, quantity, status FROM commerce_inventory_reservation WHERE tenant_id = $1 AND order_id = $2 ORDER BY id FOR UPDATE",
        )
        .bind(tenant_id).bind(order_id).fetch_all(&mut *tx).await
        .map_err(store_error("load inventory reservations for consume"))?;
        if rows.is_empty() {
            return Err(CommerceServiceError::invalid_state(
                "physical order has no inventory reservation",
            ));
        }
        let replayed = rows
            .iter()
            .all(|row| text_postgres(row, "status").eq_ignore_ascii_case("consumed"));
        if !replayed {
            for row in &rows {
                let status = text_postgres(row, "status");
                if status.eq_ignore_ascii_case("consumed") {
                    continue;
                }
                if !status.eq_ignore_ascii_case("reserved") {
                    return Err(CommerceServiceError::invalid_state(
                        "inventory reservation cannot be consumed",
                    ));
                }
                consume_stock_postgres(&mut tx, row).await?;
                sqlx::query("UPDATE commerce_inventory_reservation SET status = 'consumed', consumed_quantity = quantity, consumed_at = $1, updated_at = $2, idempotency_key = $3 WHERE id = $4 AND status = 'reserved'")
                    .bind(now_string()).bind(now_string()).bind(idempotency_key)
                    .bind(text_postgres(row, "id"))
                    .execute(&mut *tx).await.map_err(store_error("consume inventory reservation"))?;
            }
        }
        tx.commit()
            .await
            .map_err(store_error("commit inventory consume"))?;
        Ok(PhysicalInventoryMutationOutcome {
            accepted: true,
            replayed,
        })
}


async fn reserve_postgres(
    pool: &sqlx::PgPool,
    request: &ReservePhysicalOrderInventoryRequest,
) -> Result<PhysicalInventoryMutationOutcome, CommerceServiceError> {
    validate_reserve_request(request)?;
    let mut tx = pool
        .begin()
        .await
        .map_err(store_error("begin inventory reservation"))?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!(
            "physical-inventory:{}:{}",
            request.tenant_id, request.order_id
        ))
        .execute(&mut *tx)
        .await
        .map_err(store_error("lock physical inventory reservation"))?;
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM commerce_inventory_reservation WHERE tenant_id = $1 AND order_id = $2")
        .bind(&request.tenant_id).bind(&request.order_id).fetch_one(&mut *tx).await
        .map_err(store_error("count inventory reservations"))?;
    if existing > 0 {
        validate_reservation_replay_postgres(&mut tx, request, existing).await?;
        tx.commit()
            .await
            .map_err(store_error("commit inventory reservation replay"))?;
        return Ok(PhysicalInventoryMutationOutcome {
            accepted: true,
            replayed: true,
        });
    }
    for line in &request.lines {
        let stock = sqlx::query("SELECT id, warehouse_id, fulfillment_node_id FROM commerce_inventory_stock WHERE tenant_id = $1 AND organization_id = $2 AND shop_id = $3 AND sku_id = $4 AND status = 'active' AND available_quantity - safety_stock_quantity >= $5 ORDER BY available_quantity DESC, id LIMIT 1 FOR UPDATE")
            .bind(&request.tenant_id).bind(&request.merchant_organization_id).bind(&line.shop_id)
            .bind(&line.sku_id).bind(line.quantity).fetch_optional(&mut *tx).await
            .map_err(store_error("select reservable inventory stock"))?
            .ok_or_else(|| CommerceServiceError::conflict("physical SKU inventory is insufficient"))?;
        let stock_id = text_postgres(&stock, "id");
        let updated = sqlx::query("UPDATE commerce_inventory_stock SET available_quantity = available_quantity - $1, reserved_quantity = reserved_quantity + $1, version = version + 1, updated_at = $2 WHERE id = $3 AND available_quantity - safety_stock_quantity >= $1")
            .bind(line.quantity).bind(now_string()).bind(&stock_id)
            .execute(&mut *tx).await.map_err(store_error("reserve inventory stock"))?;
        if updated.rows_affected() != 1 {
            return Err(CommerceServiceError::conflict(
                "physical SKU inventory is insufficient",
            ));
        }
        insert_reservation_postgres(&mut tx, request, line, &stock).await?;
    }
    tx.commit()
        .await
        .map_err(store_error("commit inventory reservation"))?;
    Ok(PhysicalInventoryMutationOutcome {
        accepted: true,
        replayed: false,
    })
}


async fn release_postgres(
    pool: &sqlx::PgPool,
    request: &ReleasePhysicalOrderInventoryRequest,
) -> Result<PhysicalInventoryMutationOutcome, CommerceServiceError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(store_error("begin inventory release"))?;
    let rows = sqlx::query("SELECT id, tenant_id, organization_id, sku_id, warehouse_id, fulfillment_node_id, quantity, status FROM commerce_inventory_reservation WHERE tenant_id = $1 AND order_id = $2 ORDER BY id FOR UPDATE")
        .bind(&request.tenant_id).bind(&request.order_id).fetch_all(&mut *tx).await.map_err(store_error("load inventory reservations for release"))?;
    if rows.is_empty() {
        return Ok(PhysicalInventoryMutationOutcome {
            accepted: true,
            replayed: true,
        });
    }
    let replayed = rows.iter().all(|row| {
        matches!(
            text_postgres(row, "status").as_str(),
            "released" | "consumed"
        )
    });
    if !replayed {
        for row in &rows {
            let status = text_postgres(row, "status");
            if matches!(status.as_str(), "released" | "consumed") {
                // Released rows are already returned to stock; consumed rows
                // are fulfilled out of stock and must not be released again.
                continue;
            }
            if !status.eq_ignore_ascii_case("reserved") {
                return Err(CommerceServiceError::invalid_state(
                    "inventory reservation cannot be released",
                ));
            }
            release_stock_postgres(&mut tx, row).await?;
            sqlx::query("UPDATE commerce_inventory_reservation SET status = 'released', released_quantity = quantity, release_reason_code = $1, released_at = $2, updated_at = $3, idempotency_key = $4 WHERE id = $5 AND status = 'reserved'")
                .bind(&request.reason_code).bind(now_string()).bind(now_string()).bind(&request.idempotency_key).bind(text_postgres(row, "id"))
                .execute(&mut *tx).await.map_err(store_error("release inventory reservation"))?;
        }
    }
    tx.commit()
        .await
        .map_err(store_error("commit inventory release"))?;
    Ok(PhysicalInventoryMutationOutcome {
        accepted: true,
        replayed,
    })
}

/// Returns consumed stock to available stock after a physical return is
/// completed. `consumed` reservations become `restocked`; released/restocked
/// reservations are skipped so retries never double-credit the stock.
async fn restock_postgres(
    pool: &sqlx::PgPool,
    request: &ReleasePhysicalOrderInventoryRequest,
) -> Result<PhysicalInventoryMutationOutcome, CommerceServiceError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(store_error("begin inventory restock"))?;
    let rows = sqlx::query("SELECT id, tenant_id, organization_id, sku_id, warehouse_id, fulfillment_node_id, quantity, status FROM commerce_inventory_reservation WHERE tenant_id = $1 AND order_id = $2 ORDER BY id FOR UPDATE")
        .bind(&request.tenant_id).bind(&request.order_id).fetch_all(&mut *tx).await.map_err(store_error("load inventory reservations for restock"))?;
    if rows.is_empty() {
        return Ok(PhysicalInventoryMutationOutcome {
            accepted: true,
            replayed: true,
        });
    }
    let replayed = rows.iter().all(|row| {
        matches!(
            text_postgres(row, "status").as_str(),
            "restocked" | "released"
        )
    });
    if !replayed {
        for row in &rows {
            let status = text_postgres(row, "status");
            if matches!(status.as_str(), "restocked" | "released") {
                // Already returned to stock by an earlier restock or release.
                continue;
            }
            if !status.eq_ignore_ascii_case("consumed") {
                return Err(CommerceServiceError::invalid_state(
                    "inventory reservation is not consumed; cannot restock",
                ));
            }
            restock_stock_postgres(&mut tx, row).await?;
            sqlx::query("UPDATE commerce_inventory_reservation SET status = 'restocked', updated_at = $1, idempotency_key = $2 WHERE id = $3 AND status = 'consumed'")
                .bind(now_string()).bind(&request.idempotency_key).bind(text_postgres(row, "id"))
                .execute(&mut *tx).await.map_err(store_error("restock inventory reservation"))?;
        }
    }
    tx.commit()
        .await
        .map_err(store_error("commit inventory restock"))?;
    Ok(PhysicalInventoryMutationOutcome {
        accepted: true,
        replayed,
    })
}

/// Releases every reservation whose `expires_at` (unix seconds, set at
/// reserve time) has elapsed while still `reserved`. This is the consistency
/// backstop for failed releases, legacy orders without `expired_at`, and
/// abandoned payment windows. Returns the number of affected orders.
async fn sweep_expired_reservations_postgres(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<i64, CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT tenant_id, order_id
        FROM commerce_inventory_reservation
        WHERE status = 'reserved'
          AND NULLIF(expires_at, '')::bigint <= $1::bigint
        ORDER BY order_id
        LIMIT $2
        "#,
    )
    .bind(now_seconds())
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(store_error("load expired inventory reservations for sweep"))?;
    let mut swept = 0;
    for row in rows {
        let request = ReleasePhysicalOrderInventoryRequest {
            tenant_id: text_postgres(&row, "tenant_id"),
            order_id: text_postgres(&row, "order_id"),
            reason_code: "reservation_expired".to_owned(),
            request_no: format!("sweep-{}", text_postgres(&row, "order_id")),
            idempotency_key: format!("sweep:{}", text_postgres(&row, "order_id")),
        };
        // release is idempotent; concurrent sweepers converge on released.
        if let Err(error) = release_postgres(pool, &request).await {
            tracing::warn!(
                target = "inventory.sweep",
                order_id = %request.order_id,
                error = ?error,
                "failed to release expired inventory reservation"
            );
            continue;
        }
        swept += 1;
    }
    Ok(swept)
}

fn validate_reserve_request(
    request: &ReservePhysicalOrderInventoryRequest,
) -> Result<(), CommerceServiceError> {
    if request.tenant_id.trim().is_empty()
        || request.merchant_organization_id.trim().is_empty()
        || request.order_id.trim().is_empty()
        || request.request_no.trim().is_empty()
        || request.idempotency_key.trim().is_empty()
        || request.lines.is_empty()
        || request.lines.iter().any(|line| {
            line.quantity <= 0 || line.sku_id.trim().is_empty() || line.shop_id.trim().is_empty()
        })
    {
        return Err(CommerceServiceError::validation(
            "physical inventory reservation lines are invalid",
        ));
    }
    let mut sku_ids = HashSet::with_capacity(request.lines.len());
    if request
        .lines
        .iter()
        .any(|line| !sku_ids.insert(line.sku_id.trim()))
    {
        return Err(CommerceServiceError::validation(
            "physical inventory reservation contains duplicate SKU lines",
        ));
    }
    Ok(())
}


async fn validate_reservation_replay_postgres(
    tx: &mut Transaction<'_, Postgres>,
    request: &ReservePhysicalOrderInventoryRequest,
    existing: i64,
) -> Result<(), CommerceServiceError> {
    let rows = sqlx::query("SELECT organization_id, sku_id, quantity, idempotency_key, status FROM commerce_inventory_reservation WHERE tenant_id = $1 AND order_id = $2 ORDER BY sku_id FOR UPDATE")
        .bind(&request.tenant_id).bind(&request.order_id).fetch_all(&mut **tx).await.map_err(store_error("validate inventory reservation replay"))?;
    if !reservation_replay_matches_postgres(&rows, request, existing) {
        return Err(CommerceServiceError::conflict(
            "inventory reservation replay does not match the original order",
        ));
    }
    Ok(())
}


async fn insert_reservation_postgres(
    tx: &mut Transaction<'_, Postgres>,
    request: &ReservePhysicalOrderInventoryRequest,
    line: &sdkwork_order_service::PhysicalInventoryLine,
    stock: &sqlx::postgres::PgRow,
) -> Result<(), CommerceServiceError> {
    let id = reservation_id(&request.order_id, &line.sku_id);
    sqlx::query("INSERT INTO commerce_inventory_reservation (id, tenant_id, organization_id, reservation_no, order_id, reservation_source_type, reservation_source_id, reservation_type, sku_id, warehouse_id, fulfillment_node_id, quantity, reserved_quantity, consumed_quantity, released_quantity, status, request_no, idempotency_key, expires_at, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, 'order', $6, 'sale', $7, $8, $9, $10, $10, 0, 0, 'reserved', $11, $12, $13, $14, $14)")
        .bind(&id).bind(&request.tenant_id).bind(&request.merchant_organization_id).bind(&id).bind(&request.order_id).bind(&request.order_id)
        .bind(&line.sku_id).bind(optional_text_postgres(stock, "warehouse_id")).bind(optional_text_postgres(stock, "fulfillment_node_id"))
        .bind(line.quantity).bind(&request.request_no).bind(&request.idempotency_key).bind(expires_at()).bind(now_string())
        .execute(&mut **tx).await.map_err(store_error("insert inventory reservation"))?;
    Ok(())
}

async fn consume_stock_postgres(
    tx: &mut Transaction<'_, Postgres>,
    row: &sqlx::postgres::PgRow,
) -> Result<(), CommerceServiceError> {
    mutate_stock_postgres(tx, row, false).await
}
async fn release_stock_postgres(
    tx: &mut Transaction<'_, Postgres>,
    row: &sqlx::postgres::PgRow,
) -> Result<(), CommerceServiceError> {
    mutate_stock_postgres(tx, row, true).await
}
async fn restock_stock_postgres(
    tx: &mut Transaction<'_, Postgres>,
    row: &sqlx::postgres::PgRow,
) -> Result<(), CommerceServiceError> {
    let quantity = row.try_get::<i64, _>("quantity").unwrap_or(0);
    sqlx::query("UPDATE commerce_inventory_stock SET available_quantity = available_quantity + $1, sold_quantity = sold_quantity - $1, version = version + 1, updated_at = $2 WHERE tenant_id = $3 AND organization_id = $4 AND sku_id = $5 AND ((warehouse_id = $6) OR (warehouse_id IS NULL AND $6 IS NULL)) AND ((fulfillment_node_id = $7) OR (fulfillment_node_id IS NULL AND $7 IS NULL)) AND sold_quantity >= $1")
        .bind(quantity)
        .bind(now_string())
        .bind(text_postgres(row, "tenant_id"))
        .bind(text_postgres(row, "organization_id"))
        .bind(text_postgres(row, "sku_id"))
        .bind(optional_text_postgres(row, "warehouse_id"))
        .bind(optional_text_postgres(row, "fulfillment_node_id"))
        .execute(&mut **tx)
        .await
        .map_err(store_error("restock sold inventory stock"))?;
    Ok(())
}
async fn mutate_stock_postgres(
    tx: &mut Transaction<'_, Postgres>,
    row: &sqlx::postgres::PgRow,
    release: bool,
) -> Result<(), CommerceServiceError> {
    let quantity = row.try_get::<i64, _>("quantity").unwrap_or(0);
    let sql = if release {
        "UPDATE commerce_inventory_stock SET available_quantity = available_quantity + $1, reserved_quantity = reserved_quantity - $1, version = version + 1, updated_at = $2 WHERE tenant_id = $3 AND organization_id = $4 AND sku_id = $5 AND ((warehouse_id = $6) OR (warehouse_id IS NULL AND $6 IS NULL)) AND ((fulfillment_node_id = $7) OR (fulfillment_node_id IS NULL AND $7 IS NULL)) AND reserved_quantity >= $1"
    } else {
        "UPDATE commerce_inventory_stock SET reserved_quantity = reserved_quantity - $1, sold_quantity = sold_quantity + $1, version = version + 1, updated_at = $2 WHERE tenant_id = $3 AND organization_id = $4 AND sku_id = $5 AND ((warehouse_id = $6) OR (warehouse_id IS NULL AND $6 IS NULL)) AND ((fulfillment_node_id = $7) OR (fulfillment_node_id IS NULL AND $7 IS NULL)) AND reserved_quantity >= $1"
    };
    let result = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(quantity)
        .bind(now_string())
        .bind(text_postgres(row, "tenant_id"))
        .bind(text_postgres(row, "organization_id"))
        .bind(text_postgres(row, "sku_id"))
        .bind(optional_text_postgres(row, "warehouse_id"))
        .bind(optional_text_postgres(row, "fulfillment_node_id"))
        .execute(&mut **tx)
        .await
        .map_err(store_error("mutate reserved inventory stock"))?;
    if result.rows_affected() != 1 {
        return Err(CommerceServiceError::invalid_state(
            "reserved inventory stock is inconsistent",
        ));
    }
    Ok(())
}

fn reservation_id(order_id: &str, sku_id: &str) -> String {
    format!("inventory-reservation-{order_id}-{sku_id}")
}


fn reservation_replay_matches_postgres(
    rows: &[sqlx::postgres::PgRow],
    request: &ReservePhysicalOrderInventoryRequest,
    existing: i64,
) -> bool {
    reservation_replay_matches(
        rows.iter().map(|row| {
            (
                text_postgres(row, "organization_id"),
                text_postgres(row, "sku_id"),
                row.try_get::<i64, _>("quantity").unwrap_or(0),
                text_postgres(row, "idempotency_key"),
                text_postgres(row, "status"),
            )
        }),
        request,
        existing,
    )
}

fn reservation_replay_matches<I>(
    stored: I,
    request: &ReservePhysicalOrderInventoryRequest,
    existing: i64,
) -> bool
where
    I: Iterator<Item = (String, String, i64, String, String)>,
{
    if existing != request.lines.len() as i64 {
        return false;
    }
    let mut stored = stored.collect::<Vec<_>>();
    stored.sort_by(|left, right| left.1.cmp(&right.1));
    let mut requested = request
        .lines
        .iter()
        .map(|line| (line.sku_id.trim(), line.quantity))
        .collect::<Vec<_>>();
    requested.sort_unstable_by(|left, right| left.0.cmp(right.0));

    stored.len() == requested.len()
        && stored.iter().zip(requested).all(
            |((organization_id, sku_id, quantity, idempotency_key, status), requested)| {
                organization_id == &request.merchant_organization_id
                    && sku_id == requested.0
                    && *quantity == requested.1
                    && idempotency_key == &request.idempotency_key
                    && matches!(status.as_str(), "reserved" | "consumed")
            },
        )
}
fn now_string() -> String {
    now_seconds().to_string()
}
fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs() as i64)
        .unwrap_or(0)
}
fn expires_at() -> String {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0)
        + 1800)
        .to_string()
}
fn text_postgres(row: &sqlx::postgres::PgRow, column: &str) -> String {
    optional_text_postgres(row, column).unwrap_or_default()
}
fn optional_text_postgres(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}
fn store_error(message: &'static str) -> impl FnOnce(sqlx::Error) -> CommerceServiceError {
    move |error| CommerceServiceError::storage(format!("{message}: {error}"))
}

#[cfg(test)]
mod tests;
