mod account_value;
mod app_template;
mod fulfillment;
mod membership;
mod recharge;

use crate::OrderItemDraft;
use sdkwork_contract_service::CommerceMoney;

pub use account_value::{
    AccountValueRequestReviewAction, CreateAccountRechargeOrderCommand,
    CreateCashWithdrawalRequestCommand, CreateCouponRechargeOrderCommand,
    CreateOrderRefundRequestCommand, FulfillAccountValueOrderCommand,
    RetireAccountValuePackageCommand, RetireTokenBankPlanCommand, ReviewAccountValueRequestCommand,
    UpsertAccountValuePackageCommand, UpsertTokenBankPlanCommand,
};
pub use app_template::CreateAppTemplateOrderCommand;
pub use fulfillment::{
    FulfillPointsRechargeOrderCommand, MarkPointsRechargePaymentSucceededCommand,
};
pub use membership::CreateMembershipOrderCommand;
pub use recharge::CreatePointsRechargeOrderCommand;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateOrderCommand {
    pub discount_amount: CommerceMoney,
    pub idempotency_key: String,
    pub items: Vec<OrderItemDraft>,
    pub owner_user_id: String,
    pub request_no: String,
    pub subject: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CancelOrderCommand {
    pub idempotency_key: String,
    pub order_id: String,
    pub request_no: String,
    pub tenant_id: String,
}

/// Applies one promotion discount to a payable order and reprices its
/// `order_total` breakdown. `discount_amount_minor` is incremental: the store
/// adds it to the discount already on the order so combinable coupons stack.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyOrderDiscountCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
    pub discount_amount_minor: i64,
}

impl ApplyOrderDiscountCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
        discount_amount_minor: i64,
    ) -> Result<Self, sdkwork_contract_service::CommerceServiceError> {
        if tenant_id.trim().is_empty()
            || owner_user_id.trim().is_empty()
            || order_id.trim().is_empty()
        {
            return Err(sdkwork_contract_service::CommerceServiceError::validation(
                "order discount identifiers must not be empty",
            ));
        }
        if discount_amount_minor < 0 {
            return Err(sdkwork_contract_service::CommerceServiceError::validation(
                "order discount amount must not be negative",
            ));
        }
        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: organization_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            owner_user_id: owner_user_id.trim().to_string(),
            order_id: order_id.trim().to_string(),
            discount_amount_minor,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderDiscountView {
    pub order_id: String,
    /// Integer minor units, matching `commerce_order_amount_breakdown`.
    pub original_amount_minor: String,
    pub discount_amount_minor: String,
    pub payable_amount_minor: String,
    pub currency_code: String,
}
