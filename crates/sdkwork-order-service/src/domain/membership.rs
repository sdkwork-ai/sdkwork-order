use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateMembershipOrderOutcome {
    pub action: String,
    pub order_id: String,
    pub order_no: String,
    pub out_trade_no: String,
    pub amount: CommerceMoney,
    pub currency_code: String,
    pub package_id: String,
    pub package_name: String,
    pub duration_days: i64,
    pub expires_at: String,
    pub payment_method: String,
    pub status: String,
    pub reused: bool,
    pub cashier_url: String,
}

impl CreateMembershipOrderOutcome {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        order_id: &str,
        action: &str,
        order_no: &str,
        out_trade_no: &str,
        amount: CommerceMoney,
        currency_code: &str,
        package_id: &str,
        package_name: &str,
        duration_days: i64,
        expires_at: &str,
        payment_method: &str,
        status: &str,
        reused: bool,
        cashier_url: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("order_id", order_id)?;
        crate::validation::require_non_empty("action", action)?;
        crate::validation::require_non_empty("order_no", order_no)?;
        crate::validation::require_non_empty("out_trade_no", out_trade_no)?;
        crate::validation::require_non_empty("currency_code", currency_code)?;
        crate::validation::require_non_empty("package_id", package_id)?;
        crate::validation::require_non_empty("package_name", package_name)?;
        crate::validation::require_non_empty("payment_method", payment_method)?;
        crate::validation::require_non_empty("expires_at", expires_at)?;
        crate::validation::require_non_empty("status", status)?;
        crate::validation::require_non_empty("cashier_url", cashier_url)?;
        if duration_days <= 0 && !action.trim().eq_ignore_ascii_case("recharge") {
            return Err(CommerceServiceError::validation(
                "membership package duration must be greater than zero",
            ));
        }

        Ok(Self {
            action: action.trim().to_ascii_lowercase(),
            order_id: order_id.trim().to_string(),
            order_no: order_no.trim().to_string(),
            out_trade_no: out_trade_no.trim().to_string(),
            amount,
            currency_code: currency_code.trim().to_ascii_uppercase(),
            package_id: package_id.trim().to_string(),
            package_name: package_name.trim().to_string(),
            duration_days,
            expires_at: expires_at.trim().to_string(),
            payment_method: payment_method.trim().to_ascii_lowercase(),
            status: status.trim().to_string(),
            reused,
            cashier_url: cashier_url.trim().to_string(),
        })
    }
}
