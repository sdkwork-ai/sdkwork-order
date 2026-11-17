//! Paid-order commission intake for the partner distribution capability.
//!
//! The partner domain owns the commission ledger; the order domain only reports a
//! paid order's revenue. Settlement later resolves the customer's active partner
//! binding and allocates the differential (level) commission, so a paid order from
//! an unbound customer is a harmless SKIPPED event.

use chrono::{DateTime, Utc};
use sdkwork_commerce_partner_repository_sqlx::order_intake::record_and_settle_order_commission;
use sdkwork_commerce_partner_service::domain::parse_money_to_cents;
use sdkwork_contract_service::CommerceServiceError;
use sqlx::PgPool;

const SELECT_ACTIVE_BINDING: &str = r#"
SELECT EXISTS(
    SELECT 1 FROM partner_customer_binding
    WHERE tenant_id = $1 AND organization_id = $2 AND customer_user_id = $3
      AND status = 'ACTIVE'
) AS bound
"#;

const SELECT_ORDER_REVENUE: &str = r#"
SELECT CAST(COALESCE((
        SELECT b.payable_amount
        FROM commerce_order_amount_breakdown b
        WHERE b.tenant_id = CAST($1 AS TEXT)
          AND b.order_id = CAST($2 AS TEXT)
          AND b.allocation_type = 'order_total'
        LIMIT 1
    ), 0) AS TEXT) AS revenue
"#;

/// Records the revenue of a genuinely paid order as a PENDING commission event.
///
/// Returns the event id, or `None` when the order has no numeric tenant/customer
/// identity (nothing to attribute) — the payment itself is unaffected.
pub async fn record_paid_order_commission(
    pool: &PgPool,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    order_id: &str,
    paid_at: &str,
) -> Result<Option<i64>, CommerceServiceError> {
    let Ok(tenant_id) = tenant_id.trim().parse::<i64>() else {
        return Ok(None);
    };
    let organization_id = organization_id
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(0);
    let Ok(customer_user_id) = owner_user_id.trim().parse::<i64>() else {
        return Ok(None);
    };
    // Orders from unbound customers cannot earn commission: skip the event
    // entirely instead of queuing one that settlement would only mark SKIPPED.
    let bound = sqlx::query_scalar::<_, bool>(SELECT_ACTIVE_BINDING)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(customer_user_id)
        .fetch_one(pool)
        .await
        .map_err(|error| {
            CommerceServiceError::storage(format!("failed to resolve partner binding: {error}"))
        })?;
    if !bound {
        return Ok(None);
    }
    let revenue = sqlx::query_scalar::<_, String>(SELECT_ORDER_REVENUE)
        .bind(tenant_id.to_string())
        .bind(order_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| {
            CommerceServiceError::storage(format!("failed to load order revenue: {error}"))
        })?
        .unwrap_or_else(|| "0".to_owned());
    let revenue_cents = parse_money_to_cents("revenue", &revenue)?;
    let event_at = DateTime::parse_from_rfc3339(paid_at.trim())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    // Settle a bounded batch on the payment path so the buyer's distributor sees
    // the commission without waiting for an admin settlement run.
    let (event_id, _outcome) = record_and_settle_order_commission(
        pool,
        tenant_id,
        organization_id,
        order_id,
        customer_user_id,
        revenue_cents,
        event_at,
        20,
    )
    .await?;
    Ok(Some(event_id))
}
