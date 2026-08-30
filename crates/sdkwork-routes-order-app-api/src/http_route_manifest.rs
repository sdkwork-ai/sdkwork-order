//! Order app-api HTTP route manifest (`API_SPEC.md` §4.2.1, `WEB_FRAMEWORK_SPEC.md` §2/§7).

use sdkwork_web_core::{HttpMethod, HttpRoute, HttpRouteManifest};

pub const APP_API_PREFIX: &str = "/app/v3/api";

pub fn order_app_api_public_path_prefixes() -> Vec<String> {
    sdkwork_web_bootstrap::infra_public_path_prefixes()
}

const HTTP_ROUTES: &[HttpRoute] = &[
    // === Checkout ===
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/checkout/sessions",
        "checkout",
        "checkout.sessions.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/checkout/sessions/{checkoutSessionId}",
        "checkout",
        "checkout.sessions.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/checkout/sessions/{checkoutSessionId}/quotes",
        "checkout",
        "checkout.sessions.quotes.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/checkout/sessions/{checkoutSessionId}/orders",
        "checkout",
        "checkout.sessions.orders.create",
    )
    .with_idempotent(true),
    // === Orders ===
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/orders",
        "orders",
        "orders.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/orders/statistics",
        "orders",
        "orders.statistics.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/orders/{orderId}",
        "orders",
        "orders.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/orders/{orderId}/payments",
        "payments",
        "payments.orderPayments.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/orders/{orderId}/payments",
        "orders",
        "orders.payments.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/orders/{orderId}/status",
        "orders",
        "orders.status.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/orders/{orderId}/payment_success",
        "orders",
        "orders.paymentSuccess.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/orders/{orderId}/events",
        "orders",
        "orders.events.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/orders/{orderId}/cancellations",
        "orders",
        "orders.cancellations.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/orders/{orderId}/receipt_confirmations",
        "orders",
        "orders.receipts.confirm",
    )
    .with_idempotent(true),
    // === Payment webhooks (PSP callbacks; no user token) ===
    HttpRoute::public(
        HttpMethod::Post,
        "/app/v3/api/orders/payments/webhooks/{providerCode}",
        "orders",
        "orders.payments.webhooks.receive",
    ),
    HttpRoute::public(
        HttpMethod::Post,
        "/app/v3/api/orders/refunds/webhooks/{providerCode}",
        "orders",
        "orders.refunds.webhooks.receive",
    ),
    // === After-sales ===
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/after_sales/requests",
        "afterSales",
        "afterSales.requests.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/after_sales/requests",
        "afterSales",
        "afterSales.requests.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/after_sales/requests/{afterSalesRequestId}",
        "afterSales",
        "afterSales.requests.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Patch,
        "/app/v3/api/after_sales/requests/{afterSalesRequestId}",
        "afterSales",
        "afterSales.requests.update",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/after_sales/requests/{afterSalesRequestId}/events",
        "afterSales",
        "afterSales.events.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/after_sales/requests/{afterSalesRequestId}/return_shipments",
        "afterSales",
        "afterSales.returnShipments.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/after_sales/requests/{afterSalesRequestId}/return_shipments",
        "afterSales",
        "afterSales.returnShipments.create",
    )
    .with_idempotent(true),
    // === Fulfillment / shipment ===
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/fulfillments",
        "fulfillments",
        "fulfillments.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/fulfillments/{fulfillmentId}",
        "fulfillments",
        "fulfillments.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/shipments/{shipmentId}",
        "shipments",
        "shipments.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/shipments/{shipmentId}/packages",
        "shipments",
        "shipments.packages.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/shipments/{shipmentId}/tracking_events",
        "shipments",
        "shipments.trackingEvents.list",
    ),
    // === Recharges ===
    HttpRoute::public(
        HttpMethod::Get,
        "/app/v3/api/recharges/packages",
        "recharges",
        "recharges.packages.list",
    ),
    HttpRoute::public(
        HttpMethod::Get,
        "/app/v3/api/recharges/plans",
        "recharges",
        "recharges.plans.list",
    ),
    HttpRoute::public(
        HttpMethod::Get,
        "/app/v3/api/recharges/settings",
        "recharges",
        "recharges.settings.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/recharges/orders",
        "recharges",
        "recharges.orders.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/recharges/orders",
        "recharges",
        "recharges.orders.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/recharges/orders/{orderId}",
        "recharges",
        "recharges.orders.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/recharges/orders/{orderId}/cancel",
        "recharges",
        "recharges.orders.cancel",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/orders/coupon_redemptions",
        "orders",
        "orders.couponRedemptions.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/orders/refund_requests",
        "orders",
        "orders.refundRequests.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/orders/refund_requests",
        "orders",
        "orders.refundRequests.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/orders/refund_requests/{refundRequestId}",
        "orders",
        "orders.refundRequests.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/withdrawals/requests",
        "withdrawals",
        "withdrawals.requests.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/withdrawals/requests/{withdrawalRequestId}",
        "withdrawals",
        "withdrawals.requests.retrieve",
    ),
    // === Memberships ===
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/memberships/orders",
        "memberships",
        "memberships.orders.create",
    )
    .with_idempotent(true),
];

