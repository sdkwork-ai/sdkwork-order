//! Order backend-api HTTP route manifest (`API_SPEC.md` §4.2.1, `WEB_BACKEND_SPEC.md` §4.2).

use sdkwork_web_core::{HttpMethod, HttpRoute, HttpRouteManifest};

pub const BACKEND_API_PREFIX: &str = "/backend/v3/api";

pub fn order_backend_api_public_path_prefixes() -> Vec<String> {
    sdkwork_web_bootstrap::infra_public_path_prefixes()
}

const HTTP_ROUTES: &[HttpRoute] = &[
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/orders",
        "orders",
        "orders.admin.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/orders/cancellations",
        "orders",
        "orders.admin.cancellations.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/orders/{orderId}",
        "orders",
        "orders.admin.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/orders/{orderId}/cancel",
        "orders",
        "orders.admin.cancel",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/orders/{orderId}/close",
        "orders",
        "orders.admin.close",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/orders/{orderId}/events",
        "orders",
        "orders.admin.events.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/orders/{orderId}/payment_confirmations",
        "orders",
        "orders.paymentConfirmations.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/account_value_packages",
        "backend",
        "backend.accountValuePackages.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/account_value_packages",
        "backend",
        "backend.accountValuePackages.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Patch,
        "/backend/v3/api/account_value_packages/{packageId}",
        "backend",
        "backend.accountValuePackages.update",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/account_value_packages/{packageId}/retire",
        "backend",
        "backend.accountValuePackages.retire",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/token_bank_plans",
        "backend",
        "backend.tokenBankPlans.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/token_bank_plans",
        "backend",
        "backend.tokenBankPlans.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Patch,
        "/backend/v3/api/token_bank_plans/{planCode}",
        "backend",
        "backend.tokenBankPlans.update",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/token_bank_plans/{planCode}/retire",
        "backend",
        "backend.tokenBankPlans.retire",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/refund_requests",
        "backend",
        "backend.refundRequests.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/refund_requests/{refundRequestId}/approve",
        "backend",
        "backend.refundRequests.approve",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/refund_requests/{refundRequestId}/reject",
        "backend",
        "backend.refundRequests.reject",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/refund_requests/{refundRequestId}/retry",
        "backend",
        "backend.refundRequests.retry",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/withdrawal_requests",
        "backend",
        "backend.withdrawalRequests.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/withdrawal_requests/{withdrawalRequestId}/approve",
        "backend",
        "backend.withdrawalRequests.approve",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/withdrawal_requests/{withdrawalRequestId}/reject",
        "backend",
        "backend.withdrawalRequests.reject",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/withdrawal_requests/{withdrawalRequestId}/retry",
        "backend",
        "backend.withdrawalRequests.retry",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/after_sales/requests",
        "afterSales",
        "afterSales.management.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/after_sales/requests/{afterSalesRequestId}",
        "afterSales",
        "afterSales.management.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/after_sales/requests/{afterSalesRequestId}/reviews",
        "afterSales",
        "afterSales.reviews.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/shipments",
        "commerce",
        "shipments.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/shipments/{shipmentId}",
        "commerce",
        "shipments.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/shipments/{shipmentId}/packages",
        "commerce",
        "shipments.packages.management.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/shipments/{shipmentId}/packages",
        "commerce",
        "shipments.packages.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Patch,
        "/backend/v3/api/shipments/{shipmentId}/packages/{packageId}",
        "commerce",
        "shipments.packages.update",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/shipments/{shipmentId}/tracking_events",
        "commerce",
        "shipments.trackingEvents.management.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/shipments/{shipmentId}/tracking_events",
        "commerce",
        "shipments.trackingEvents.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/carriers",
        "commerce",
        "carriers.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/carriers",
        "commerce",
        "carriers.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Patch,
        "/backend/v3/api/carriers/{carrierId}",
        "commerce",
        "carriers.update",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/fulfillments",
        "commerce",
        "fulfillments.management.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/shipments",
        "commerce",
        "shipments.create",
    )
    .with_idempotent(true),
];

pub fn backend_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(HTTP_ROUTES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_web_core::{RouteAuth, WebRequestContextProfile};

    #[test]
    fn manifest_declares_all_routes_with_dual_token_auth() {
        let manifest = backend_route_manifest();
        for route in manifest.routes() {
            assert_eq!(route.auth, RouteAuth::DualToken);
            assert!(route.path.starts_with(BACKEND_API_PREFIX));
        }
    }

    #[test]
    fn manifest_passes_framework_validations() {
        let manifest = backend_route_manifest();
        let profile = WebRequestContextProfile {
            public_path_prefixes: order_backend_api_public_path_prefixes(),
            ..WebRequestContextProfile::default()
        };
        manifest
            .validate_public_path_prefixes(&profile.public_path_prefixes)
            .expect("public prefixes must not cover protected manifest routes");
        manifest
            .validate_route_auth_for_surfaces(&profile)
            .expect("all backend-api routes must declare dual-token auth");
        manifest
            .validate_no_ambient_context_path_markers(&profile)
            .expect("manifest must not embed ambient tenant/org scoping");
    }

    #[test]
    fn manifest_methods_match_openapi_authority() {
        let manifest = backend_route_manifest();
        let openapi: serde_json::Value = serde_json::from_str(include_str!(
            "../../../apis/backend-api/order/order-backend-api.openapi.json"
        ))
        .expect("parse backend openapi authority");

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
