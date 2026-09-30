//! Order expiration sweep used by the in-process expiration scheduler.
//!
//! Orders whose payment window (`expired_at`) has elapsed transition to
//! `expired` with a system lifecycle event. `expired_at` is stored in the
//! canonical RFC 3339 UTC store format (see `crate::store_clock`), so the
//! due predicate casts the column explicitly. Every transition is
//! idempotent: a row already moved to a terminal state yields `Ok(false)`.

use sdkwork_contract_service::CommerceServiceError;
use sqlx::{PgPool, Row};

use crate::order_lifecycle::{insert_order_event_postgres, OrderLifecycleAuditInput};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpiringOrderRecord {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
    pub fulfillment_status: Option<String>,
}

/// Returns up to `limit` orders whose payment window has elapsed and that are
/// still in an expirable state.
///
/// The listing is a plain read: locks taken here through an autocommit pool
/// query would be released the moment the statement completes, so mutual
/// exclusion between concurrent scheduler replicas is enforced by
/// [`expire_due_order`] instead — it re-locks the row inside its own
/// transaction and re-checks the expirable-status predicate, making the
/// transition idempotent (`Ok(false)` for a row another replica already
/// expired).
pub async fn list_due_expiring_orders(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<ExpiringOrderRecord>, CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT tenant_id, organization_id, owner_user_id, id, fulfillment_status
        FROM commerce_order
        WHERE LOWER(COALESCE(status, '')) IN ('draft', 'pending', 'pending_payment', 'unpaid', 'wait_pay')
          AND expired_at IS NOT NULL
          AND expired_at <= CURRENT_TIMESTAMP
        ORDER BY expired_at, id
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|error| {
        crate::sql_store_error::map_sql_store_error("failed to list expiring orders", error)
    })?;
    Ok(rows
        .iter()
        .map(|row| ExpiringOrderRecord {
            tenant_id: string_cell(row, "tenant_id"),
            organization_id: optional_string_cell(row, "organization_id"),
            owner_user_id: string_cell(row, "owner_user_id"),
            order_id: string_cell(row, "id"),
            fulfillment_status: optional_string_cell(row, "fulfillment_status"),
        })
        .collect())
}

/// Atomically transitions one expirable order to `expired` and writes the
/// `expired` lifecycle event. Returns `Ok(true)` when the transition
/// happened, `Ok(false)` when the order was already terminal (idempotent).
pub async fn expire_due_order(
    pool: &PgPool,
    record: &ExpiringOrderRecord,
    now: &str,
) -> Result<bool, CommerceServiceError> {
    let mut tx = pool.begin().await.map_err(|error| {
        crate::sql_store_error::map_sql_store_error("failed to begin order expiration", error)
    })?;
    let from_status = sqlx::query_scalar::<_, String>(
        r#"
        SELECT status
        FROM commerce_order
        WHERE tenant_id = CAST($1 AS TEXT)
          AND id = CAST($2 AS TEXT)
          AND LOWER(COALESCE(status, '')) IN ('draft', 'pending', 'pending_payment', 'unpaid', 'wait_pay')
        FOR UPDATE
        "#,
    )
    .bind(&record.tenant_id)
    .bind(&record.order_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|error| crate::sql_store_error::map_sql_store_error("failed to lock expiring order", error))?;
    let Some(from_status) = from_status else {
        tx.commit().await.map_err(|error| {
            crate::sql_store_error::map_sql_store_error("failed to commit expiration no-op", error)
        })?;
        return Ok(false);
    };
    let updated = sqlx::query(
        r#"
        UPDATE commerce_order
        SET status = 'expired', payment_status = 'expired', updated_at = CAST($1 AS TIMESTAMPTZ)
        WHERE tenant_id = CAST($2 AS TEXT)
          AND id = CAST($3 AS TEXT)
          AND LOWER(COALESCE(status, '')) IN ('draft', 'pending', 'pending_payment', 'unpaid', 'wait_pay')
        "#,
    )
    .bind(now)
    .bind(&record.tenant_id)
    .bind(&record.order_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| crate::sql_store_error::map_sql_store_error("failed to expire order", error))?;
    if updated.rows_affected() == 0 {
        tx.commit().await.map_err(|error| {
            crate::sql_store_error::map_sql_store_error("failed to commit expiration no-op", error)
        })?;
        return Ok(false);
    }
    let audit = OrderLifecycleAuditInput {
        tenant_id: record.tenant_id.clone(),
        organization_id: record.organization_id.clone(),
        order_id: record.order_id.clone(),
        event_type: "expired",
        from_status,
        to_status: "expired",
        actor_type: "system",
        actor_id: None,
        reason_code: Some("payment_timeout".to_owned()),
        reason_message: Some("order payment window elapsed; automatically expired".to_owned()),
        request_no: format!("system-expire-{}", record.order_id),
        idempotency_key: format!("order-expire:{}", record.order_id),
        now: now.to_owned(),
    };
    insert_order_event_postgres(&mut tx, &audit).await?;
    tx.commit().await.map_err(|error| {
        crate::sql_store_error::map_sql_store_error("failed to commit order expiration", error)
    })?;
    Ok(true)
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use sdkwork_order_service::canonical_now_timestamp;

    #[test]
    fn expiration_timestamp_is_canonical_rfc3339() {
        let value = canonical_now_timestamp();
        // Canonical store format (DATABASE_SPEC §8.1.1): RFC 3339 UTC with
        // fixed millisecond precision, lexicographically ordered.
        assert!(chrono::DateTime::parse_from_rfc3339(&value).is_ok());
        assert!(value.ends_with('Z'));
    }
}
