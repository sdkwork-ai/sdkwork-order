use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_database_host::{bootstrap_order_database_from_env, OrderDatabaseHost};
use sdkwork_order_integration_account::{
    account_points_credit_port_from_env, account_value_ledger_port_from_env,
};
use sdkwork_order_integration_membership::membership_purchase_fulfillment_port_from_database_pool;
use sdkwork_order_integration_payment::{
    owner_order_payment_reconciliation_port_from_database_pool,
    payment_refund_executor_port_from_database_pool,
};
use sdkwork_order_integration_physical_commerce::physical_commerce_ports_from_env;
use sdkwork_order_integration_promotion::promotion_coupon_redemption_port_from_database_pool;
use sdkwork_order_integration_partner::order_partner_relation_port_from_database_pool;
pub use sdkwork_order_service::order_service_contract;
use sdkwork_order_service::{
    AccountPointsCreditPort, AccountValueLedgerPort, CouponRedemptionPort,
    MembershipPurchaseFulfillmentPort, NoopCouponRedemptionPort, NoopPaymentPayoutExecutorPort,
    OrderPartnerRelationPort, OwnerOrderPaymentReconciliationPort, PaymentPayoutExecutorPort,
    PaymentRefundExecutorPort, PhysicalCheckoutResolverPort, PhysicalGoodsFulfillmentPort,
    PhysicalInventoryReservationPort, UnavailablePhysicalCheckoutResolverPort,
    UnavailablePhysicalGoodsFulfillmentPort, UnavailablePhysicalInventoryReservationPort,
};
use std::sync::Arc;

pub mod expiration;

pub use expiration::spawn_order_expiration_scheduler;

pub struct OrderServiceHost {
    database: OrderDatabaseHost,
    account_credit_port: Arc<dyn AccountPointsCreditPort>,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
    membership_fulfillment_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
    owner_order_payment_reconciliation_port: Arc<dyn OwnerOrderPaymentReconciliationPort>,
    partner_relation_port: Arc<dyn OrderPartnerRelationPort>,
    payment_refund_executor_port: Arc<dyn PaymentRefundExecutorPort>,
    payment_payout_executor_port: Arc<dyn PaymentPayoutExecutorPort>,
    physical_checkout_resolver_port: Arc<dyn PhysicalCheckoutResolverPort>,
    physical_inventory_reservation_port: Arc<dyn PhysicalInventoryReservationPort>,
    physical_goods_fulfillment_port: Arc<dyn PhysicalGoodsFulfillmentPort>,
}

impl OrderServiceHost {
    pub async fn new() -> Self {
        Self::from_env()
            .await
            .expect("order service host bootstrap failed")
    }

    pub async fn from_env() -> Result<Self, String> {
        let database = bootstrap_order_database_from_env().await?;
        Self::from_database_with_env_integrations(database).await
    }

    /// Builds the Order service container on a pool owned by an embedding gateway assembly.
    pub async fn from_database_pool(pool: DatabasePool) -> Result<Self, String> {
        let database = OrderDatabaseHost::from_pool(pool)?;
        Self::from_database_with_env_integrations(database).await
    }

    async fn from_database_with_env_integrations(
        database: OrderDatabaseHost,
    ) -> Result<Self, String> {
        let account_credit_port = account_points_credit_port_from_env().await?;
        let account_value_ledger_port = account_value_ledger_port_from_env().await?;
        let coupon_redemption_port =
            promotion_coupon_redemption_port_from_database_pool(database.pool());
        let membership_fulfillment_port =
            membership_purchase_fulfillment_port_from_database_pool(database.pool())?;
        let owner_order_payment_reconciliation_port =
            owner_order_payment_reconciliation_port_from_database_pool(database.pool());
        let payment_refund_executor_port =
            payment_refund_executor_port_from_database_pool(database.pool());
        let payment_payout_executor_port = Arc::new(NoopPaymentPayoutExecutorPort);
        let physical_ports = physical_commerce_ports_from_env(database.pool()).await?;
        let partner_relation_port = order_partner_relation_port_from_database_pool(database.pool());
        Ok(Self {
            database,
            account_credit_port,
            account_value_ledger_port,
            coupon_redemption_port,
            membership_fulfillment_port,
            owner_order_payment_reconciliation_port,
            partner_relation_port,
            payment_refund_executor_port,
            payment_payout_executor_port,
            physical_checkout_resolver_port: physical_ports.checkout_resolver,
            physical_inventory_reservation_port: physical_ports.inventory,
            physical_goods_fulfillment_port: physical_ports.fulfillment,
        })
    }

