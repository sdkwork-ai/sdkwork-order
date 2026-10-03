//! Fulfillment port for a settled app-template order.
//!
//! The deployments module owns the template catalog; this port is the seam the
//! order center uses to tell that owner "this order is paid, count the install".
//! The entitlement itself stays here in `commerce_order` — the adapter grants
//! nothing on the deployments side, it only advances the order's fulfillment
//! state and maintains the listing's install counter.

use std::future::Future;
use std::pin::Pin;

use sdkwork_contract_service::CommerceServiceError;

pub type AppTemplatePurchaseFulfillmentFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FulfillPaidAppTemplateOrderRequest {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
    pub paid_at: String,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppTemplatePurchaseFulfillmentOutcome {
    pub accepted: bool,
    pub replayed: bool,
    pub fulfillment_status: String,
    /// Install counter after the grant, when the adapter could read it back.
    pub install_count: Option<i64>,
    /// The template the fulfilled order entitles, when it could be resolved.
    pub template_uuid: Option<String>,
}

pub trait AppTemplatePurchaseFulfillmentPort: Send + Sync {
    fn fulfill_app_template_purchase<'a>(
        &'a self,
        request: FulfillPaidAppTemplateOrderRequest,
    ) -> AppTemplatePurchaseFulfillmentFuture<'a, AppTemplatePurchaseFulfillmentOutcome>;
}

/// Idempotency key for one order's template fulfillment. Provider webhook
/// redelivery replays the same key, so the grant happens exactly once.
pub fn app_template_purchase_fulfillment_idempotency_key(order_id: &str) -> String {
    format!("app-template-purchase:fulfill:{order_id}")
}

pub const APP_TEMPLATE_PURCHASE_FULFILLMENT_PORT: &str = "app_template.purchase.fulfillment";

/// No-op adapter used when the deployments integration is not wired at gateway
/// assembly. It defers instead of failing so the payment/order status
/// transition still completes and the compensation worker can retry.
pub struct NoopAppTemplatePurchaseFulfillmentPort;

impl AppTemplatePurchaseFulfillmentPort for NoopAppTemplatePurchaseFulfillmentPort {
    fn fulfill_app_template_purchase<'a>(
        &'a self,
        _request: FulfillPaidAppTemplateOrderRequest,
    ) -> AppTemplatePurchaseFulfillmentFuture<'a, AppTemplatePurchaseFulfillmentOutcome> {
        Box::pin(async move {
            Ok(AppTemplatePurchaseFulfillmentOutcome {
                accepted: false,
                replayed: false,
                fulfillment_status: "awaiting_external_fulfillment".to_owned(),
                install_count: None,
                template_uuid: None,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_template_purchase_fulfillment_idempotency_key, AppTemplatePurchaseFulfillmentPort,
        FulfillPaidAppTemplateOrderRequest, NoopAppTemplatePurchaseFulfillmentPort,
    };

    #[test]
    fn fulfillment_idempotency_key_is_per_order() {
        assert_eq!(
            app_template_purchase_fulfillment_idempotency_key("order-1"),
            "app-template-purchase:fulfill:order-1"
        );
    }

    #[tokio::test]
    async fn noop_port_defers_instead_of_failing_the_settlement() {
        let outcome = NoopAppTemplatePurchaseFulfillmentPort
            .fulfill_app_template_purchase(FulfillPaidAppTemplateOrderRequest {
                tenant_id: "tenant-1".to_owned(),
                organization_id: None,
                owner_user_id: "user-1".to_owned(),
                order_id: "order-1".to_owned(),
                paid_at: "2026-07-26T00:00:00Z".to_owned(),
                request_no: "request-1".to_owned(),
                idempotency_key: app_template_purchase_fulfillment_idempotency_key("order-1"),
            })
            .await
            .expect("noop port resolves");

        assert!(!outcome.accepted);
        assert_eq!(outcome.fulfillment_status, "awaiting_external_fulfillment");
    }
}
