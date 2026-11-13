use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::CreateMembershipOrderCommand;
use sdkwork_utils_rust::sha256_hash;

pub(crate) fn membership_order_request_fingerprint(
    command: &CreateMembershipOrderCommand,
) -> String {
    sha256_hash(
        serde_json::json!({
            "action": command.action,
            "clientRequestNo": command.client_request_no,
            "packageId": command.package_id,
            "paymentMethod": command.method,
            "paymentProduct": command.payment_product,
            "source": command.source,
        })
        .to_string()
        .as_bytes(),
    )
}

/// Business purchase-intent identity for a membership order.
///
/// The price dimension is the order total (unit price × seat quantity), so two
/// requests for the same package with different seat counts never reuse each
/// other's pending order. For a single-seat order the total equals the unit
/// price, which keeps the historical identity stable.
pub(crate) fn membership_purchase_intent_key(
    command: &CreateMembershipOrderCommand,
    sku_id: &str,
    price_amount: &str,
    currency_code: &str,
    duration_days: i64,
) -> String {
    sha256_hash(
        serde_json::json!({
            "action": command.action,
            "currencyCode": currency_code,
            "durationDays": duration_days,
            "organizationId": command.organization_id,
            "ownerUserId": command.owner_user_id,
            "packageId": command.package_id,
            "priceAmount": price_amount,
            "skuId": sku_id,
            "subject": "membership",
            "tenantId": command.tenant_id,
        })
        .to_string()
        .as_bytes(),
    )
}

pub(crate) fn ensure_membership_request_fingerprint_matches(
    persisted: &str,
    expected: &str,
) -> Result<(), CommerceServiceError> {
    if persisted == expected {
        return Ok(());
    }
    Err(CommerceServiceError::conflict(
        "membership order idempotency key was already used with a different request",
    ))
}

#[cfg(test)]
mod tests {
    use sdkwork_order_service::CreateMembershipOrderCommand;

    use super::{membership_order_request_fingerprint, membership_purchase_intent_key};

    fn command(method: &str, product: &str) -> CreateMembershipOrderCommand {
        CreateMembershipOrderCommand::new(
            "tenant-1",
            Some("0"),
            "user-1",
            "201",
            "purchase",
            method,
            product,
            "order-1",
            "item-1",
            "MB1",
            "MEMBERSHIP1",
            "2026-07-26T00:00:00Z",
            "2026-07-26T00:30:00Z",
            "idem-1",
            None,
            None,
            None,
            None,
            1,
        )
        .expect("membership command")
    }

    #[test]
    fn payment_selection_changes_transport_fingerprint_not_purchase_intent() {
        let wechat = command("wechat_pay", "wechat_native");
        let alipay = command("alipay", "alipay_native");

        assert_ne!(
            membership_order_request_fingerprint(&wechat),
            membership_order_request_fingerprint(&alipay)
        );
        assert_eq!(
            membership_purchase_intent_key(&wechat, "sku-1", "6800", "CNY", 30),
            membership_purchase_intent_key(&alipay, "sku-1", "6800", "CNY", 30)
        );
    }
}
