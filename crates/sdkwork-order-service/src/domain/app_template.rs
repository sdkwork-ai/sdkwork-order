//! Order-center outcome and wire vocabulary for app-template purchases.
//!
//! A template purchase is an ordinary `commerce_order` whose subject is
//! [`APP_TEMPLATE_ORDER_SUBJECT`] and whose single item carries the template
//! reference in its SKU snapshot. Ownership is therefore the order itself: a
//! settled order (paid + fulfilled) *is* the entitlement, and no module-owned
//! purchase row exists anywhere.

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

/// `commerce_order.subject` for an app-template purchase.
pub const APP_TEMPLATE_ORDER_SUBJECT: &str = "app_template";

/// Status reported for a settled template order (payment collected).
pub const APP_TEMPLATE_ORDER_STATUS_PAID: &str = "paid";
/// Status reported while the buyer still has to pay.
pub const APP_TEMPLATE_ORDER_STATUS_PENDING_PAYMENT: &str = "pending_payment";
/// Status reported for a cancelled/expired template order.
pub const APP_TEMPLATE_ORDER_STATUS_CLOSED: &str = "closed";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateAppTemplateOrderOutcome {
    pub amount: CommerceMoney,
    pub cashier_url: String,
    pub currency_code: String,
    pub expires_at: String,
    pub order_id: String,
    pub order_no: String,
    pub out_trade_no: String,
    pub payment_method: String,
    pub reused: bool,
    pub status: String,
    pub template_name: String,
    pub template_uuid: String,
    /// The published version the purchase snapshot points at; `None` when the
    /// listing carried no published version at order time.
    pub version_uuid: Option<String>,
}

impl CreateAppTemplateOrderOutcome {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        order_id: &str,
        order_no: &str,
        out_trade_no: &str,
        amount: CommerceMoney,
        currency_code: &str,
        template_uuid: &str,
        template_name: &str,
        version_uuid: Option<&str>,
        expires_at: &str,
        payment_method: &str,
        status: &str,
        reused: bool,
        cashier_url: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("order_id", order_id)?;
        crate::validation::require_non_empty("order_no", order_no)?;
        crate::validation::require_non_empty("out_trade_no", out_trade_no)?;
        crate::validation::require_non_empty("currency_code", currency_code)?;
        crate::validation::require_non_empty("template_uuid", template_uuid)?;
        crate::validation::require_non_empty("template_name", template_name)?;
        crate::validation::require_non_empty("expires_at", expires_at)?;
        crate::validation::require_non_empty("payment_method", payment_method)?;
        crate::validation::require_non_empty("status", status)?;
        crate::validation::require_non_empty("cashier_url", cashier_url)?;

        Ok(Self {
            amount,
            cashier_url: cashier_url.trim().to_string(),
            currency_code: currency_code.trim().to_ascii_uppercase(),
            expires_at: expires_at.trim().to_string(),
            order_id: order_id.trim().to_string(),
            order_no: order_no.trim().to_string(),
            out_trade_no: out_trade_no.trim().to_string(),
            payment_method: payment_method.trim().to_ascii_lowercase(),
            reused,
            status: status.trim().to_string(),
            template_name: template_name.trim().to_string(),
            template_uuid: template_uuid.trim().to_string(),
            version_uuid: version_uuid
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        })
    }
}

/// Folds the stored order lifecycle into the three states a buyer acts on.
pub fn app_template_order_status_label(status: &str) -> &'static str {
    match status.trim().to_ascii_lowercase().as_str() {
        "paid" | "fulfilled" | "completed" | "finished" => APP_TEMPLATE_ORDER_STATUS_PAID,
        "cancelled" | "canceled" | "expired" | "closed" => APP_TEMPLATE_ORDER_STATUS_CLOSED,
        _ => APP_TEMPLATE_ORDER_STATUS_PENDING_PAYMENT,
    }
}

#[cfg(test)]
mod tests {
    use super::{app_template_order_status_label, APP_TEMPLATE_ORDER_SUBJECT};

    #[test]
    fn settled_order_states_read_as_paid() {
        for status in ["paid", "FULFILLED", "completed", "finished"] {
            assert_eq!(app_template_order_status_label(status), "paid");
        }
        for status in ["cancelled", "canceled", "expired", "closed"] {
            assert_eq!(app_template_order_status_label(status), "closed");
        }
        assert_eq!(
            app_template_order_status_label("pending_payment"),
            "pending_payment"
        );
    }

    #[test]
    fn subject_is_a_machine_token_that_survives_settlement_resolution() {
        // `stable_order_settlement_subject` only falls back to the stored
        // subject for machine tokens, so the literal must stay one.
        assert!(APP_TEMPLATE_ORDER_SUBJECT
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'));
    }
}
