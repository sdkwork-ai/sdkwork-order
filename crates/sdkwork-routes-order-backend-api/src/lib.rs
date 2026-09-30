pub mod api_response;
pub mod backend_acl;
pub mod backend_command_headers;
pub mod backend_commerce_admin_router;
pub mod backend_management_lifecycle;
pub mod backend_order_admin_router;
pub mod http_route_manifest;
pub mod openapi_contract;
pub mod payment_confirmation_router;
pub mod routes;
pub mod subject;
pub mod web_bootstrap;

pub use backend_commerce_admin_router::{
    backend_commerce_admin_router_with_postgres_pool,
    backend_commerce_admin_router_with_postgres_pool_and_ports,
};
pub use backend_order_admin_router::backend_order_admin_router_with_postgres_pool;
pub use payment_confirmation_router::{
    payment_confirmation_router_with_postgres_pool,
    payment_confirmation_router_with_postgres_pool_and_coupon,
    payment_confirmation_router_with_postgres_pool_and_integrations,
};
pub use routes::{build_order_backend_business_router, build_order_backend_router_with_framework};

use axum::Router;
use sdkwork_order_service_host::OrderServiceHost;
use std::sync::Arc;

pub async fn gateway_mount(host: Arc<OrderServiceHost>) -> Router {
    build_order_backend_business_router(host)
}

pub fn gateway_route_manifest() -> sdkwork_web_core::HttpRouteManifest {
    http_route_manifest::backend_route_manifest()
}
