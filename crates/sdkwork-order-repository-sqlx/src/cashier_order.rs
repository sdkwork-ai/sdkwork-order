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

/// Trade numbers a legacy (token-less) cashier link may carry.
///
/// A legacy link is `{base}/cashier/{orderId}?scene=..&outTradeNo=..`; the page
/// exchanges it for a signed capability token so the payer does not need a
/// session. The exchange must prove the caller saw the QR, which means matching
/// the URL's `outTradeNo` against a value only the order owner's checkout could
/// have produced:
///
/// - `request_no` holds the membership/app-template out-trade number,
/// - `order_no` is the fallback used by the cashier read model, and
/// - the latest payment attempt holds the provider out-trade number used by the
///   standard checkout flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashierLinkOrderRecord {
    /// Tenant that owns the order.
    pub tenant_id: String,
    /// Merchant trade number stored on the order row.
    pub request_no: String,
    /// Business order number.
    pub order_no: String,
    /// Provider trade number of the latest payment attempt, when one exists.
    pub attempt_out_trade_no: Option<String>,
}

impl CashierLinkOrderRecord {
    /// Returns the stored trade number that matches `candidate`.
    ///
    /// The comparison is constant-time so the endpoint cannot be used as a
    /// timing oracle to discover an order's out-trade number.
    #[must_use]
    pub fn matched_out_trade_no(&self, candidate: &str) -> Option<&str> {
        let candidate = candidate.trim();
        if candidate.is_empty() {
            return None;
        }
        [
            Some(self.request_no.as_str()),
            Some(self.order_no.as_str()),
            self.attempt_out_trade_no.as_deref(),
        ]
        .into_iter()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .find(|value| sdkwork_utils_rust::secure_compare(value.trim(), candidate))
    }

    /// True when `candidate` matches any trade number bound to this order.
    #[must_use]
    pub fn matches_out_trade_no(&self, candidate: &str) -> bool {
        self.matched_out_trade_no(candidate).is_some()
    }
}

/// Loads the trade numbers a legacy cashier link may present for `order_id`.
///
/// The lookup is by order primary key (globally unique), not tenant-scoped: an
/// anonymous caller has no tenant, and the returned tenant is only used after
/// the out-trade number check succeeds.
pub async fn load_cashier_link_order_postgres(
    pool: &PgPool,
    order_id: &str,
) -> Result<Option<CashierLinkOrderRecord>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT
            o.tenant_id,
            COALESCE(NULLIF(o.request_no, ''), '') AS request_no,
            COALESCE(NULLIF(o.order_no, ''), '') AS order_no,
            (
                SELECT NULLIF(pa.out_trade_no, '')
                FROM commerce_payment_attempt pa
                WHERE pa.tenant_id = o.tenant_id
                  AND pa.order_id = o.id
                ORDER BY pa.created_at DESC
                LIMIT 1
            ) AS attempt_out_trade_no
        FROM commerce_order o
        WHERE o.id = CAST($1 AS TEXT)
        LIMIT 1
        "#,
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        CommerceServiceError::storage(format!("failed to load cashier link order: {error}"))
    })?;

    let Some(row) = row else {
        return Ok(None);
    };

    let tenant_id = string_cell(&row, "tenant_id");
    if tenant_id.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(CashierLinkOrderRecord {
        tenant_id,
        request_no: string_cell(&row, "request_no"),
        order_no: string_cell(&row, "order_no"),
        attempt_out_trade_no: row
            .try_get::<Option<String>, _>("attempt_out_trade_no")
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

    fn link_record() -> CashierLinkOrderRecord {
        CashierLinkOrderRecord {
            tenant_id: "100001".to_owned(),
            request_no: "MEMBERSHIP-1".to_owned(),
            order_no: "MB-1".to_owned(),
            attempt_out_trade_no: Some("PROVIDER-1".to_owned()),
        }
    }

    #[test]
    fn legacy_link_accepts_every_bound_trade_number() {
        let link = link_record();
        assert!(link.matches_out_trade_no("MEMBERSHIP-1"));
        assert!(link.matches_out_trade_no("MB-1"));
        assert!(link.matches_out_trade_no("PROVIDER-1"));
        assert!(link.matches_out_trade_no("  MEMBERSHIP-1  "));
        // The matched stored value is what the token binds, not the raw request.
        assert_eq!(link.matched_out_trade_no("  MEMBERSHIP-1  "), Some("MEMBERSHIP-1"));
        assert_eq!(link.matched_out_trade_no("PROVIDER-1"), Some("PROVIDER-1"));
    }

    #[test]
    fn legacy_link_rejects_unknown_or_blank_trade_numbers() {
        let link = link_record();
        assert!(!link.matches_out_trade_no("MEMBERSHIP-2"));
        assert!(!link.matches_out_trade_no(""));
        assert!(!link.matches_out_trade_no("   "));

        let without_attempt = CashierLinkOrderRecord {
            attempt_out_trade_no: None,
            ..link_record()
        };
        assert!(without_attempt.matches_out_trade_no("MEMBERSHIP-1"));
        assert!(!without_attempt.matches_out_trade_no("PROVIDER-1"));
    }
}
