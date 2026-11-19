use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use sdkwork_order_repository_sqlx::order_points_recharge_e2e_postgres_pool_from_env;
use sdkwork_order_service::{
    AccountPointsCreditFuture, AccountPointsCreditPort, NoopAccountValueLedgerPort,
    NoopAppTemplatePurchaseFulfillmentPort, NoopMembershipPurchaseFulfillmentPort,
    NoopPromotionDiscountPort, PointsRechargeCreditOutcome, PointsRechargeCreditRequest,
};
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_routes_order_app_api::{
    app_after_sales_router_with_postgres_pool, app_app_template_order_router_with_postgres_pool,
    app_cart_router_with_postgres_pool, app_checkout_router_with_postgres_pool,
    app_demand_claim_router_with_postgres_pool, app_fulfillment_router_with_postgres_pool,
    app_membership_order_router_with_postgres_pool, app_order_discount_router_with_postgres_pool,
    app_order_router_with_postgres_pool, app_payment_webhook_router_with_postgres_pool,
    app_recharge_checkout_router_with_postgres_pool, app_refund_webhook_router_with_postgres_pool,
    app_shipment_router_with_postgres_pool, build_app_cashier_router,
    openapi_contract::mount_app_openapi,
    owner_order_payment_enrich::enriched_postgres_owner_order_payments,
};
use serde_json::Value;
use std::sync::Arc;
use tower::util::ServiceExt;

struct NoopAccountPointsCreditPort;

impl AccountPointsCreditPort for NoopAccountPointsCreditPort {
    fn credit_points_recharge<'a>(
        &'a self,
        _request: PointsRechargeCreditRequest,
    ) -> AccountPointsCreditFuture<'a, PointsRechargeCreditOutcome> {
        Box::pin(async move {
            Ok(PointsRechargeCreditOutcome {
                accepted: true,
                replayed: false,
            })
        })
    }

    fn reverse_points_recharge_credit<'a>(
        &'a self,
        _request: PointsRechargeCreditRequest,
    ) -> AccountPointsCreditFuture<'a, PointsRechargeCreditOutcome> {
        Box::pin(async move {
            Ok(PointsRechargeCreditOutcome {
                accepted: true,
                replayed: false,
            })
        })
    }
}

fn build_test_app_router(pool: sqlx::PgPool) -> Router {
    let credentials = ProviderCredentialBundle::from_env();
    let registry = Arc::new(PaymentProviderRegistry::from_credentials(
        credentials.clone(),
    ));
    // Every router the production composition mounts on the app surface is
    // merged here: a route added to `build_order_app_business_router` without
    // appearing in the app OpenAPI document (or the reverse) fails this test.
    let payments =
        enriched_postgres_owner_order_payments(pool.clone(), registry.clone(), credentials.clone());
    mount_app_openapi(
        Router::new()
            .merge(app_cart_router_with_postgres_pool(pool.clone()))
            .merge(app_order_router_with_postgres_pool(
                pool.clone(),
                registry.clone(),
                credentials.clone(),
            ))
            .merge(app_checkout_router_with_postgres_pool(pool.clone()))
            .merge(build_app_cashier_router(pool.clone(), payments))
            .merge(app_order_discount_router_with_postgres_pool(
                &pool,
                Arc::new(NoopPromotionDiscountPort),
            ))
            .merge(app_recharge_checkout_router_with_postgres_pool(
                pool.clone(),
                registry,
                credentials,
            ))
            .merge(app_membership_order_router_with_postgres_pool(pool.clone()))
            .merge(app_app_template_order_router_with_postgres_pool(
                pool.clone(),
            ))
            .merge(app_fulfillment_router_with_postgres_pool(pool.clone()))
            .merge(app_demand_claim_router_with_postgres_pool(pool.clone()))
            .merge(app_shipment_router_with_postgres_pool(pool.clone()))
            .merge(app_after_sales_router_with_postgres_pool(pool.clone()))
            .merge(app_payment_webhook_router_with_postgres_pool(
                pool.clone(),
                Arc::new(NoopAccountPointsCreditPort),
                Arc::new(NoopAccountValueLedgerPort),
                Arc::new(NoopMembershipPurchaseFulfillmentPort),
                Arc::new(NoopAppTemplatePurchaseFulfillmentPort),
            ))
            .merge(app_refund_webhook_router_with_postgres_pool(pool)),
    )
}

async fn test_pool() -> Option<sqlx::PgPool> {
    order_points_recharge_e2e_postgres_pool_from_env().await
}

#[tokio::test]
async fn app_openapi_document_is_served() {
    let Some(pool) = test_pool().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    let app = build_test_app_router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/app/v3/api/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn app_router_mounts_every_openapi_operation_path() {
    let Some(pool) = test_pool().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    let spec: Value = serde_json::from_str(include_str!(
        "../../../apis/app-api/order/order-app-api.openapi.json"
    ))
    .unwrap();
    let app = build_test_app_router(pool);
    let paths = spec["paths"].as_object().unwrap();

    for (template_path, item) in paths {
        for method_name in item.as_object().unwrap().keys() {
            let Some(method) = openapi_http_method(method_name) else {
                // Path-item objects may carry `parameters`, `summary`,
                // `description`, `servers`, or `$ref`; those are metadata, not
                // operations.
                continue;
            };
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(concrete_uri(template_path))
                        .header("content-type", "application/json")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert!(
                route_is_mounted(&response),
                "{method_name} {template_path} is not mounted"
            );
        }
    }
}

/// Whether a response came from a mounted route.
///
/// A matched SDKWork handler reports a missing resource (or a rejected command)
/// as `application/problem+json`, while the router fallback answers an unmounted
/// path with an empty default 404. A bare status check would misreport every
/// handler-level 404 (`GET /cashier/{token}` with an unknown token, for
/// example) as a missing route.
fn route_is_mounted(response: &axum::response::Response) -> bool {
    if response.status() != StatusCode::NOT_FOUND {
        return true;
    }
    response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/problem+json"))
}

fn openapi_http_method(method_name: &str) -> Option<Method> {
    match method_name.to_ascii_lowercase().as_str() {
        "get" => Some(Method::GET),
        "post" => Some(Method::POST),
        "put" => Some(Method::PUT),
        "patch" => Some(Method::PATCH),
        "delete" => Some(Method::DELETE),
        _ => None,
    }
}

fn concrete_uri(template_path: &str) -> String {
    template_path
        .replace("{orderId}", "order-1")
        .replace("{checkoutSessionId}", "session-1")
        .replace("{afterSalesRequestId}", "as-1")
        .replace("{refundRequestId}", "refund-1")
        .replace("{withdrawalRequestId}", "withdrawal-1")
        .replace("{shipmentId}", "shipment-1")
        .replace("{fulfillmentId}", "fulfillment-1")
        .replace("{providerCode}", "wechat_pay")
        .replace("{claimId}", "claim-1")
        .replace("{cartItemId}", "cart-item-1")
        .replace("{token}", "cashier-token-1")
}
