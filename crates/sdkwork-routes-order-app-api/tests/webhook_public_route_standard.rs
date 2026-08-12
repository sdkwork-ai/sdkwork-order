//! PSP webhook callback routes must be reachable WITHOUT platform login or
//! dual-token credentials: the PSP calls the notify URL with its own
//! signature headers, never with a platform session. These tests compose the
//! gateway exactly like the standalone entrypoint (IAM resolver + web
//! framework + route manifest) and prove the webhook endpoints are public
//! while ordinary app-api routes still require dual-token auth.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use sdkwork_iam_web_adapter::{build_web_framework_builder, IamWebRequestContextResolver};
use sdkwork_order_service::{NoopAccountValueLedgerPort, NoopMembershipPurchaseFulfillmentPort};
use sdkwork_routes_order_app_api::{
    app_payment_webhook_router_with_postgres_pool, http_route_manifest::app_route_manifest,
};
use sdkwork_web_bootstrap::{ApiAssemblyContribution, ComposedApiAssembly};
use sqlx::postgres::PgPoolOptions;
use tower::util::ServiceExt;

fn composed_gateway() -> axum::Router {
    // The IAM adapter resolves the web environment from process env and
    // applies production defaults when unset; the test pins Dev so the
    // assembly builds without a production audit emitter.
    std::env::set_var("SDKWORK_ENVIRONMENT", "dev");
    let manifest = app_route_manifest();
    let router = Router::new().merge(app_payment_webhook_router_with_postgres_pool(
        unreachable_pool_placeholder(),
        Arc::new(NoopAccountPointsCreditPort),
        Arc::new(NoopAccountValueLedgerPort),
        Arc::new(NoopMembershipPurchaseFulfillmentPort),
    ));
    let contribution = ApiAssemblyContribution::from_manifest(
        "sdkwork-order-test",
        "Order Test API",
        router,
        manifest.clone(),
        Vec::new(),
        Arc::new(sdkwork_web_bootstrap::AlwaysReady),
    )
    .expect("test contribution must build");
    let framework = build_web_framework_builder(
        IamWebRequestContextResolver::from_database_pool(None),
        manifest,
        Vec::new(),
    );
    ComposedApiAssembly::try_compose("Order Test API", vec![contribution])
        .expect("assembly must compose")
        .into_hosted(framework)
        .router
}

/// The webhook handler does not touch the pool on the anonymous no-attempt
/// path used by this test (verification resolves the provider registry from
/// env credentials and sandbox never queries storage); the pool exists only
/// to satisfy the constructor.
fn unreachable_pool_placeholder() -> sqlx::PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost:5432/nonexistent")
        .expect("lazy pool construction must not connect")
}

struct NoopAccountPointsCreditPort;

impl sdkwork_order_service::AccountPointsCreditPort for NoopAccountPointsCreditPort {
    fn credit_points_recharge<'a>(
        &'a self,
        _request: sdkwork_order_service::PointsRechargeCreditRequest,
    ) -> sdkwork_order_service::AccountPointsCreditFuture<
        'a,
        sdkwork_order_service::PointsRechargeCreditOutcome,
    > {
        Box::pin(async move {
            Ok(sdkwork_order_service::PointsRechargeCreditOutcome {
                accepted: true,
                replayed: false,
            })
        })
    }

    fn reverse_points_recharge_credit<'a>(
        &'a self,
        _request: sdkwork_order_service::PointsRechargeCreditRequest,
    ) -> sdkwork_order_service::AccountPointsCreditFuture<
        'a,
        sdkwork_order_service::PointsRechargeCreditOutcome,
    > {
        Box::pin(async move {
            Ok(sdkwork_order_service::PointsRechargeCreditOutcome {
                accepted: true,
                replayed: false,
            })
        })
    }
}

#[tokio::test]
async fn payment_webhook_is_reachable_without_any_platform_credentials() {
    let app = composed_gateway();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/orders/payments/webhooks/sandbox")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"event":"sandbox.payment.succeeded"}"#))
                .unwrap(),
        )
        .await
        .expect("webhook request must not be blocked by the framework");
    // Auth must pass (no 401/403): the sandbox provider normalizes the
    // envelope and the pipeline acks the unmatched trade for forensics.
    assert_ne!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "PSP payment notify must not require platform login or dual tokens"
    );
    assert_ne!(
        response.status(),
        StatusCode::FORBIDDEN,
        "PSP payment notify must not require platform login or dual tokens"
    );
}

#[tokio::test]
async fn refund_webhook_is_reachable_without_any_platform_credentials() {
    std::env::set_var("SDKWORK_ENVIRONMENT", "dev");
    let manifest = app_route_manifest();
    let router = Router::new().merge(
        sdkwork_routes_order_app_api::app_refund_webhook_router_with_postgres_pool(
            unreachable_pool_placeholder(),
        ),
    );
    let contribution = ApiAssemblyContribution::from_manifest(
        "sdkwork-order-test-refund",
        "Order Test Refund API",
        router,
        manifest.clone(),
        Vec::new(),
        Arc::new(sdkwork_web_bootstrap::AlwaysReady),
    )
    .expect("test contribution must build");
    let framework = build_web_framework_builder(
        IamWebRequestContextResolver::from_database_pool(None),
        manifest,
        Vec::new(),
    );
    let app = ComposedApiAssembly::try_compose("Order Test Refund API", vec![contribution])
        .expect("assembly must compose")
        .into_hosted(framework)
        .router;
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/orders/refunds/webhooks/sandbox")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"event":"sandbox.refund.succeeded"}"#))
                .unwrap(),
        )
        .await
        .expect("refund webhook request must not be blocked by the framework");
    assert_ne!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "PSP refund notify must not require platform login or dual tokens"
    );
    assert_ne!(
        response.status(),
        StatusCode::FORBIDDEN,
        "PSP refund notify must not require platform login or dual tokens"
    );
}

#[tokio::test]
async fn protected_app_route_still_requires_dual_token_without_credentials() {
    std::env::set_var("SDKWORK_ENVIRONMENT", "dev");
    let manifest = app_route_manifest();
    let router = Router::new();
    let contribution = ApiAssemblyContribution::from_manifest(
        "sdkwork-order-test-protected",
        "Order Test Protected API",
        router,
        manifest.clone(),
        Vec::new(),
        Arc::new(sdkwork_web_bootstrap::AlwaysReady),
    )
    .expect("test contribution must build");
    let framework = build_web_framework_builder(
        IamWebRequestContextResolver::from_database_pool(None),
        manifest,
        Vec::new(),
    );
    let app = ComposedApiAssembly::try_compose("Order Test Protected API", vec![contribution])
        .expect("assembly must compose")
        .into_hosted(framework)
        .router;
    // A protected app-api route without credentials must be rejected by the
    // framework auth chain (the dual-token/login contract stays intact).
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/app/v3/api/orders")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("protected request must reach the framework");
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "protected app-api routes must keep requiring dual-token credentials"
    );
}
