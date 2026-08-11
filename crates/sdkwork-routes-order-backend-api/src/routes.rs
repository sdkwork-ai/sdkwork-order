use axum::Router;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service_host::OrderServiceHost;
use std::sync::Arc;

use crate::{
    backend_commerce_admin_router_with_postgres_pool_and_ports,
    backend_order_admin_router_with_postgres_pool,
    openapi_contract::mount_backend_openapi,
    payment_confirmation_router_with_postgres_pool_and_integrations,
};

pub fn build_order_backend_router(host: Arc<OrderServiceHost>) -> Router {
    mount_backend_openapi(build_order_backend_business_router(host))
}

/// Builds backend business routes without OpenAPI or process infrastructure mounts.
pub fn build_order_backend_business_router(host: Arc<OrderServiceHost>) -> Router {
    let credit_port = host.account_credit_port();
    let account_value_ledger_port = host.account_value_ledger_port();
    let coupon_redemption_port = host.coupon_redemption_port();
    let membership_port = host.membership_fulfillment_port();
    let reconciliation_port = host.owner_order_payment_reconciliation_port();
    let physical_goods_port = host.physical_goods_fulfillment_port();
    let payment_refund_executor_port = host.payment_refund_executor_port();
    let payment_payout_executor_port = host.payment_payout_executor_port();
    let DatabasePool::Postgres(pool, _) = host.database_pool() else {
        panic!("order backend router requires a PostgreSQL database pool");
    };
    let router = Router::new()
        .merge(backend_order_admin_router_with_postgres_pool(
            pool.clone(),
            host.physical_inventory_reservation_port(),
        ))
        .merge(backend_commerce_admin_router_with_postgres_pool_and_ports(
            pool.clone(),
            account_value_ledger_port.clone(),
            payment_refund_executor_port.clone(),
            payment_payout_executor_port.clone(),
            host.physical_inventory_reservation_port(),
        ))
        .merge(
            payment_confirmation_router_with_postgres_pool_and_integrations(
                pool.clone(),
                credit_port,
                account_value_ledger_port,
                coupon_redemption_port,
                membership_port,
                reconciliation_port,
                physical_goods_port,
            ),
        );
    router
}

pub async fn build_order_backend_router_with_framework(host: Arc<OrderServiceHost>) -> Router {
    crate::web_bootstrap::wrap_router_with_web_framework_from_env(build_order_backend_router(host))
        .await
}
