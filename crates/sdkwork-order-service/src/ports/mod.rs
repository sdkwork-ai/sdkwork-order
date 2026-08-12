mod account_ledger;
mod account_value;
mod membership_fulfillment;
mod owner_order_payment;
mod partner_relation;
mod payment_notify;
mod physical_goods;
mod physical_purchase;
mod points_recharge_fulfillment;
mod refund_notify;

pub use partner_relation::{
    NoopOrderPartnerRelationPort, OrderPartnerRelationFuture, OrderPartnerRelationPort,
    OrderPartnerSnapshot, ORDER_PARTNER_RELATION_PORT,
};

pub use account_ledger::{
    points_recharge_compensation_idempotency_key, points_recharge_compensation_transaction_no,
    points_recharge_fulfillment_idempotency_key, points_recharge_fulfillment_transaction_no,
    points_recharge_payment_success_idempotency_key, AccountPointsCreditFuture,
    AccountPointsCreditPort, PointsRechargeCreditOutcome, PointsRechargeCreditRequest,
    ACCOUNT_POINTS_CREDIT_PORT, POINTS_RECHARGE_LEDGER_BUSINESS_TYPE,
};
pub use account_value::{
    account_package_fulfillment_idempotency_key, coupon_recharge_fulfillment_idempotency_key,
    coupon_recharge_redemption_idempotency_key, refund_account_hold_idempotency_key,
    refund_payment_execution_idempotency_key, token_bank_plan_purchase_idempotency_key,
    token_bank_plan_renewal_idempotency_key, token_bank_recharge_fulfillment_idempotency_key,
    withdrawal_account_hold_idempotency_key, withdrawal_payment_execution_idempotency_key,
    AccountValueFulfillmentFuture, AccountValueFulfillmentStore, AccountValueFuture,
    AccountValueLedgerCommand, AccountValueLedgerOperation, AccountValueLedgerOutcome,
    AccountValueLedgerPort, AccountValueRequestExecutionStore, AccountValueRequestStatusCommand,
    CouponRedemptionBenefit, CouponRedemptionOutcome, CouponRedemptionPort,
    CouponRedemptionRequest, CouponSubscriptionPeriod, NoopAccountValueLedgerPort,
    NoopCouponRedemptionPort, NoopPaymentPayoutExecutorPort, NoopPaymentRefundExecutorPort,
    PaymentExecutorOutcome, PaymentPayoutExecutionRequest, PaymentPayoutExecutorPort,
    PaymentRefundExecutionRequest, PaymentRefundExecutorPort, ACCOUNT_VALUE_LEDGER_PORT,
    COUPON_REDEMPTION_PORT, PAYMENT_PAYOUT_EXECUTOR_PORT, PAYMENT_REFUND_EXECUTOR_PORT,
};
pub use membership_fulfillment::{
    membership_purchase_fulfillment_idempotency_key, membership_quota_recharge_idempotency_key,
    CouponSubscriptionFulfillmentOutcome, CouponSubscriptionFulfillmentRequest,
    MembershipPurchaseFulfillmentFuture, MembershipPurchaseFulfillmentOutcome,
    MembershipPurchaseFulfillmentPort, MembershipPurchaseFulfillmentRequest,
    MembershipPurchaseSettlementSnapshot, MembershipQuotaRechargeFulfillmentOutcome,
    MembershipQuotaRechargeFulfillmentRequest, NoopMembershipPurchaseFulfillmentPort,
    MEMBERSHIP_PURCHASE_FULFILLMENT_PORT,
};
pub use owner_order_payment::{
    ConfirmOwnerOrderPaymentOutcome, OrderPaymentSettlementAttempt,
    OwnerOrderPaymentConfirmationFuture, OwnerOrderPaymentConfirmationPort,
    OwnerOrderPaymentReconciliationPort, OwnerOrderPaymentStateOutcome, OwnerOrderPaymentStatePort,
    ReconcileOwnerOrderPaymentOutcome, ReconcileOwnerOrderPaymentRequest,
    UnavailableOwnerOrderPaymentReconciliationPort, OWNER_ORDER_PAYMENT_CONFIRMATION_PORT,
    OWNER_ORDER_PAYMENT_RECONCILIATION_PORT, OWNER_ORDER_PAYMENT_STATE_PORT,
};

pub use payment_notify::{
    PaymentNotifyAttemptContext, PaymentNotifyEvent, PaymentNotifyIngestFuture,
    PaymentNotifyIngestOutcome, PaymentNotifyIngestPort, PaymentNotifyOrderContext,
    PaymentNotifyOrderContextFuture, PaymentNotifyOrderContextPort, PaymentNotifyVerifyFuture,
    PaymentNotifyVerifyPort,
};

pub use physical_goods::{
    physical_goods_fulfillment_idempotency_key, FulfillPaidPhysicalOrderRequest,
    PhysicalGoodsFulfillmentOutcome, PhysicalGoodsFulfillmentPort, PhysicalGoodsFuture,
    UnavailablePhysicalGoodsFulfillmentPort, PHYSICAL_GOODS_FULFILLMENT_PORT,
};
pub use physical_purchase::{
    physical_inventory_release_idempotency_key, physical_inventory_reserve_idempotency_key,
    physical_order_fulfillment_requires_release, PhysicalCheckoutResolverPort,
    PhysicalInventoryLine, PhysicalInventoryMutationOutcome, PhysicalInventoryReservationPort,
    PhysicalPurchaseFuture, ReleasePhysicalOrderInventoryRequest,
    ReservePhysicalOrderInventoryRequest, ResolvePhysicalCheckoutLine,
    ResolvePhysicalCheckoutRequest, ResolvedPhysicalCheckout, ResolvedPhysicalCheckoutLine,
    ShippingAddressSnapshot, UnavailablePhysicalCheckoutResolverPort,
    UnavailablePhysicalInventoryReservationPort, PHYSICAL_CHECKOUT_RESOLVER_PORT,
    PHYSICAL_INVENTORY_RESERVATION_PORT,
};
pub use points_recharge_fulfillment::{
    PointsRechargeFulfillmentFuture, PointsRechargeFulfillmentStore,
    POINTS_RECHARGE_FULFILLMENT_STORE,
};
pub use refund_notify::{
    OwnerOrderRefundStateFuture, OwnerOrderRefundStateOutcome, RefundNotifyContext,
    RefundNotifyHandler, RefundNotifyHandlerFuture, RefundNotifyHandlerRegistry,
    RefundNotifyIngestFuture, RefundNotifyIngestOutcome, RefundNotifyIngestPort,
    RefundNotifyStatePort, REFUND_NOTIFY_BUSINESS_REFUND,
};

/// 仓储端口标识符，用于 `CommerceServiceContract` 能力注册。
///
/// 实际仓储抽象由 `SqliteCommerceOrderStore` / `PostgresCommerceOrderStore` 通过
/// 路由层枚举适配器（`BackendOrderAdminStore` / `AppAfterSalesState`）提供，
/// 无需额外的同步 trait 抽象层。
pub const ORDER_REPOSITORY_PORT: &str = "order.repository";
pub const IDEMPOTENCY_REPOSITORY_PORT: &str = "idempotency.repository";
