//! The composed assembly is the one routing composition the platform cloud
//! gateway embeds (`web_module_with_pool`), so the buyer cart routes must be
//! present on its exported manifest — that manifest is what the gateway route
//! registry materializes. When they are missing the gateway answers
//! `404 route-not-in-manifest` for every `commerce().cart.*` client call.

use sdkwork_routes_order_app_api::http_route_manifest::app_route_manifest;
use sdkwork_web_core::HttpMethod;

#[test]
fn composed_app_manifest_carries_the_cart_operations() {
    let manifest = app_route_manifest();
    let routes = manifest.routes();

    let expected = [
        (
            HttpMethod::Get,
            "/app/v3/api/cart/current",
            "cart.current.retrieve",
        ),
        (
            HttpMethod::Post,
            "/app/v3/api/cart/items",
            "cart.items.create",
        ),
        (
            HttpMethod::Patch,
            "/app/v3/api/cart/items/{cartItemId}",
            "cart.items.update",
        ),
        (
            HttpMethod::Delete,
            "/app/v3/api/cart/items/{cartItemId}",
            "cart.items.delete",
        ),
    ];

    for (method, path, operation_id) in expected {
        assert!(
            routes.iter().any(|route| route.method == method
                && route.path == path
                && route.operation_id == operation_id),
            "app route manifest must declare {operation_id} ({method:?} {path})"
        );
    }
}
