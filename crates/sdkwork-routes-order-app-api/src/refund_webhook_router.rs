//! PSP refund webhooks are owned by order-app-api with their own URL and
//! flow system, independent from payment webhooks:
//!
//! `POST /app/v3/api/orders/refunds/webhooks/{providerCode}`
//!
//! This module is a thin HTTP adapter: it reuses the shared verification and
//! refund-ingest ports (`StorePaymentNotifyPorts` in
//! sdkwork-order-integration-payment), rejects non-refund events with an
//! audit record, and delegates the whole decision chain to
//! `process_refund_notify_verified` in sdkwork-order-service.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Extension, Path, State};
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_integration_payment::StorePaymentNotifyPorts;
use sdkwork_order_repository_sqlx::PostgresCommerceOrderStore;
use sdkwork_order_service::{
    default_refund_notify_handler_registry, is_refund_event_type, process_refund_notify_verified,
    verify_and_normalize_event, RefundNotifyHandlerRegistry,
};
use sdkwork_payment_providers::{normalize_provider_code, ProviderCredentialBundle};
use sdkwork_payment_repository_sqlx::record_rejected_provider_webhook_postgres;
use sdkwork_web_core::WebRequestContext;
use sqlx::PgPool;

use crate::api_response::{map_webhook_service_error, success_command};

#[derive(Clone)]
struct RefundWebhookState {
    orders: Arc<PostgresCommerceOrderStore>,
}

/// Maximum provider refund notification body size (same bound as payment).
pub const REFUND_WEBHOOK_BODY_MAX_BYTES: usize = 512 * 1024;

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
    let credentials = ProviderCredentialBundle::from_env();
    let deployment_registry = Arc::new(
        sdkwork_payment_providers::PaymentProviderRegistry::from_credentials(credentials.clone()),
    );
    Router::new()
        .route(
            "/app/v3/api/orders/refunds/webhooks/{providerCode}",
            post(receive_provider_refund_webhook),
        )
        .with_state(RefundWebhookState {
            orders: Arc::new(PostgresCommerceOrderStore::new(pool.clone())),
        })
        .layer(axum::extract::DefaultBodyLimit::max(
            REFUND_WEBHOOK_BODY_MAX_BYTES,
        ))
        .layer(axum::Extension(StorePaymentNotifyPorts::postgres(
            pool,
            credentials,
            deployment_registry,
        )))
        .layer(axum::Extension(RefundWebhookRegistries {
            refund: refund_notify_handler_registry
                .unwrap_or_else(default_refund_notify_handler_registry),
        }))
}

#[derive(Clone)]
struct RefundWebhookRegistries {
    refund: Arc<dyn RefundNotifyHandlerRegistry>,
}

async fn receive_provider_refund_webhook(
    State(state): State<RefundWebhookState>,
    Extension(ports): Extension<StorePaymentNotifyPorts>,
    Extension(registries): Extension<RefundWebhookRegistries>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(provider_code): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let header_pairs = headers
        .iter()
        .filter_map(|(name, value)| {
            Some((name.as_str().to_owned(), value.to_str().ok()?.to_owned()))
        })
        .collect::<Vec<_>>();

    let provider_code = normalize_provider_code(&provider_code);
    match verify_and_normalize_event(&ports, &provider_code, &header_pairs, &body).await {
        Ok(event) if is_refund_event_type(event.event_type.as_deref()) => {
            match process_refund_notify_verified(
                event,
                &ports,
                state.orders.as_ref(),
                registries.refund.as_ref(),
            )
            .await
            {
                Ok(outcome) => {
                    success_command(ctx, Some(outcome.webhook_event_id), Some(outcome.status))
                }
                Err(error) => map_webhook_service_error(ctx, error),
            }
        }
        Ok(_) => {
            // The refund URL only serves refund notifications; a payment event
            // here means a PSP misconfiguration. Reject with an audit record
            // instead of silently acking.
            let reason = "payment event delivered to the refund webhook url";
            tracing::warn!(target = "order.refund_notify", provider_code, reason);
            if let Err(error) = record_rejected_provider_webhook_postgres(
                ports.pool(),
                &provider_code,
                &body,
                reason,
            )
            .await
            {
                tracing::error!(
                    target = "order.refund_notify",
                    provider_code,
                    error = ?error,
                    "failed to record rejected webhook"
                );
            }
            map_webhook_service_error(ctx, CommerceServiceError::validation("bad request"))
        }
        Err(error) => map_webhook_service_error(ctx, error),
    }
}
