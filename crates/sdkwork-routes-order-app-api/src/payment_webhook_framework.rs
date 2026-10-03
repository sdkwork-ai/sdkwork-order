//! Unified PSP webhook framework（统一回调框架）.
//!
//! Every provider notification — any channel, any payment method, any event
//! family — lands through ONE pipeline mounted on ONE canonical path-variable
//! URL space:
//!
//! ```text
//! POST /app/v3/api/orders/payments/webhooks/{providerCode}   (all families)
//! POST /app/v3/api/orders/refunds/webhooks/{providerCode}    (refund only)
//! ```
//!
//! The `{providerCode}` path variable selects the channel adapter; the event
//! family (payment / refund / …) is classified from the verified payload, not
//! from the URL, so single-URL providers (WeChat Pay v3) and multi-URL
//! providers (Stripe, Alipay, PayPal) flow through identical stages:
//!
//! intake → tenant/account resolution → adapter verification → normalization
//! → family classification → family policy check → idempotent ingest →
//! business dispatch (handler registries) → standardized HTTP ack.
//!
//! Extension points (open-closed): new channels register adapters in the
//! payment provider registry; new business handlers join the payment/refund
//! registries; new event families add a variant to [`WebhookEventFamily`] and
//! one dispatch arm. None of the existing stages change.

use std::sync::Arc;

use axum::body::Bytes;
use axum::response::Response;
use sdkwork_order_integration_payment::StorePaymentNotifyPorts;
use sdkwork_order_repository_sqlx::{PostgresCommerceOrderStore, PostgresCommerceRechargeStore};
use sdkwork_order_service::{
    is_refund_event_type, process_payment_notify_verified, process_refund_notify_verified,
    AccountPointsCreditPort, AccountValueLedgerPort, AppTemplatePurchaseFulfillmentPort,
    CouponRedemptionPort, MembershipPurchaseFulfillmentPort, OwnerOrderSettlementPorts,
    PaymentNotifyHandlerRegistry, PhysicalGoodsFulfillmentPort, RefundNotifyHandlerRegistry,
};
use sdkwork_payment_providers::normalize_provider_code;
use sdkwork_payment_repository_sqlx::PostgresCommerceOwnerOrderPaymentStore;
use sdkwork_web_core::WebRequestContext;

use crate::api_response::{map_webhook_service_error, success_command};

/// Maximum provider notification body size — the single bound applied at
/// every intake URL before any signature work or persistence.
pub const WEBHOOK_BODY_MAX_BYTES: usize = 512 * 1024;

/// Event family of a verified webhook, classified from the payload so
/// single-URL providers route correctly no matter which intake accepted the
/// delivery. Open set: dispute/payout/kyc families slot in here without
/// touching the pipeline stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookEventFamily {
    Payment,
    Refund,
}

/// Classifies a verified event by its provider event type.
pub fn classify_webhook_event(event_type: Option<&str>) -> WebhookEventFamily {
    if is_refund_event_type(event_type) {
        WebhookEventFamily::Refund
    } else {
        WebhookEventFamily::Payment
    }
}

/// Which families an intake URL accepts. The canonical payment URL accepts
/// everything (single-URL providers deliver refund notifications there too);
/// the dedicated refund URL keeps its stricter contract and rejects
/// misrouted payment events with an audit record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookFamilyPolicy {
    All,
    RefundOnly,
}

impl WebhookFamilyPolicy {
    fn admits(self, family: WebhookEventFamily) -> bool {
        match (self, family) {
            (WebhookFamilyPolicy::All, _) => true,
            (WebhookFamilyPolicy::RefundOnly, WebhookEventFamily::Refund) => true,
            (WebhookFamilyPolicy::RefundOnly, WebhookEventFamily::Payment) => false,
        }
    }
}

/// Payment-family dispatch dependencies. Held behind [`Option`] so the
/// refund-only intake mount does not need to fabricate payment-side ports:
/// the family policy rejects payment events before dispatch is reached.
struct PaymentDispatchPorts {
    payments: Arc<PostgresCommerceOwnerOrderPaymentStore>,
    recharge: Arc<PostgresCommerceRechargeStore>,
    credit_port: Arc<dyn AccountPointsCreditPort>,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
    membership_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
    app_template_port: Arc<dyn AppTemplatePurchaseFulfillmentPort>,
    physical_goods_port: Arc<dyn PhysicalGoodsFulfillmentPort>,
    payment_registry: Arc<dyn PaymentNotifyHandlerRegistry>,
}

/// Shared state of the unified webhook pipeline: the settlement ports, the
/// business handler registries, and the notify ports live here so both
/// intake URLs are two-line mounts over identical machinery.
pub struct ProviderWebhookFramework {
    payment_dispatch: Option<PaymentDispatchPorts>,
    orders: Arc<PostgresCommerceOrderStore>,
    refund_registry: Arc<dyn RefundNotifyHandlerRegistry>,
}

