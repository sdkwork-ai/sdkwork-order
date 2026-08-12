pub mod after_sales_router;
pub mod api_response;
pub mod checkout_router;
pub mod command_headers;
pub mod fulfillment_router;
pub mod http_route_manifest;
pub mod membership_router;
pub mod openapi_contract;
pub mod order_router;
pub mod owner_order_cancel;
pub mod owner_order_payment_enrich;
pub mod payment_webhook_router;
pub mod recharge_router;
mod refund_webhook_router;
pub mod routes;
pub mod shipment_router;
pub mod subject;
pub mod web_bootstrap;

pub use routes::{
    build_order_app_business_router, build_order_app_router, build_order_app_router_with_framework,
};

pub use after_sales_router::{
    app_after_sales_router_with_postgres_pool, build_app_after_sales_router,
    CommerceAfterSalesFuture, CommerceAfterSalesStore,
};
pub use checkout_router::{
    app_checkout_router_with_postgres_pool, build_app_checkout_router,
    build_app_checkout_router_with_integrations, CommerceCheckoutFuture, CommerceCheckoutStore,
};
pub use fulfillment_router::{
    app_fulfillment_router_with_postgres_pool, build_app_fulfillment_router,
    CommerceFulfillmentFuture, CommerceFulfillmentStore,
};
pub use membership_router::{
    app_membership_order_router_with_postgres_pool,
    app_membership_order_router_with_postgres_pool_and_payments,
    build_app_membership_order_router_with_payments, CommerceMembershipOrderFuture,
    CommerceMembershipOrderStore,
};
pub use order_router::{
    app_order_router_with_postgres_pool, app_order_router_with_postgres_pool_and_inventory,
    build_app_order_router, build_app_order_router_with_inventory, CommerceOrderFuture,
    CommerceOrderStore, OwnerOrderPaymentStore,
};
pub use payment_webhook_router::{
    app_payment_webhook_router_with_postgres_pool,
    app_payment_webhook_router_with_postgres_pool_and_coupon,
    app_payment_webhook_router_with_postgres_pool_and_integrations,
    app_payment_webhook_router_with_postgres_pool_and_integrations_and_registries,
};
pub use recharge_router::{
    app_recharge_checkout_router_with_postgres_pool, build_app_recharge_checkout_router,
    build_app_recharge_checkout_router_with_integrations, CommerceRechargeCheckoutFuture,
    CommerceRechargeCheckoutStore,
};
pub use refund_webhook_router::{
    app_refund_webhook_router_with_postgres_pool,
    app_refund_webhook_router_with_postgres_pool_and_registries,
};
pub use shipment_router::{
    app_shipment_router_with_postgres_pool, build_app_shipment_router, CommerceShipmentFuture,
    CommerceShipmentStore,
};

use axum::Router;
use sdkwork_order_service_host::OrderServiceHost;
use std::sync::Arc;

pub async fn gateway_mount(host: Arc<OrderServiceHost>) -> Router {
    build_order_app_business_router(host)
}

pub fn gateway_route_manifest() -> sdkwork_web_core::HttpRouteManifest {
    http_route_manifest::app_route_manifest()
}
