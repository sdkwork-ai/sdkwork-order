use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::MembershipPurchaseSettlementSnapshot;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderPaymentSettlementContext {
    pub membership_purchase: Option<MembershipPurchaseSettlementSnapshot>,
    pub owner_user_id: String,
    pub subject: String,
}

pub(crate) fn membership_purchase_snapshot(
    subject: &str,
    order_no: &str,
    membership_action: Option<&str>,
    sku_snapshot_json: Option<&str>,
) -> Result<Option<MembershipPurchaseSettlementSnapshot>, CommerceServiceError> {
    if !subject.eq_ignore_ascii_case("membership") {
        return Ok(None);
    }

    let order_no = required_snapshot_text(order_no, "membership order number")?;
    let snapshot = sku_snapshot_json
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CommerceServiceError::invalid_state("membership order item snapshot is unavailable")
        })?;
    let value: serde_json::Value = serde_json::from_str(snapshot).map_err(|_| {
        CommerceServiceError::invalid_state("membership order item snapshot is invalid")
    })?;
    let action = membership_action
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| value.get("action").and_then(serde_json::Value::as_str))
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(
        action.as_str(),
        "purchase" | "renew" | "upgrade" | "recharge"
    ) {
        return Err(CommerceServiceError::invalid_state(
            "membership order item snapshot action is invalid",
        ));
    }
    if action == "recharge" {
        // 订阅期额度充值：不依赖目录套餐，数量必填
        let grant_quantity = value
            .get("grantQuantity")
            .and_then(json_positive_i64)
            .ok_or_else(|| {
                CommerceServiceError::invalid_state(
                    "membership order item snapshot grant quantity is invalid",
                )
            })?;
        return Ok(Some(MembershipPurchaseSettlementSnapshot {
            action,
            order_no: order_no.to_owned(),
            package_id: 0,
            grant_quantity: Some(grant_quantity),
        }));
    }
    let package_id = value
        .get("packageId")
        .and_then(json_positive_i64)
        .ok_or_else(|| {
            CommerceServiceError::invalid_state(
                "membership order item snapshot package id is invalid",
            )
        })?;

    Ok(Some(MembershipPurchaseSettlementSnapshot {
        action,
        order_no: order_no.to_owned(),
        package_id,
        grant_quantity: None,
    }))
}

fn json_positive_i64(value: &serde_json::Value) -> Option<i64> {
    let parsed = value
        .as_i64()
        .or_else(|| value.as_str()?.trim().parse::<i64>().ok())?;
    (parsed > 0).then_some(parsed)
}

fn required_snapshot_text<'a>(value: &'a str, name: &str) -> Result<&'a str, CommerceServiceError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(CommerceServiceError::invalid_state(format!(
            "{name} is unavailable"
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::membership_purchase_snapshot;

    #[test]
    fn membership_snapshot_uses_order_authority_and_string_package_id() {
        let snapshot = membership_purchase_snapshot(
            "membership",
            "MB-1",
            Some(" UPGRADE "),
            Some(r#"{"packageId":"21","action":"purchase"}"#),
        )
        .expect("valid snapshot")
        .expect("membership snapshot");

        assert_eq!(snapshot.action, "upgrade");
        assert_eq!(snapshot.order_no, "MB-1");
        assert_eq!(snapshot.package_id, 21);
        assert_eq!(snapshot.grant_quantity, None);
    }

    #[test]
    fn membership_snapshot_accepts_recharge_with_grant_quantity() {
        let snapshot = membership_purchase_snapshot(
            "membership",
            "MB-2",
            Some("recharge"),
            Some(r#"{"action":"recharge","grantQuantity":1000}"#),
        )
        .expect("valid snapshot")
        .expect("recharge snapshot");

        assert_eq!(snapshot.action, "recharge");
        assert_eq!(snapshot.grant_quantity, Some(1000));
    }

    #[test]
    fn membership_snapshot_rejects_recharge_without_grant_quantity() {
        assert!(membership_purchase_snapshot(
            "membership",
            "MB-3",
            Some("recharge"),
            Some(r#"{"action":"recharge"}"#),
        )
        .is_err());
    }

    #[test]
    fn membership_snapshot_rejects_missing_or_invalid_business_fields() {
        assert!(membership_purchase_snapshot("membership", "MB-1", None, None).is_err());
        assert!(membership_purchase_snapshot(
            "membership",
            "MB-1",
            Some("downgrade"),
            Some(r#"{"packageId":21}"#),
        )
        .is_err());
        assert!(membership_purchase_snapshot(
            "membership",
            "MB-1",
            Some("purchase"),
            Some(r#"{"packageId":0}"#),
        )
        .is_err());
    }
}
