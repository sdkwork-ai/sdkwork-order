//! Stable identity helpers for app-template orders.
//!
//! Two hashes are derived from one command: the *request fingerprint* covers the
//! transport/payment selection and detects an idempotency key reused for a
//! different request, while the *purchase intent key* covers the commercial
//! intent (buyer + listing + price) and lets a retry reuse the order that is
//! still awaiting payment instead of creating a second one.

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{CreateAppTemplateOrderCommand, APP_TEMPLATE_ORDER_SUBJECT};
use sdkwork_utils_rust::sha256_hash;

pub(crate) fn app_template_order_request_fingerprint(
    command: &CreateAppTemplateOrderCommand,
) -> String {
    sha256_hash(
        serde_json::json!({
            "clientRequestNo": command.client_request_no,
            "paymentMethod": command.method,
            "paymentProduct": command.payment_product,
            "source": command.source,
            "templateUuid": command.template_uuid,
        })
        .to_string()
        .as_bytes(),
    )
}

pub(crate) fn app_template_purchase_intent_key(
    command: &CreateAppTemplateOrderCommand,
    price_amount: &str,
    currency_code: &str,
) -> String {
    sha256_hash(
        serde_json::json!({
            "currencyCode": currency_code,
            "organizationId": command.organization_id,
            "ownerUserId": command.owner_user_id,
            "priceAmount": price_amount,
            "subject": APP_TEMPLATE_ORDER_SUBJECT,
            "templateUuid": command.template_uuid,
            "tenantId": command.tenant_id,
        })
        .to_string()
        .as_bytes(),
    )
}

pub(crate) fn ensure_app_template_request_fingerprint_matches(
    persisted: &str,
    expected: &str,
) -> Result<(), CommerceServiceError> {
    if persisted == expected {
        return Ok(());
    }
    Err(CommerceServiceError::conflict(
        "app template order idempotency key was already used with a different request",
    ))
}

#[cfg(test)]
mod tests {
    use sdkwork_order_service::CreateAppTemplateOrderCommand;

    use super::{
        app_template_order_request_fingerprint, app_template_purchase_intent_key,
        ensure_app_template_request_fingerprint_matches,
    };

    fn command(method: &str, product: &str) -> CreateAppTemplateOrderCommand {
        CreateAppTemplateOrderCommand::new(
            "tenant-1",
            Some("0"),
            "user-1",
            "template-1",
            method,
            product,
            "order-1",
            "item-1",
            "AT1",
            "APPTEMPLATE1",
            "2026-07-26T00:00:00Z",
            "2026-07-26T00:30:00Z",
            "idem-1",
            None,
            None,
        )
        .expect("app template command")
    }

    #[test]
    fn payment_selection_changes_transport_fingerprint_not_purchase_intent() {
        let wechat = command("wechat_pay", "wechat_native");
        let alipay = command("alipay", "alipay_native");

        assert_ne!(
            app_template_order_request_fingerprint(&wechat),
            app_template_order_request_fingerprint(&alipay)
        );
        assert_eq!(
            app_template_purchase_intent_key(&wechat, "9900", "CNY"),
            app_template_purchase_intent_key(&alipay, "9900", "CNY")
        );
    }

    #[test]
    fn purchase_intent_separates_listings_and_prices() {
        let base = command("wechat_pay", "wechat_native");
        let mut other_template = base.clone();
        other_template.template_uuid = "template-2".to_owned();

        assert_ne!(
            app_template_purchase_intent_key(&base, "9900", "CNY"),
            app_template_purchase_intent_key(&other_template, "9900", "CNY")
        );
        assert_ne!(
            app_template_purchase_intent_key(&base, "9900", "CNY"),
            app_template_purchase_intent_key(&base, "19900", "CNY")
        );
    }

    #[test]
    fn fingerprint_mismatch_is_a_conflict() {
        assert!(ensure_app_template_request_fingerprint_matches("a", "a").is_ok());
        assert!(ensure_app_template_request_fingerprint_matches("a", "b").is_err());
    }
}
