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
