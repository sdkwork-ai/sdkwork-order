//! Command creating one app-template purchase order in the order center.
//!
//! The deployments module owns the template catalog (`deploy_app_template`),
//! this module owns the trade: the command carries only the template reference
//! plus the transport/payment selection, and the store resolves the listing's
//! title, price and currency at order time and snapshots them onto the order
//! item. Nothing about the purchase is stored back in the deployments module.

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_utils_rust::parse_datetime;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateAppTemplateOrderCommand {
    pub client_request_no: Option<String>,
    pub expire_at: String,
    pub idempotency_key: String,
    pub method: String,
    pub order_id: String,
    pub order_item_id: String,
    pub order_no: String,
    pub organization_id: Option<String>,
    pub out_trade_no: String,
    pub owner_user_id: String,
    pub payment_product: String,
    pub requested_at: String,
    pub source: Option<String>,
    pub tenant_id: String,
    /// `deploy_app_template.uuid` — the listing being purchased.
    pub template_uuid: String,
}

impl CreateAppTemplateOrderCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        template_uuid: &str,
        method: &str,
        payment_product: &str,
        order_id: &str,
        order_item_id: &str,
        order_no: &str,
        out_trade_no: &str,
        requested_at: &str,
        expire_at: &str,
        idempotency_key: &str,
        client_request_no: Option<&str>,
        source: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("template_uuid", template_uuid)?;
        crate::validation::require_non_empty("method", method)?;
        crate::validation::require_non_empty("payment_product", payment_product)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        crate::validation::require_non_empty("order_item_id", order_item_id)?;
        crate::validation::require_non_empty("order_no", order_no)?;
        crate::validation::require_non_empty("out_trade_no", out_trade_no)?;
        crate::validation::require_non_empty("requested_at", requested_at)?;
        crate::validation::require_non_empty("expire_at", expire_at)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        let requested_at_value = parse_datetime(requested_at.trim(), None).ok_or_else(|| {
            CommerceServiceError::validation("requested_at must be an RFC3339 timestamp")
        })?;
        let expire_at_value = parse_datetime(expire_at.trim(), None).ok_or_else(|| {
            CommerceServiceError::validation("expire_at must be an RFC3339 timestamp")
        })?;
        if expire_at_value <= requested_at_value {
            return Err(CommerceServiceError::validation(
                "expire_at must be later than requested_at",
            ));
        }

        Ok(Self {
            client_request_no: optional_text(client_request_no),
            expire_at: expire_at.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
            method: method.trim().to_ascii_lowercase(),
            order_id: order_id.trim().to_string(),
            order_item_id: order_item_id.trim().to_string(),
            order_no: order_no.trim().to_string(),
            organization_id: optional_text(organization_id),
            out_trade_no: out_trade_no.trim().to_string(),
            owner_user_id: owner_user_id.trim().to_string(),
            payment_product: payment_product.trim().to_ascii_lowercase(),
            requested_at: requested_at.trim().to_string(),
            source: optional_text(source),
            tenant_id: tenant_id.trim().to_string(),
            template_uuid: template_uuid.trim().to_string(),
        })
    }
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::CreateAppTemplateOrderCommand;

    fn command(
        requested_at: &str,
        expire_at: &str,
    ) -> Result<CreateAppTemplateOrderCommand, sdkwork_contract_service::CommerceServiceError> {
        CreateAppTemplateOrderCommand::new(
            "tenant-1",
            Some("0"),
            "user-1",
            "template-1",
            "wechat_pay",
            "wechat_native",
            "order-1",
            "item-1",
            "AT1",
            "APPTEMPLATE1",
            requested_at,
            expire_at,
            "idem-1",
            None,
            None,
        )
    }

    #[test]
    fn app_template_order_command_normalizes_its_selections() {
        let value = command("2026-07-26T08:00:00+08:00", "2026-07-26T00:30:00Z")
            .expect("valid app template order command");

        assert_eq!(value.method, "wechat_pay");
        assert_eq!(value.payment_product, "wechat_native");
        assert_eq!(value.template_uuid, "template-1");
        assert_eq!(value.expire_at, "2026-07-26T00:30:00Z");
    }

    #[test]
    fn app_template_order_command_rejects_invalid_or_non_increasing_windows() {
        assert!(command("not-a-time", "2026-07-26T00:30:00Z").is_err());
        assert!(command("2026-07-26T00:00:00Z", "invalid").is_err());
        assert!(command("2026-07-26T08:00:00+08:00", "2026-07-26T00:00:00Z").is_err());
    }

    #[test]
    fn app_template_order_command_requires_the_template_reference() {
        assert!(CreateAppTemplateOrderCommand::new(
            "tenant-1",
            Some("0"),
            "user-1",
            "   ",
            "wechat_pay",
            "wechat_native",
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
        .is_err());
    }
}
