mod account_value_fulfillment;
mod account_value_request_execution;
mod coupon_recharge_fulfillment;
mod order_payment_settlement;
mod payment_notify;
mod payment_notify_orchestration;
mod points_recharge_fulfillment;
mod refund_notify_orchestration;

pub use account_value_fulfillment::{
    default_fulfill_account_value_order_command, fulfill_account_value_order,
};
pub use account_value_request_execution::execute_account_value_request_review;
pub use coupon_recharge_fulfillment::{
    redeem_coupon_and_fulfill_account_value_order, redeem_coupon_and_fulfill_order,
    CouponFulfilledBenefit, CouponFulfillmentOutcome,
};
pub use order_payment_settlement::{
    settle_owner_order_after_payment_success,
    settle_owner_order_after_payment_success_with_registry, stable_checkout_order_subject,
    stable_order_settlement_subject, OrderSubjectKind, OwnerOrderSettlementOutcome,
    OwnerOrderSettlementPorts,
};
pub use payment_notify::{
    default_payment_notify_handler_registry, dispatch_payment_notify_handler,
    AccountValueNotifyHandler, CouponRechargeNotifyHandler, DefaultPaymentNotifyHandlerRegistry,
    ExternalFulfillmentNotifyHandler, MembershipNotifyHandler, PaymentNotifyContext,
    PaymentNotifyHandler, PaymentNotifyHandlerFuture, PaymentNotifyHandlerRegistry,
    PaymentNotifyHandlingOutcome, PhysicalGoodsNotifyHandler, PointsRechargeNotifyHandler,
    UnknownSubjectNotifyHandler, PAYMENT_NOTIFY_BUSINESS_ACCOUNT_RECHARGE_PACKAGE,
    PAYMENT_NOTIFY_BUSINESS_COUPON_RECHARGE, PAYMENT_NOTIFY_BUSINESS_EXTERNAL,
    PAYMENT_NOTIFY_BUSINESS_MEMBERSHIP, PAYMENT_NOTIFY_BUSINESS_POINTS_RECHARGE,
    PAYMENT_NOTIFY_BUSINESS_PRODUCT, PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_PLAN_PURCHASE,
    PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_PLAN_RENEWAL, PAYMENT_NOTIFY_BUSINESS_TOKEN_BANK_RECHARGE,
    PAYMENT_NOTIFY_BUSINESS_UNKNOWN,
};
pub use payment_notify_orchestration::{
    is_refund_event_type, normalize_event_identity, process_payment_notify,
    process_payment_notify_verified, verify_and_normalize_event, PaymentNotifyProcessingOutcome,
    PAYMENT_NOTIFY_STATUS_ACCEPTED, PAYMENT_NOTIFY_STATUS_ORDER_FAILURE_MARKED,
    PAYMENT_NOTIFY_STATUS_SETTLED, PAYMENT_NOTIFY_STATUS_UNMAPPED,
};
pub use points_recharge_fulfillment::{
    default_fulfill_points_recharge_command, fulfill_points_recharge_order,
    ledger_business_type_for_points_recharge, mark_points_recharge_payment_succeeded,
};
pub use refund_notify_orchestration::{
    default_refund_notify_handler_registry, process_refund_notify, process_refund_notify_verified,
    DefaultRefundNotifyHandlerRegistry, RefundNotifyProcessingOutcome,
    ORDER_REFUND_STATUS_REFUNDED, ORDER_REFUND_STATUS_REFUNDING, ORDER_REFUND_STATUS_REFUND_FAILED,
    REFUND_NOTIFY_STATUS_ACCEPTED, REFUND_NOTIFY_STATUS_REFUNDED, REFUND_NOTIFY_STATUS_REFUNDING,
    REFUND_NOTIFY_STATUS_REFUND_FAILED,
};

use sdkwork_contract_service::CommerceServiceContract;

pub fn order_service_contract() -> CommerceServiceContract {
    CommerceServiceContract::new(
        "order",
        "commerce.order",
        vec![
            "checkout.sessions.create",
            "checkout.sessions.quotes.create",
            "checkout.sessions.orders.create",
            "afterSales.requests.create",
            "afterSales.requests.update",
            "afterSales.returnShipments.create",
            "afterSales.reviews.create",
            "orders.cancellations.create",
            "orders.payments.create",
            "recharges.orders.create",
            "recharges.orders.cancel",
            "orders.refundRequests.create",
            "withdrawals.requests.create",
            "backend.accountValuePackages.create",
            "backend.accountValuePackages.update",
            "backend.accountValuePackages.retire",
            "backend.tokenBankPlans.create",
            "backend.tokenBankPlans.update",
            "backend.tokenBankPlans.retire",
            "backend.refundRequests.approve",
            "backend.refundRequests.reject",
            "backend.refundRequests.retry",
            "backend.withdrawalRequests.approve",
            "backend.withdrawalRequests.reject",
            "backend.withdrawalRequests.retry",
            "memberships.orders.create",
            "orders.paymentConfirmations.create",
            "orders.admin.cancel",
            "orders.admin.close",
            "shipments.packages.create",
            "shipments.packages.update",
        ],
        vec![
            "checkout.sessions.retrieve",
            "orders.list",
            "orders.retrieve",
            "orders.events.list",
            "recharges.plans.list",
            "orders.refundRequests.list",
            "orders.refundRequests.retrieve",
            "withdrawals.requests.retrieve",
            "backend.accountValuePackages.list",
            "backend.tokenBankPlans.list",
            "backend.refundRequests.list",
            "backend.withdrawalRequests.list",
            "afterSales.requests.list",
            "afterSales.requests.retrieve",
            "afterSales.management.list",
            "afterSales.management.retrieve",
            "afterSales.returnShipments.list",
            "afterSales.events.list",
            "fulfillments.list",
            "fulfillments.retrieve",
            "shipments.list",
            "shipments.retrieve",
            "shipments.packages.list",
            "shipments.packages.management.list",
            "shipments.trackingEvents.list",
        ],
        vec![
            crate::ports::ORDER_REPOSITORY_PORT,
            crate::ports::IDEMPOTENCY_REPOSITORY_PORT,
            crate::ports::POINTS_RECHARGE_FULFILLMENT_STORE,
            crate::ports::ACCOUNT_POINTS_CREDIT_PORT,
            crate::ports::ACCOUNT_VALUE_LEDGER_PORT,
            crate::ports::PAYMENT_REFUND_EXECUTOR_PORT,
            crate::ports::PAYMENT_PAYOUT_EXECUTOR_PORT,
            crate::ports::COUPON_REDEMPTION_PORT,
            crate::ports::OWNER_ORDER_PAYMENT_CONFIRMATION_PORT,
            crate::ports::OWNER_ORDER_PAYMENT_STATE_PORT,
            crate::ports::MEMBERSHIP_PURCHASE_FULFILLMENT_PORT,
            crate::ports::PHYSICAL_GOODS_FULFILLMENT_PORT,
            crate::ports::PHYSICAL_CHECKOUT_RESOLVER_PORT,
            crate::ports::PHYSICAL_INVENTORY_RESERVATION_PORT,
        ],
        true,
    )
}