impl ProviderWebhookFramework {
    /// Assembles the full framework (all families) from the concrete
    /// Postgres stores and the injected business handler registries
    /// (extension seam; `None` falls back to the defaults).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool: sqlx::PgPool,
        credit_port: Arc<dyn AccountPointsCreditPort>,
        account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
        coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
        membership_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
        app_template_port: Arc<dyn AppTemplatePurchaseFulfillmentPort>,
        physical_goods_port: Arc<dyn PhysicalGoodsFulfillmentPort>,
        payment_registry: Arc<dyn PaymentNotifyHandlerRegistry>,
        refund_registry: Arc<dyn RefundNotifyHandlerRegistry>,
    ) -> Self {
        Self {
            payment_dispatch: Some(PaymentDispatchPorts {
                payments: Arc::new(PostgresCommerceOwnerOrderPaymentStore::new(pool.clone())),
                recharge: Arc::new(PostgresCommerceRechargeStore::new(pool.clone())),
                credit_port,
                account_value_ledger_port,
                coupon_redemption_port,
                membership_port,
                app_template_port,
                physical_goods_port,
                payment_registry,
            }),
            orders: Arc::new(PostgresCommerceOrderStore::new(pool)),
            refund_registry,
        }
    }

    /// Refund-only assembly: serves the dedicated refund intake URL without
    /// payment-side dispatch ports (the `RefundOnly` family policy makes
    /// them unreachable).
    pub fn new_refund_only(
        pool: sqlx::PgPool,
        refund_registry: Arc<dyn RefundNotifyHandlerRegistry>,
    ) -> Self {
        Self {
            payment_dispatch: None,
            orders: Arc::new(PostgresCommerceOrderStore::new(pool)),
            refund_registry,
        }
    }

    /// The ONE pipeline every intake URL runs. `intake_provider_code` is the
    /// `{providerCode}` path variable; `policy` is the per-URL family
    /// contract.
    pub async fn receive(
        &self,
        ctx: Option<&WebRequestContext>,
        ports: &StorePaymentNotifyPorts,
        intake_provider_code: &str,
        header_pairs: &[(String, String)],
        body: &Bytes,
        policy: WebhookFamilyPolicy,
    ) -> Response {
        let provider_code = normalize_provider_code(intake_provider_code);
        let event = match sdkwork_order_service::verify_and_normalize_event(
            ports,
            &provider_code,
            header_pairs,
            body,
        )
        .await
        {
            Ok(event) => event,
            Err(error) => return map_webhook_service_error(ctx, error),
        };
        let family = classify_webhook_event(event.event_type.as_deref());
        if !policy.admits(family) {
            self.reject_misrouted_event(ports, &provider_code, body, family)
                .await;
            return map_webhook_service_error(
                ctx,
                sdkwork_contract_service::CommerceServiceError::validation("bad request"),
            );
        }
        match family {
            WebhookEventFamily::Refund => {
                match process_refund_notify_verified(
                    event,
                    ports,
                    self.orders.as_ref(),
                    self.refund_registry.as_ref(),
                )
                .await
                {
                    Ok(outcome) => {
                        success_command(ctx, Some(outcome.webhook_event_id), Some(outcome.status))
                    }
                    Err(error) => map_webhook_service_error(ctx, error),
                }
            }
            WebhookEventFamily::Payment => {
                let Some(dispatch) = self.payment_dispatch.as_ref() else {
                    // Unreachable behind the RefundOnly policy check above;
                    // defensive so a policy bug can never panic a PSP ack.
                    return map_webhook_service_error(
                        ctx,
                        sdkwork_contract_service::CommerceServiceError::validation(
                            "payment events are not served by this webhook intake",
                        ),
                    );
                };
                let settlement_ports = OwnerOrderSettlementPorts {
                    payment_store: dispatch.payments.as_ref(),
                    order_state_store: self.orders.as_ref(),
                    recharge_store: dispatch.recharge.as_ref(),
                    account_value_store: dispatch.recharge.as_ref(),
                    credit_port: dispatch.credit_port.as_ref(),
                    account_value_ledger_port: dispatch.account_value_ledger_port.as_ref(),
                    coupon_redemption_port: dispatch.coupon_redemption_port.as_ref(),
                    membership_port: dispatch.membership_port.as_ref(),
                    app_template_port: dispatch.app_template_port.as_ref(),
                    physical_goods_port: dispatch.physical_goods_port.as_ref(),
                };
                match process_payment_notify_verified(
                    event,
                    ports,
                    ports,
                    settlement_ports,
                    dispatch.payment_registry.as_ref(),
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
    }

    /// A delivery whose family the intake URL does not serve is a PSP
    /// misconfiguration: audit-record it instead of silently acking.
    async fn reject_misrouted_event(
        &self,
        ports: &StorePaymentNotifyPorts,
        provider_code: &str,
        body: &Bytes,
        family: WebhookEventFamily,
    ) {
        let reason = format!("{family:?} event delivered to a non-matching webhook url");
        tracing::warn!(target = "order.payment_webhook", provider_code, reason);
        if let Err(error) =
            sdkwork_payment_repository_sqlx::record_rejected_provider_webhook_postgres(
                ports.pool(),
                provider_code,
                body,
                &reason,
            )
            .await
        {
            tracing::error!(
                target = "order.payment_webhook",
                provider_code,
                error = ?error,
                "failed to record rejected webhook"
            );
        }
    }
}

/// Collects the HTTP layer into the transport-neutral intake the pipeline
/// consumes (identical for every channel and family).
pub fn collect_webhook_intake(headers: &axum::http::HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            Some((name.as_str().to_owned(), value.to_str().ok()?.to_owned()))
        })
        .collect()
}
