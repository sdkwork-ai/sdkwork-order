//! PSP payment webhooks are owned by order-app-api (Order → Payment ingest → in-process settlement).
//!
//! This module is a thin HTTP adapter: it resolves the provider registry
//! through the shared `StorePaymentNotifyPorts` adapter
//! (sdkwork-order-integration-payment), verifies and normalizes the event
//! once, routes by event type (single-URL providers deliver refund
//! notifications here too), and delegates the decision chains to
//! `process_payment_notify_verified` / `process_refund_notify_verified` in
//! sdkwork-order-service.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Extension, Path, State};
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_integration_payment::StorePaymentNotifyPorts;
use sdkwork_order_repository_sqlx::{PostgresCommerceOrderStore, PostgresCommerceRechargeStore};
use sdkwork_order_service::{
    default_payment_notify_handler_registry, default_refund_notify_handler_registry,
    is_refund_event_type, process_payment_notify_verified, process_refund_notify_verified,
    verify_and_normalize_event, AccountPointsCreditPort, AccountValueLedgerPort,
    CouponRedemptionPort, MembershipPurchaseFulfillmentPort, NoopCouponRedemptionPort,
    OwnerOrderSettlementPorts, PaymentNotifyHandlerRegistry, PhysicalGoodsFulfillmentPort,
    RefundNotifyHandlerRegistry, UnavailablePhysicalGoodsFulfillmentPort,
};
use sdkwork_payment_providers::{
    normalize_provider_code, PaymentProviderRegistry, ProviderCredentialBundle,
};
use sdkwork_payment_repository_sqlx::PostgresCommerceOwnerOrderPaymentStore;
use sdkwork_web_core::WebRequestContext;
use sqlx::PgPool;

use crate::api_response::{map_webhook_service_error, success_command};

/// Maximum provider notification body size. Explicit and bounded so oversized
/// forged payloads are rejected before any signature work or persistence.
pub const PAYMENT_WEBHOOK_BODY_MAX_BYTES: usize = 512 * 1024;

#[derive(Clone)]
enum PaymentWebhookState {
    Postgres {
        payments: Arc<PostgresCommerceOwnerOrderPaymentStore>,
        recharge: Arc<PostgresCommerceRechargeStore>,
        orders: Arc<PostgresCommerceOrderStore>,
        credit_port: Arc<dyn AccountPointsCreditPort>,
        account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
        coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
        membership_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
        physical_goods_port: Arc<dyn PhysicalGoodsFulfillmentPort>,
    },
}

/// Injected business handler registries (extension seam). Deployments pass
/// custom registries through
/// `app_payment_webhook_router_with_postgres_pool_and_integrations_and_registries`;
/// `None` falls back to the defaults.
#[derive(Clone)]
struct WebhookHandlerRegistries {
    payment: Arc<dyn PaymentNotifyHandlerRegistry>,
    refund: Arc<dyn RefundNotifyHandlerRegistry>,
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
    Router::new()
        .route(
            "/app/v3/api/orders/payments/webhooks/{providerCode}",
            post(receive_provider_webhook),
        )
        .with_state(PaymentWebhookState::Postgres {
            payments: Arc::new(PostgresCommerceOwnerOrderPaymentStore::new(pool.clone())),
            recharge: Arc::new(PostgresCommerceRechargeStore::new(pool.clone())),
            orders: Arc::new(PostgresCommerceOrderStore::new(pool.clone())),
            credit_port,
            account_value_ledger_port,
            coupon_redemption_port,
            membership_port,
            physical_goods_port,
        })
        .layer(axum::extract::DefaultBodyLimit::max(
            PAYMENT_WEBHOOK_BODY_MAX_BYTES,
        ))
        .layer(axum::Extension(StorePaymentNotifyPorts::postgres(
            pool,
            credentials,
            deployment_registry,
        )))
        .layer(axum::Extension(WebhookHandlerRegistries {
            payment: payment_notify_handler_registry
                .unwrap_or_else(default_payment_notify_handler_registry),
            refund: refund_notify_handler_registry
                .unwrap_or_else(default_refund_notify_handler_registry),
        }))
}

async fn receive_provider_webhook(
    State(state): State<PaymentWebhookState>,
    Extension(ports): Extension<StorePaymentNotifyPorts>,
    Extension(registries): Extension<WebhookHandlerRegistries>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(provider_code): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    match state {
        PaymentWebhookState::Postgres {
            payments,
            recharge,
            orders,
            credit_port,
            account_value_ledger_port,
            coupon_redemption_port,
            membership_port,
            physical_goods_port,
        } => {
            let header_pairs = headers
                .iter()
                .filter_map(|(name, value)| {
                    Some((name.as_str().to_owned(), value.to_str().ok()?.to_owned()))
                })
                .collect::<Vec<_>>();
            let settlement_ports = OwnerOrderSettlementPorts {
                payment_store: payments.as_ref(),
                order_state_store: orders.as_ref(),
                recharge_store: recharge.as_ref(),
                account_value_store: recharge.as_ref(),
                credit_port: credit_port.as_ref(),
                account_value_ledger_port: account_value_ledger_port.as_ref(),
                coupon_redemption_port: coupon_redemption_port.as_ref(),
                membership_port: membership_port.as_ref(),
                physical_goods_port: physical_goods_port.as_ref(),
            };
            // Single-URL providers (WeChat API v3) deliver payment AND refund
            // notifications to the payment webhook URL; route by event type so
            // refund events are never acked unprocessed.
            match verify_and_normalize_event(
                &ports,
                &normalize_provider_code(&provider_code),
                &header_pairs,
                &body,
            )
            .await
            {
                Ok(event) => {
                    if is_refund_event_type(event.event_type.as_deref()) {
                        match process_refund_notify_verified(
                            event,
                            &ports,
                            orders.as_ref(),
                            registries.refund.as_ref(),
                        )
                        .await
                        {
                            Ok(outcome) => success_command(
                                ctx,
                                Some(outcome.webhook_event_id),
                                Some(outcome.status),
                            ),
                            Err(error) => map_webhook_service_error(ctx, error),
                        }
                    } else {
                        match process_payment_notify_verified(
                            event,
                            &ports,
                            &ports,
                            settlement_ports,
                            registries.payment.as_ref(),
                        )
                        .await
                        {
                            Ok(outcome) => success_command(
                                ctx,
                                outcome
                                    .payment_attempt_id
                                    .or(Some(outcome.webhook_event_id)),
                                Some(outcome.status),
                            ),
                            Err(error) => map_webhook_service_error(ctx, error),
                        }
                    }
                }
                Err(error) => map_webhook_service_error(ctx, error),
            }
        }
    }
}