    pub fn from_parts(
        database: OrderDatabaseHost,
        account_credit_port: Arc<dyn AccountPointsCreditPort>,
        account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
        membership_fulfillment_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
        payment_refund_executor_port: Arc<dyn PaymentRefundExecutorPort>,
        payment_payout_executor_port: Arc<dyn PaymentPayoutExecutorPort>,
    ) -> Self {
        Self::from_parts_with_coupon(
            database,
            account_credit_port,
            account_value_ledger_port,
            Arc::new(NoopCouponRedemptionPort),
            membership_fulfillment_port,
            payment_refund_executor_port,
            payment_payout_executor_port,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_parts_with_coupon(
        database: OrderDatabaseHost,
        account_credit_port: Arc<dyn AccountPointsCreditPort>,
        account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
        coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
        membership_fulfillment_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
        payment_refund_executor_port: Arc<dyn PaymentRefundExecutorPort>,
        payment_payout_executor_port: Arc<dyn PaymentPayoutExecutorPort>,
    ) -> Self {
        let owner_order_payment_reconciliation_port =
            owner_order_payment_reconciliation_port_from_database_pool(database.pool());
        let partner_relation_port =
            order_partner_relation_port_from_database_pool(database.pool());
        Self {
            database,
            account_credit_port,
            account_value_ledger_port,
            coupon_redemption_port,
            membership_fulfillment_port,
            owner_order_payment_reconciliation_port,
            partner_relation_port,
            payment_refund_executor_port,
            payment_payout_executor_port,
            physical_checkout_resolver_port: Arc::new(UnavailablePhysicalCheckoutResolverPort),
            physical_inventory_reservation_port: Arc::new(
                UnavailablePhysicalInventoryReservationPort,
            ),
            physical_goods_fulfillment_port: Arc::new(UnavailablePhysicalGoodsFulfillmentPort),
        }
    }

    pub fn database_pool(&self) -> &DatabasePool {
        self.database.pool()
    }

    pub fn database_module(&self) -> std::sync::Arc<sdkwork_database_spi::DefaultDatabaseModule> {
        self.database.module()
    }

    pub fn account_credit_port(&self) -> Arc<dyn AccountPointsCreditPort> {
        self.account_credit_port.clone()
    }

    pub fn account_value_ledger_port(&self) -> Arc<dyn AccountValueLedgerPort> {
        self.account_value_ledger_port.clone()
    }

    pub fn coupon_redemption_port(&self) -> Arc<dyn CouponRedemptionPort> {
        self.coupon_redemption_port.clone()
    }

    pub fn membership_fulfillment_port(&self) -> Arc<dyn MembershipPurchaseFulfillmentPort> {
        self.membership_fulfillment_port.clone()
    }

    pub fn owner_order_payment_reconciliation_port(
        &self,
    ) -> Arc<dyn OwnerOrderPaymentReconciliationPort> {
        self.owner_order_payment_reconciliation_port.clone()
    }

    pub fn partner_relation_port(&self) -> Arc<dyn OrderPartnerRelationPort> {
        self.partner_relation_port.clone()
    }

    pub fn payment_refund_executor_port(&self) -> Arc<dyn PaymentRefundExecutorPort> {
        self.payment_refund_executor_port.clone()
    }

    pub fn payment_payout_executor_port(&self) -> Arc<dyn PaymentPayoutExecutorPort> {
        self.payment_payout_executor_port.clone()
    }

    pub fn physical_checkout_resolver_port(&self) -> Arc<dyn PhysicalCheckoutResolverPort> {
        self.physical_checkout_resolver_port.clone()
    }

    pub fn physical_inventory_reservation_port(&self) -> Arc<dyn PhysicalInventoryReservationPort> {
        self.physical_inventory_reservation_port.clone()
    }

    pub fn physical_goods_fulfillment_port(&self) -> Arc<dyn PhysicalGoodsFulfillmentPort> {
        self.physical_goods_fulfillment_port.clone()
    }
}
