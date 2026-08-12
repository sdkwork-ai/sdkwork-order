use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use sdkwork_order_repository_sqlx::order_points_recharge_e2e_postgres_pool_from_env;
use sdkwork_order_service::{
    AccountPointsCreditFuture, AccountPointsCreditPort, NoopAccountValueLedgerPort,
    NoopCouponRedemptionPort, NoopMembershipPurchaseFulfillmentPort, PointsRechargeCreditOutcome,
    PointsRechargeCreditRequest,
};
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_routes_order_app_api::{
    app_after_sales_router_with_postgres_pool, app_checkout_router_with_postgres_pool,
    app_fulfillment_router_with_postgres_pool, app_membership_order_router_with_postgres_pool,
    app_order_router_with_postgres_pool, app_payment_webhook_router_with_postgres_pool,
    app_recharge_checkout_router_with_postgres_pool, app_refund_webhook_router_with_postgres_pool,
    app_shipment_router_with_postgres_pool, openapi_contract::mount_app_openapi,
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
    mount_app_openapi(
        Router::new()
            .merge(app_order_router_with_postgres_pool(
                pool.clone(),
                registry.clone(),
                credentials.clone(),
            ))
            .merge(app_checkout_router_with_postgres_pool(pool.clone()))
            .merge(app_recharge_checkout_router_with_postgres_pool(
                pool.clone(),
                registry,
                credentials,
            ))
            .merge(app_membership_order_router_with_postgres_pool(pool.clone()))
            .merge(app_fulfillment_router_with_postgres_pool(pool.clone()))
            .merge(app_shipment_router_with_postgres_pool(pool.clone()))
            .merge(app_after_sales_router_with_postgres_pool(pool.clone()))
            .merge(app_payment_webhook_router_with_postgres_pool(
                pool.clone(),
                Arc::new(NoopAccountPointsCreditPort),
                Arc::new(NoopAccountValueLedgerPort),
                Arc::new(NoopMembershipPurchaseFulfillmentPort),
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

    for (template_path, methods) in paths {
        for method_name in methods.as_object().unwrap().keys() {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method_from_openapi(method_name))
                        .uri(concrete_uri(template_path))
                        .header("content-type", "application/json")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_ne!(
                response.status(),
                StatusCode::NOT_FOUND,
                "{method_name} {template_path} is not mounted"
            );
        }
    }
}

fn method_from_openapi(method_name: &str) -> Method {
    match method_name.to_ascii_lowercase().as_str() {
        "get" => Method::GET,
        "post" => Method::POST,
        "put" => Method::PUT,
        "patch" => Method::PATCH,
        "delete" => Method::DELETE,
        other => panic!("unsupported openapi method: {other}"),
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
}
