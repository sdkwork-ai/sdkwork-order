//! PSP refund webhooks are owned by order-app-api with their own URL and
//! flow system, independent from payment webhooks:
//!
//! `POST /app/v3/api/orders/refunds/webhooks/{providerCode}`
//!
//! This module is a thin mount over the unified [`ProviderWebhookFramework`]
//! with the `RefundOnly` family policy: a misrouted payment event on this URL
//! is rejected with an audit record exactly as before, while refund events
//! run the identical pipeline stages as the canonical payment intake.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Extension, Path, State};
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use sdkwork_order_integration_payment::StorePaymentNotifyPorts;
use sdkwork_order_service::{default_refund_notify_handler_registry, RefundNotifyHandlerRegistry};
use sdkwork_payment_providers::ProviderCredentialBundle;
use sdkwork_web_core::WebRequestContext;
use sqlx::PgPool;

use crate::payment_webhook_framework::{
    collect_webhook_intake, ProviderWebhookFramework, WebhookFamilyPolicy, WEBHOOK_BODY_MAX_BYTES,
};

/// Maximum provider refund notification body size (same bound as payment).
pub const REFUND_WEBHOOK_BODY_MAX_BYTES: usize = WEBHOOK_BODY_MAX_BYTES;

#[derive(Clone)]
struct RefundWebhookState {
    framework: Arc<ProviderWebhookFramework>,
}

pub fn app_refund_webhook_router_with_postgres_pool(pool: PgPool) -> Router {
    app_refund_webhook_router_with_postgres_pool_and_registries(pool, None)
}

/// Extension seam mirroring the payment webhook router: deployments inject
/// their refund post-processing handlers here; `None` falls back to the
/// default (empty) registry.
pub fn app_refund_webhook_router_with_postgres_pool_and_registries(
    pool: PgPool,
    refund_notify_handler_registry: Option<Arc<dyn RefundNotifyHandlerRegistry>>,
) -> Router {
    app_refund_webhook_router_with_postgres_pool_and_handlers(
        pool,
        None,
        None,
        refund_notify_handler_registry,
    )
}

/// Full post-processing mount: the default refund registry settles
/// request-backed refunds (account hold settle/release + request
/// terminalization) from the webhook outcome, using the deployment's ledger
/// and physical inventory ports. A custom registry replaces the default
/// wholesale.
pub fn app_refund_webhook_router_with_postgres_pool_and_handlers(
    pool: PgPool,
    account_value_ledger_port: Option<Arc<dyn sdkwork_order_service::AccountValueLedgerPort>>,
    physical_inventory_port: Option<
        Arc<dyn sdkwork_order_service::PhysicalInventoryReservationPort>,
    >,
    refund_notify_handler_registry: Option<Arc<dyn RefundNotifyHandlerRegistry>>,
) -> Router {
    let credentials = ProviderCredentialBundle::from_env();
    let deployment_registry = Arc::new(
        sdkwork_payment_providers::PaymentProviderRegistry::from_credentials(credentials.clone()),
    );
    let default_registry = match account_value_ledger_port {
        Some(ledger) => sdkwork_order_service::refund_notify_handler_registry_with(
            Arc::new(
                sdkwork_order_repository_sqlx::PostgresRefundRequestSettlementStore::new(
                    pool.clone(),
                ),
            ),
            ledger,
            physical_inventory_port,
            Some(Arc::new(
                sdkwork_order_repository_sqlx::PostgresCommerceRechargeStore::new(pool.clone()),
            )
                as Arc<
                    dyn sdkwork_order_service::AccountValueRequestExecutionStore,
                >),
        ),
        None => default_refund_notify_handler_registry(),
    };
    let framework = Arc::new(ProviderWebhookFramework::new_refund_only(
        pool.clone(),
        refund_notify_handler_registry.unwrap_or(default_registry),
    ));
    Router::new()
        .route(
            "/app/v3/api/orders/refunds/webhooks/{providerCode}",
            post(receive_provider_refund_webhook),
        )
        .with_state(RefundWebhookState { framework })
        .layer(axum::extract::DefaultBodyLimit::max(
            REFUND_WEBHOOK_BODY_MAX_BYTES,
        ))
        .layer(axum::Extension(StorePaymentNotifyPorts::postgres(
            pool,
            credentials,
            deployment_registry,
        )))
}

async fn receive_provider_refund_webhook(
    State(state): State<RefundWebhookState>,
    Extension(ports): Extension<StorePaymentNotifyPorts>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(provider_code): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let header_pairs = collect_webhook_intake(&headers);
    state
        .framework
        .receive(
            ctx,
            &ports,
            &provider_code,
            &header_pairs,
            &body,
            WebhookFamilyPolicy::RefundOnly,
        )
        .await
}
