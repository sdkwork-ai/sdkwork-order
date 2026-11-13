//! Anonymous cashier read model.
//!
//! The no-login cashier must render a payable order without an authenticated
//! principal. This module loads exactly the fields that surface is allowed to
//! show, plus the ownership facts the payment command needs.
//!
//! Ownership facts (`owner_user_id`, `organization_id`) are loaded so the
//! anonymous payment path can act *on the order's behalf* — it never widens what
//! the caller may see. They are deliberately kept out of the wire DTO in
//! `cashier_router`.

use sdkwork_contract_service::CommerceServiceError;
use sqlx::{PgPool, Row};

/// A payable order resolved from a cashier capability token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashierOrderRecord {
    /// Order primary key.
    pub order_id: String,
    /// Business order number.
    pub order_sn: String,
    /// Order subject shown on the cashier.
    pub subject: String,
    /// Minor-unit payable amount.
    pub amount: String,
    /// ISO-4217 currency code.
    pub currency_code: String,
    /// Total item quantity.
    pub quantity: i64,
    /// Lifecycle status (`pending_payment`, `paid`, ...).
    pub status: String,
    /// ISO-8601 expiry, when the order carries one.
    pub expire_time: Option<String>,
    /// Order owner. Drives the payment command; never serialized.
    pub owner_user_id: String,
    /// Order organization scope. Drives the payment command; never serialized.
    pub organization_id: Option<String>,
}

impl CashierOrderRecord {
    /// True when the order is in a state the cashier may charge.
    ///
    /// Only `pending_payment` is payable: `draft`/`pending` orders were never
    /// submitted, and every other state is either paid, closed, or refunded.
    #[must_use]
    pub fn is_payable(&self) -> bool {
        self.status.trim().eq_ignore_ascii_case("pending_payment")
    }
}

/// Loads a cashier order by tenant + order id.
///
/// The lookup is intentionally tenant-scoped and **not** owner-scoped: the caller
/// is authenticated by the cashier capability token, not by a user session.
pub async fn load_cashier_order_postgres(
    pool: &PgPool,
    tenant_id: &str,
    order_id: &str,
) -> Result<Option<CashierOrderRecord>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT
            o.id,
            o.order_no,
            o.status,
            o.owner_user_id,
            NULLIF(o.organization_id, '') AS organization_id,
            COALESCE(NULLIF(o.subject, ''), '') AS subject,
            COALESCE(NULLIF(o.currency_code, ''), 'CNY') AS currency_code,
            TO_CHAR(o.expired_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS expire_time,
            COALESCE(
                (
                    SELECT CAST(b.payable_amount AS TEXT)
                    FROM commerce_order_amount_breakdown b
                    WHERE b.tenant_id = o.tenant_id
                      AND b.order_id = o.id
                      AND b.allocation_type = 'order_total'
                    LIMIT 1
                ),
                '0'
            ) AS total_amount,
            COALESCE(
                (
                    SELECT SUM(oi.quantity)
                    FROM commerce_order_item oi
                    WHERE oi.tenant_id = o.tenant_id
                      AND oi.order_id = o.id
                ),
                1
            ) AS quantity
        FROM commerce_order o
        WHERE o.tenant_id = CAST($1 AS TEXT)
          AND o.id = CAST($2 AS TEXT)
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        CommerceServiceError::storage(format!("failed to load cashier order: {error}"))
    })?;

    let Some(row) = row else {
        return Ok(None);
    };

    Ok(Some(CashierOrderRecord {
        order_id: string_cell(&row, "id"),
        order_sn: string_cell(&row, "order_no"),
        subject: string_cell(&row, "subject"),
        amount: string_cell(&row, "total_amount"),
        currency_code: string_cell(&row, "currency_code"),
        quantity: row.try_get::<i64, _>("quantity").unwrap_or(1),
        status: string_cell(&row, "status"),
        expire_time: row.try_get::<Option<String>, _>("expire_time").ok().flatten(),
        owner_user_id: string_cell(&row, "owner_user_id"),
        organization_id: row
            .try_get::<Option<String>, _>("organization_id")
            .ok()
            .flatten(),
    }))
}

/// Reads a text column, defaulting to an empty string.
fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    row.try_get::<String, _>(column).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(status: &str) -> CashierOrderRecord {
        CashierOrderRecord {
            order_id: "ORD-1".to_owned(),
            order_sn: "SW1".to_owned(),
            subject: "Membership".to_owned(),
            amount: "6990".to_owned(),
            currency_code: "CNY".to_owned(),
            quantity: 1,
            status: status.to_owned(),
            expire_time: None,
            owner_user_id: "user-1".to_owned(),
            organization_id: Some("0".to_owned()),
        }
    }

    #[test]
    fn only_pending_payment_is_payable() {
        assert!(record("pending_payment").is_payable());
        assert!(record("PENDING_PAYMENT").is_payable());
        assert!(!record("paid").is_payable());
        assert!(!record("draft").is_payable());
        assert!(!record("cancelled").is_payable());
        assert!(!record("expired").is_payable());
    }
}
