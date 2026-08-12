//! HTTP-level tests for the payment/refund webhook routers: body limits,
//! event-type routing guards, and registry seams. DB-backed assertions skip
//! when `SDKWORK_DATABASE_TEST_POSTGRES_URL` is not configured.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use sdkwork_order_repository_sqlx::order_points_recharge_e2e_postgres_pool_from_env;
use sdkwork_order_service::{
    default_payment_notify_handler_registry, AccountPointsCreditFuture, AccountPointsCreditPort,
    NoopAccountValueLedgerPort, NoopCouponRedemptionPort, NoopMembershipPurchaseFulfillmentPort,
    PointsRechargeCreditOutcome, PointsRechargeCreditRequest,
};
use sdkwork_routes_order_app_api::{
    app_payment_webhook_router_with_postgres_pool,
    app_payment_webhook_router_with_postgres_pool_and_integrations_and_registries,
    app_refund_webhook_router_with_postgres_pool,
    app_refund_webhook_router_with_postgres_pool_and_registries,
};
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

fn payment_router(pool: sqlx::PgPool) -> axum::Router {
    app_payment_webhook_router_with_postgres_pool(
        pool,
        Arc::new(NoopAccountPointsCreditPort),
        Arc::new(NoopAccountValueLedgerPort),
        Arc::new(NoopMembershipPurchaseFulfillmentPort),
    )
}

fn refund_router(pool: sqlx::PgPool) -> axum::Router {
    app_refund_webhook_router_with_postgres_pool(pool)
}

#[tokio::test]
async fn oversized_payment_webhook_body_is_rejected_with_413_before_any_processing() {
    // DefaultBodyLimit(512 KiB) applies at extraction, before signature work
    // or persistence; the lazy pool never needs a real connection here.
    let pool = sqlx::PgPool::connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .expect("lazy pool");
    let router = payment_router(pool);
    let oversized = vec![b'x'; 512 * 1024 + 1];

    let response = router
        .oneshot(
            Request::builder()
                .method(axum::http::Method::POST)
                .uri("/app/v3/api/orders/payments/webhooks/sandbox")
                .header("content-type", "application/json")
                .body(Body::from(oversized))
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(StatusCode::PAYLOAD_TOO_LARGE, response.status());
}

#[tokio::test]
async fn oversized_refund_webhook_body_is_rejected_with_413() {
    let pool = sqlx::PgPool::connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .expect("lazy pool");
    let router = refund_router(pool);
    let oversized = vec![b'x'; 512 * 1024 + 1];

    let response = router
        .oneshot(
            Request::builder()
                .method(axum::http::Method::POST)
                .uri("/app/v3/api/orders/refunds/webhooks/sandbox")
                .header("content-type", "application/json")
                .body(Body::from(oversized))
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(StatusCode::PAYLOAD_TOO_LARGE, response.status());
}

#[tokio::test]
async fn refund_url_rejects_payment_events_with_400_not_silent_ack() {
    let Some(pool) = order_points_recharge_e2e_postgres_pool_from_env().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    let router = refund_router(pool);

    let response = router
        .oneshot(
            Request::builder()
                .method(axum::http::Method::POST)
                .uri("/app/v3/api/orders/refunds/webhooks/sandbox")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"eventType":"payment.succeeded","outTradeNo":"t-1"}"#,
                ))
                .unwrap(),
        )
        .await
        .expect("request");

    // A payment event on the refund URL is a PSP misconfiguration: 400 with
    // a rejected audit record — never a 2xx silent ack.
    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn registry_seam_variants_are_constructible() {
    // Compile-level verification of the extension seam: deployments can pass
    // custom registries without forking the routes crate.
    let pool = sqlx::PgPool::connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .expect("lazy pool");
    let registries_router =
        app_payment_webhook_router_with_postgres_pool_and_integrations_and_registries(
            pool.clone(),
            Arc::new(NoopAccountPointsCreditPort),
            Arc::new(NoopAccountValueLedgerPort),
            Arc::new(NoopCouponRedemptionPort),
            Arc::new(NoopMembershipPurchaseFulfillmentPort),
            Arc::new(sdkwork_order_service::UnavailablePhysicalGoodsFulfillmentPort),
            Some(default_payment_notify_handler_registry()),
            None,
        );
    let _ = registries_router;

    let refund_registries_router = app_refund_webhook_router_with_postgres_pool_and_registries(
        pool,
        Some(Arc::new(
            sdkwork_order_service::DefaultRefundNotifyHandlerRegistry::new(),
        )),
    );
    let _ = refund_registries_router;
}