pub fn app_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(HTTP_ROUTES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_web_core::{RouteAuth, WebRequestContextProfile};

    #[test]
    fn manifest_declares_all_routes_with_dual_token_auth() {
        let manifest = app_route_manifest();
        assert!(!manifest.routes().is_empty());
        for route in manifest.routes() {
            if route.path.contains("/payments/webhooks/")
                || route.path.contains("/refunds/webhooks/")
                || route.path.contains("/recharges/packages")
                || route.path.contains("/recharges/plans")
                || route.path.contains("/recharges/settings")
            {
                assert_eq!(
                    route.auth,
                    RouteAuth::Public,
                    "provider webhook routes must be public"
                );
                continue;
            }
            assert_eq!(
                route.auth,
                RouteAuth::DualToken,
                "route {:?} {} must use dual-token auth",
                route.method,
                route.path,
            );
            assert!(
                route.path.starts_with(APP_API_PREFIX),
                "route {:?} {} must start with {APP_API_PREFIX}",
                route.method,
                route.path,
            );
        }
    }

    #[test]
    fn manifest_passes_framework_validations() {
        let manifest = app_route_manifest();
        let profile = WebRequestContextProfile {
            public_path_prefixes: order_app_api_public_path_prefixes(),
            ..WebRequestContextProfile::default()
        };
        manifest
            .validate_public_path_prefixes(&profile.public_path_prefixes)
            .expect("public prefixes must not cover protected manifest routes");
        manifest
            .validate_route_auth_for_surfaces(&profile)
            .expect("all app-api routes must declare dual-token auth");
        manifest
            .validate_no_ambient_context_path_markers(&profile)
            .expect("manifest must not embed ambient tenant/org scoping");
    }

    #[test]
    fn manifest_methods_match_openapi_authority() {
        let manifest = app_route_manifest();
        let openapi: serde_json::Value = serde_json::from_str(include_str!(
            "../../../apis/app-api/order/order-app-api.openapi.json"
        ))
        .expect("parse app openapi authority");

        for route in manifest.routes() {
            let wire_method = manifest_method_wire(route.method);
            let methods = openapi["paths"][route.path].as_object().unwrap_or_else(|| {
                panic!(
                    "manifest route {:?} {} missing from OpenAPI paths",
                    route.method, route.path
                )
            });
            assert!(
                methods.contains_key(wire_method),
                "manifest route {:?} {} must declare {wire_method} in OpenAPI",
                route.method,
                route.path
            );
        }
    }

    fn manifest_method_wire(method: HttpMethod) -> &'static str {
        match method {
            HttpMethod::Get => "get",
            HttpMethod::Post => "post",
            HttpMethod::Put => "put",
            HttpMethod::Patch => "patch",
            HttpMethod::Delete => "delete",
        }
    }
}
