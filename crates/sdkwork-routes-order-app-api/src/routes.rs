use axum::Router;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service_host::OrderServiceHost;
use std::sync::Arc;

use crate::openapi_contract::mount_app_openapi;
use crate::web_bootstrap::wrap_router_with_web_framework_from_env;
use crate::{
    app_after_sales_router_with_postgres_pool, app_fulfillment_router_with_postgres_pool,
    app_membership_order_router_with_postgres_pool_and_payments,
    app_order_router_with_postgres_pool_and_inventory,
    app_payment_webhook_router_with_postgres_pool_and_integrations,
    app_refund_webhook_router_with_postgres_pool, app_shipment_router_with_postgres_pool,
    build_app_checkout_router_with_integrations,
    build_app_recharge_checkout_router_with_integrations,
};
use sdkwork_order_repository_sqlx::{PostgresCommerceOrderStore, PostgresCommerceRechargeStore};
use sdkwork_order_service::{AccountValueLedgerPort, CouponRedemptionPort};
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};

pub fn build_order_app_router(host: Arc<OrderServiceHost>) -> Router {
    mount_app_openapi(build_order_app_business_router(host))
}

/// Builds the complete Order app-api business surface without infrastructure/OpenAPI routes.
/// Gateway assemblies use this entrypoint when Order is embedded into a shared HTTP ingress.
pub fn build_order_app_business_router(host: Arc<OrderServiceHost>) -> Router {
    let credit_port = host.account_credit_port();
    let account_value_ledger_port = host.account_value_ledger_port();
    let coupon_redemption_port = host.coupon_redemption_port();
    let membership_port = host.membership_fulfillment_port();
    let physical_checkout_resolver = host.physical_checkout_resolver_port();
    let physical_inventory = host.physical_inventory_reservation_port();
    let physical_goods = host.physical_goods_fulfillment_port();
    let credentials = ProviderCredentialBundle::from_env();
    let registry = Arc::new(PaymentProviderRegistry::from_credentials(
        credentials.clone(),
    ));
    let DatabasePool::Postgres(pool, _) = host.database_pool() else {
        panic!("order app router requires a PostgreSQL database pool");
    };
    let pool = pool.clone();
    let router = Router::new()
        .merge(app_order_router_with_postgres_pool_and_inventory(
            pool.clone(),
            registry.clone(),
            credentials.clone(),
            physical_inventory.clone(),
        ))
        .merge(build_app_checkout_router_with_integrations(
            Arc::new(PostgresCommerceOrderStore::new(pool.clone())),
            physical_checkout_resolver.clone(),
            physical_inventory.clone(),
        ))
        .merge(build_recharge_router_postgres(
            pool.clone(),
            registry.clone(),
            credentials.clone(),
            coupon_redemption_port.clone(),
            account_value_ledger_port.clone(),
            membership_port.clone(),
        ))
        .merge(app_membership_order_router_with_postgres_pool_and_payments(
            pool.clone(),
            registry,
            credentials,
        ))
        .merge(app_fulfillment_router_with_postgres_pool(pool.clone()))
        .merge(app_shipment_router_with_postgres_pool(pool.clone()))
        .merge(app_after_sales_router_with_postgres_pool(pool.clone()))
        .merge(
            app_payment_webhook_router_with_postgres_pool_and_integrations(
                pool.clone(),
                credit_port,
                account_value_ledger_port,
                coupon_redemption_port,
                membership_port,
                physical_goods,
            ),
        )
        .merge(app_refund_webhook_router_with_postgres_pool(pool));
    router
}

fn build_recharge_router_postgres(
    pool: sqlx::PgPool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
    coupon: Arc<dyn CouponRedemptionPort>,
    ledger: Arc<dyn AccountValueLedgerPort>,
    membership: Arc<dyn sdkwork_order_service::MembershipPurchaseFulfillmentPort>,
) -> axum::Router {
    let store = Arc::new(PostgresCommerceRechargeStore::new(pool.clone()));
    build_app_recharge_checkout_router_with_integrations(
        store.clone(),
        store,
        coupon,
        ledger,
        membership,
        Arc::new(PostgresCommerceOrderStore::new(pool.clone())),
        crate::owner_order_payment_enrich::enriched_postgres_owner_order_payments(
            pool,
            registry,
            credentials,
        ),
    )
}

pub async fn build_order_app_router_with_framework(host: Arc<OrderServiceHost>) -> Router {
    wrap_router_with_web_framework_from_env(build_order_app_router(host)).await
}
