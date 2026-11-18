use sdkwork_contract_service::CommerceServiceError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentDetailQuery {
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub shipment_id: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentPackageListQuery {
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub shipment_id: String,
    pub tenant_id: String,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentTrackingEventListQuery {
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub shipment_id: String,
    pub tenant_id: String,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentView {
    pub carrier_code: String,
    pub fulfillment_id: String,
    pub shipment_id: String,
    pub shipment_no: String,
    pub status: String,
    pub tracking_no: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentPackageView {
    pub package_id: String,
    pub package_no: String,
    pub package_type: String,
    pub shipment_id: String,
    pub status: String,
    pub tracking_no: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentTrackingEventView {
    pub event_id: String,
    pub event_status: Option<String>,
    pub event_time: String,
    pub event_type: String,
    pub location_text: Option<String>,
    pub shipment_id: String,
    pub tracking_event_no: String,
}

/// 物流包裹分页结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentPackagePage {
    pub items: Vec<ShipmentPackageView>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

/// 物流轨迹事件分页结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentTrackingEventPage {
    pub items: Vec<ShipmentTrackingEventView>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

impl ShipmentPackagePage {
    pub fn has_more(&self) -> bool {
        self.page.saturating_mul(self.page_size) < self.total
    }

    pub fn total_pages(&self) -> i64 {
        if self.page_size <= 0 {
            return 0;
        }
        (self.total + self.page_size - 1) / self.page_size
    }
}

impl ShipmentTrackingEventPage {
    pub fn has_more(&self) -> bool {
        self.page.saturating_mul(self.page_size) < self.total
    }

    pub fn total_pages(&self) -> i64 {
        if self.page_size <= 0 {
            return 0;
        }
        (self.total + self.page_size - 1) / self.page_size
    }
}

impl ShipmentDetailQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        shipment_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("shipment_id", shipment_id)?;

        Ok(Self {
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            shipment_id: shipment_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

impl ShipmentPackageListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        shipment_id: &str,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("shipment_id", shipment_id)?;
        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;
        Ok(Self {
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            shipment_id: shipment_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
            page,
            page_size,
        })
    }

    pub fn limit(&self) -> i64 {
        self.page_size
    }

    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.page_size
    }
}

impl ShipmentTrackingEventListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        shipment_id: &str,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("shipment_id", shipment_id)?;
        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;
        Ok(Self {
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            shipment_id: shipment_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
            page,
            page_size,
        })
    }

    pub fn limit(&self) -> i64 {
        self.page_size
    }

    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.page_size
    }
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// 运营端物流列表查询。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentManagementListQuery {
    pub fulfillment_id: Option<String>,
    pub order_id: Option<String>,
    pub organization_id: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub status: Option<String>,
    pub tenant_id: String,
}

/// 运营端物流详情查询。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentManagementDetailQuery {
    pub organization_id: Option<String>,
    pub shipment_id: String,
    pub tenant_id: String,
}

/// 运营端物流包裹列表查询。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentPackageManagementListQuery {
    pub organization_id: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub shipment_id: String,
    pub tenant_id: String,
}

/// 运营端创建物流包裹命令。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateShipmentPackageCommand {
    pub idempotency_key: String,
    pub organization_id: Option<String>,
    pub package_no: Option<String>,
    pub package_type: String,
    pub request_no: String,
    pub shipment_id: String,
    pub status: Option<String>,
    pub tenant_id: String,
    pub tracking_no: Option<String>,
}

/// 运营端更新物流包裹命令。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateShipmentPackageCommand {
    pub idempotency_key: String,
    pub organization_id: Option<String>,
    pub package_id: String,
    pub package_type: Option<String>,
    pub request_no: String,
    pub shipment_id: String,
    pub status: Option<String>,
    pub tenant_id: String,
    pub tracking_no: Option<String>,
}

/// 运营端物流分页结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentManagementListPage {
    pub items: Vec<ShipmentView>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

impl ShipmentManagementListPage {
    pub fn has_more(&self) -> bool {
        self.page.saturating_mul(self.page_size) < self.total
    }

    pub fn total_pages(&self) -> i64 {
        if self.page_size <= 0 {
            return 0;
        }
        (self.total + self.page_size - 1) / self.page_size
    }
}

impl ShipmentManagementListQuery {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: Option<&str>,
        fulfillment_id: Option<&str>,
        status: Option<&str>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;
        Ok(Self {
            fulfillment_id: optional_text(fulfillment_id),
            order_id: optional_text(order_id),
            organization_id: optional_text(organization_id),
            page,
            page_size,
            status: optional_text(status),
            tenant_id: tenant_id.trim().to_string(),
        })
    }

    pub fn limit(&self) -> i64 {
        self.page_size
    }

    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.page_size
    }
}

