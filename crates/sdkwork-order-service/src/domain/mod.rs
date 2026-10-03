mod account_value;
mod app_template;
mod fulfillment;
mod membership;
mod recharge;

pub use account_value::{
    AccountValueAssetCode, AccountValueFulfillmentContext, AccountValueOrderSubject,
    AccountValuePackageItem, AccountValuePackageListPage, AccountValueRequestListPage,
    AccountValueRequestView, CreateAccountRechargeOrderOutcome, FulfillAccountValueOrderOutcome,
    TokenBankPlanItem, TokenBankPlanListPage, TokenBankPlanPeriod,
};
pub use app_template::{
    app_template_order_status_label, CreateAppTemplateOrderOutcome,
    APP_TEMPLATE_ORDER_STATUS_CLOSED, APP_TEMPLATE_ORDER_STATUS_PAID,
    APP_TEMPLATE_ORDER_STATUS_PENDING_PAYMENT, APP_TEMPLATE_ORDER_SUBJECT,
};
pub use fulfillment::{FulfillPointsRechargeOrderOutcome, PointsRechargeFulfillmentContext};
pub use membership::CreateMembershipOrderOutcome;
pub use recharge::{
    CheckoutStatusSnapshot, CreatePointsRechargeOrderOutcome, RechargeGrantPreview,
    RechargePackageItem, RechargeSettingsSnapshot,
};

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderItemDraft {
    pub quantity: u32,
    pub sku_id: String,
    pub title: String,
    pub unit_price: CommerceMoney,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderAmountBreakdown {
    pub discount_amount: CommerceMoney,
    pub original_amount: CommerceMoney,
    pub payable_amount: CommerceMoney,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrderStatus {
    Draft,
    PendingPayment,
    Paid,
    Fulfilled,
    Completed,
    Cancelled,
    Expired,
    Refunding,
    Refunded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderTransition {
    from: OrderStatus,
    to: OrderStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaidOrderReference {
    pub order_id: String,
    pub payment_id: String,
}

impl OrderItemDraft {
    pub fn new(
        sku_id: &str,
        title: &str,
        quantity: u32,
        unit_price: CommerceMoney,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("sku_id", sku_id)?;
        crate::validation::require_non_empty("title", title)?;
        if quantity == 0 {
            return Err(CommerceServiceError::validation(
                "quantity must be greater than zero",
            ));
        }

        Ok(Self {
            quantity,
            sku_id: sku_id.to_string(),
            title: title.to_string(),
            unit_price,
        })
    }
}

impl OrderAmountBreakdown {
    pub fn from_items(
        items: Vec<OrderItemDraft>,
        discount_amount: CommerceMoney,
    ) -> Result<Self, CommerceServiceError> {
        if items.is_empty() {
            return Err(CommerceServiceError::validation(
                "order requires at least one item",
            ));
        }

        let total_amount = items.iter().try_fold(0_i64, |total, item| {
            let unit_amount = money_to_minor_units(item.unit_price.as_str())?;
            let line_amount = unit_amount
                .checked_mul(i64::from(item.quantity))
                .ok_or_else(|| {
                    CommerceServiceError::validation("order line amount is too large")
                })?;
            total
                .checked_add(line_amount)
                .ok_or_else(|| CommerceServiceError::validation("order amount is too large"))
        })?;
        let discount_amount_minor = money_to_minor_units(discount_amount.as_str())?;
        if discount_amount_minor > total_amount {
            return Err(CommerceServiceError::validation(
                "discount cannot exceed original amount",
            ));
        }
        let payable_amount = total_amount
            .checked_sub(discount_amount_minor)
            .ok_or_else(|| CommerceServiceError::validation("payable amount is invalid"))?;

        Ok(Self {
            discount_amount,
            original_amount: minor_units_to_money(total_amount),
            payable_amount: minor_units_to_money(payable_amount),
        })
    }
}

impl OrderStatus {
    pub fn can_cancel(&self) -> bool {
        matches!(self, Self::PendingPayment)
    }

    pub fn can_expire(&self) -> bool {
        matches!(self, Self::PendingPayment)
    }
}

impl OrderTransition {
    pub fn new(from: OrderStatus, to: OrderStatus) -> Self {
        Self { from, to }
    }

    pub fn validate(&self) -> Result<(), CommerceServiceError> {
        match (&self.from, &self.to) {
            (OrderStatus::Draft, OrderStatus::PendingPayment)
            | (OrderStatus::PendingPayment, OrderStatus::Paid)
            | (OrderStatus::Paid, OrderStatus::Fulfilled)
            | (OrderStatus::Fulfilled, OrderStatus::Completed)
            | (OrderStatus::PendingPayment, OrderStatus::Cancelled)
            | (OrderStatus::PendingPayment, OrderStatus::Expired)
            | (OrderStatus::Paid, OrderStatus::Refunding)
            | (OrderStatus::Refunding, OrderStatus::Refunded) => Ok(()),
            _ => Err(CommerceServiceError::invalid_state(
                "invalid order status transition",
            )),
        }
    }
}

impl PaidOrderReference {
    pub fn new(order_id: &str, payment_id: &str) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("order_id", order_id)?;
        crate::validation::require_non_empty("payment_id", payment_id)?;

        Ok(Self {
            order_id: order_id.to_string(),
            payment_id: payment_id.to_string(),
        })
    }
}

fn money_to_minor_units(value: &str) -> Result<i64, CommerceServiceError> {
    value
        .parse::<i64>()
        .map_err(|_| CommerceServiceError::validation("money amount is too large"))
}

fn minor_units_to_money(amount: i64) -> CommerceMoney {
    CommerceMoney::new(&amount.to_string()).expect("computed money should be valid")
}
