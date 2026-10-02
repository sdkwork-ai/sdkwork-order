//! PSP payment webhooks are owned by order-app-api (Order → Payment ingest → in-process settlement).
//!
//! This module is a thin mount over the unified [`ProviderWebhookFramework`]:
//! the canonical all-families intake URL, with channel selection by the
//! `{providerCode}` path variable and event-family routing inside the
//! framework (single-URL providers deliver refund notifications here too).
//! All pipeline stages live in `payment_webhook_framework`; this file only
//! wires state and keeps the historical constructor surface stable.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Extension, Path, State};
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use sdkwork_order_integration_payment::StorePaymentNotifyPorts;
use sdkwork_order_service::{
    default_payment_notify_handler_registry, default_refund_notify_handler_registry,
    AccountPointsCreditPort, AccountValueLedgerPort, CouponRedemptionPort,
    MembershipPurchaseFulfillmentPort, NoopCouponRedemptionPort, PaymentNotifyHandlerRegistry,
    PhysicalGoodsFulfillmentPort, RefundNotifyHandlerRegistry, UnavailablePhysicalGoodsFulfillmentPort,
};
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_web_core::WebRequestContext;
use sqlx::PgPool;

use crate::payment_webhook_framework::{
    collect_webhook_intake, ProviderWebhookFramework, WebhookFamilyPolicy,
    WEBHOOK_BODY_MAX_BYTES,
};

/// Maximum provider notification body size. Explicit and bounded so oversized
/// forged payloads are rejected before any signature work or persistence.
pub const PAYMENT_WEBHOOK_BODY_MAX_BYTES: usize = WEBHOOK_BODY_MAX_BYTES;

#[derive(Clone)]
struct PaymentWebhookState {
    framework: Arc<ProviderWebhookFramework>,
}

pub fn app_payment_webhook_router_with_postgres_pool(
    pool: PgPool,
    credit_port: Arc<dyn AccountPointsCreditPort>,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    membership_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
) -> Router {
    app_payment_webhook_router_with_postgres_pool_and_coupon(
        pool,
        credit_port,
        account_value_ledger_port,
        Arc::new(NoopCouponRedemptionPort),
        membership_port,
    )
}

pub fn app_payment_webhook_router_with_postgres_pool_and_coupon(
    pool: PgPool,
    credit_port: Arc<dyn AccountPointsCreditPort>,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
    membership_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
) -> Router {
    app_payment_webhook_router_with_postgres_pool_and_integrations(
        pool,
        credit_port,
        account_value_ledger_port,
        coupon_redemption_port,
        membership_port,
        Arc::new(UnavailablePhysicalGoodsFulfillmentPort),
    )
}

pub fn app_payment_webhook_router_with_postgres_pool_and_integrations(
    pool: PgPool,
    credit_port: Arc<dyn AccountPointsCreditPort>,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
    membership_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
    physical_goods_port: Arc<dyn PhysicalGoodsFulfillmentPort>,
) -> Router {
    app_payment_webhook_router_with_postgres_pool_and_integrations_and_registries(
        pool,
        credit_port,
        account_value_ledger_port,
        coupon_redemption_port,
        membership_port,
        physical_goods_port,
        None,
        None,
    )
}

/// Extension seam: deployments inject their business handler registries
/// (payment fulfillment + refund post-processing) without forking the routes
/// crate. `None` falls back to the default registries.
#[allow(clippy::too_many_arguments)]
pub fn app_payment_webhook_router_with_postgres_pool_and_integrations_and_registries(
    pool: PgPool,
    credit_port: Arc<dyn AccountPointsCreditPort>,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
    membership_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
    physical_goods_port: Arc<dyn PhysicalGoodsFulfillmentPort>,
    payment_notify_handler_registry: Option<Arc<dyn PaymentNotifyHandlerRegistry>>,
    refund_notify_handler_registry: Option<Arc<dyn RefundNotifyHandlerRegistry>>,
) -> Router {
    let credentials = ProviderCredentialBundle::from_env();
    let deployment_registry = Arc::new(PaymentProviderRegistry::from_credentials(
        credentials.clone(),
    ));
    let framework = Arc::new(ProviderWebhookFramework::new(
        pool.clone(),
        credit_port,
        account_value_ledger_port,
        coupon_redemption_port,
        membership_port,
        physical_goods_port,
        payment_notify_handler_registry
            .unwrap_or_else(default_payment_notify_handler_registry),
        refund_notify_handler_registry.unwrap_or_else(default_refund_notify_handler_registry),
    ));
    Router::new()
        .route(
            "/app/v3/api/orders/payments/webhooks/{providerCode}",
            post(receive_provider_webhook),
        )
        .with_state(PaymentWebhookState { framework })
        .layer(axum::extract::DefaultBodyLimit::max(
            PAYMENT_WEBHOOK_BODY_MAX_BYTES,
        ))
        .layer(axum::Extension(StorePaymentNotifyPorts::postgres(
            pool,
            credentials,
            deployment_registry,
        )))
}

async fn receive_provider_webhook(
    State(state): State<PaymentWebhookState>,
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
            WebhookFamilyPolicy::All,
        )
        .await
}