impl ShipmentManagementDetailQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        shipment_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("shipment_id", shipment_id)?;
        Ok(Self {
            organization_id: optional_text(organization_id),
            shipment_id: shipment_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

impl ShipmentPackageManagementListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        shipment_id: &str,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("shipment_id", shipment_id)?;
        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;
        Ok(Self {
            organization_id: optional_text(organization_id),
            page,
            page_size,
            shipment_id: shipment_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }

    pub fn limit(&self) -> i64 {
        self.page_size
    }

    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.page_size
    }
}

impl CreateShipmentPackageCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        shipment_id: &str,
        package_type: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("shipment_id", shipment_id)?;
        crate::validation::require_non_empty("package_type", package_type)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;
        Ok(Self {
            idempotency_key: idempotency_key.trim().to_string(),
            organization_id: optional_text(organization_id),
            package_no: None,
            package_type: package_type.trim().to_string(),
            request_no: request_no.trim().to_string(),
            shipment_id: shipment_id.trim().to_string(),
            status: None,
            tenant_id: tenant_id.trim().to_string(),
            tracking_no: None,
        })
    }
}

impl UpdateShipmentPackageCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        shipment_id: &str,
        package_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("shipment_id", shipment_id)?;
        crate::validation::require_non_empty("package_id", package_id)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;
        Ok(Self {
            idempotency_key: idempotency_key.trim().to_string(),
            organization_id: optional_text(organization_id),
            package_id: package_id.trim().to_string(),
            package_type: None,
            request_no: request_no.trim().to_string(),
            shipment_id: shipment_id.trim().to_string(),
            status: None,
            tenant_id: tenant_id.trim().to_string(),
            tracking_no: None,
        })
    }
}

/// 运营端物流轨迹列表查询。
///
/// The owner-facing `ShipmentTrackingEventListQuery` carries `owner_user_id`
/// because a buyer may only read their own shipment; an operator scope is
/// tenant + organization only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShipmentTrackingEventManagementListQuery {
    pub organization_id: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub shipment_id: String,
    pub tenant_id: String,
}

impl ShipmentTrackingEventManagementListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        shipment_id: &str,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("shipment_id", shipment_id)?;
        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;
        Ok(Self {
            organization_id: optional_text(organization_id),
            page,
            page_size,
            shipment_id: shipment_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }

    pub fn limit(&self) -> i64 {
        self.page_size
    }

    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.page_size
    }
}

/// 运营端追加物流轨迹事件命令。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateShipmentTrackingEventCommand {    pub event_status: Option<String>,
    pub event_time: Option<String>,
    pub event_type: String,
    pub idempotency_key: String,
    pub location_text: Option<String>,
    pub organization_id: Option<String>,
    pub request_no: String,
    pub shipment_id: String,
    pub tenant_id: String,
}

impl CreateShipmentTrackingEventCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        shipment_id: &str,
        event_type: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("shipment_id", shipment_id)?;
        crate::validation::require_non_empty("event_type", event_type)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;
        Ok(Self {
            event_status: None,
            event_time: None,
            event_type: event_type.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
            location_text: None,
            organization_id: optional_text(organization_id),
            request_no: request_no.trim().to_string(),
            shipment_id: shipment_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

/// 运营端承运商列表查询。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CarrierListQuery {
    pub organization_id: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub status: Option<String>,
    pub tenant_id: String,
}

impl CarrierListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        status: Option<&str>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;
        Ok(Self {
            organization_id: optional_text(organization_id),
            page,
            page_size,
            status: optional_text(status),
            tenant_id: tenant_id.trim().to_string(),
        })
    }

    pub fn limit(&self) -> i64 {
        self.page_size
    }

    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.page_size
    }
}

/// 承运商档案。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CarrierView {
    pub carrier_code: String,
    pub carrier_id: String,
    pub carrier_name: String,
    pub sort_order: i64,
    pub status: String,
    pub tracking_url_template: Option<String>,
}

/// 运营端承运商分页结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CarrierPage {
    pub items: Vec<CarrierView>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

impl CarrierPage {
    pub fn has_more(&self) -> bool {
        self.page.saturating_mul(self.page_size) < self.total
    }

    pub fn total_pages(&self) -> i64 {
        if self.page_size <= 0 {
            return 0;
        }
        (self.total + self.page_size - 1) / self.page_size
    }
}

/// 创建承运商命令。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateCarrierCommand {
    pub carrier_code: String,
    pub carrier_name: String,
    pub idempotency_key: String,
    pub organization_id: Option<String>,
    pub request_no: String,
    pub sort_order: i64,
    pub status: String,
    pub tenant_id: String,
    pub tracking_url_template: Option<String>,
}

impl CreateCarrierCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        carrier_code: &str,
        carrier_name: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("carrier_code", carrier_code)?;
        crate::validation::require_non_empty("carrier_name", carrier_name)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;
        Ok(Self {
            carrier_code: carrier_code.trim().to_lowercase(),
            carrier_name: carrier_name.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
            organization_id: optional_text(organization_id),
            request_no: request_no.trim().to_string(),
            sort_order: 0,
            status: "active".to_string(),
            tenant_id: tenant_id.trim().to_string(),
            tracking_url_template: None,
        })
    }
}

/// 更新承运商命令。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateCarrierCommand {
    pub carrier_id: String,
    pub carrier_name: Option<String>,
    pub idempotency_key: String,
    pub organization_id: Option<String>,
    pub request_no: String,
    pub sort_order: Option<i64>,
    pub status: Option<String>,
    pub tenant_id: String,
    pub tracking_url_template: Option<String>,
}

impl UpdateCarrierCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        carrier_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("carrier_id", carrier_id)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;
        Ok(Self {
            carrier_id: carrier_id.trim().to_string(),
            carrier_name: None,
            idempotency_key: idempotency_key.trim().to_string(),
            organization_id: optional_text(organization_id),
            request_no: request_no.trim().to_string(),
            sort_order: None,
            status: None,
            tenant_id: tenant_id.trim().to_string(),
            tracking_url_template: None,
        })
    }
}

/// 运营端将发货单标记为已发货。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkShipmentShippedCommand {
    pub organization_id: Option<String>,
    pub shipment_id: String,
    pub tenant_id: String,
}

impl MarkShipmentShippedCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        shipment_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("shipment_id", shipment_id)?;
        Ok(Self {
            organization_id: optional_text(organization_id),
            shipment_id: shipment_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

/// 运营端创建发货单命令。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateShipmentCommand {
    pub carrier_code: String,
    pub fulfillment_id: String,
    pub idempotency_key: String,
    pub organization_id: Option<String>,
    pub request_no: String,
    pub tenant_id: String,
    pub tracking_no: Option<String>,
}

impl CreateShipmentCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        fulfillment_id: &str,
        carrier_code: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("fulfillment_id", fulfillment_id)?;
        crate::validation::require_non_empty("carrier_code", carrier_code)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;
        Ok(Self {
            carrier_code: carrier_code.trim().to_lowercase(),
            fulfillment_id: fulfillment_id.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
            organization_id: optional_text(organization_id),
            request_no: request_no.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
            tracking_no: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::CreateShipmentCommand;

    #[test]
    fn create_shipment_command_normalizes_the_carrier_code() {
        let command =
            CreateShipmentCommand::new("t-1", Some("org-1"), "f-1", " SF ", "req-1", "key-1")
                .expect("valid command");
        assert_eq!(command.carrier_code, "sf");
        assert_eq!(command.organization_id.as_deref(), Some("org-1"));
        assert_eq!(command.tracking_no, None);
    }

    #[test]
    fn create_shipment_command_rejects_missing_keys() {
        assert!(CreateShipmentCommand::new("", None, "f-1", "sf", "req-1", "key-1").is_err());
        assert!(CreateShipmentCommand::new("t-1", None, "", "sf", "req-1", "key-1").is_err());
        assert!(CreateShipmentCommand::new("t-1", None, "f-1", "", "req-1", "key-1").is_err());
        assert!(CreateShipmentCommand::new("t-1", None, "f-1", "sf", "", "key-1").is_err());
        assert!(CreateShipmentCommand::new("t-1", None, "f-1", "sf", "req-1", "").is_err());
    }
}

#[cfg(test)]
mod mark_shipped_tests {
    use super::MarkShipmentShippedCommand;

    #[test]
    fn mark_shipped_command_requires_tenant_and_shipment() {
        assert!(MarkShipmentShippedCommand::new("", None, "sh-1").is_err());
        assert!(MarkShipmentShippedCommand::new("t-1", None, "").is_err());
    }

    #[test]
    fn mark_shipped_command_trims_ids() {
        let command =
            MarkShipmentShippedCommand::new(" t-1 ", Some(" org-1 "), " sh-1 ").expect("valid");
        assert_eq!(command.tenant_id, "t-1");
        assert_eq!(command.shipment_id, "sh-1");
        assert_eq!(command.organization_id.as_deref(), Some("org-1"));
    }
}
