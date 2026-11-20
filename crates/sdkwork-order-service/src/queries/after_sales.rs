use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterSalesRequestDetailQuery {
    pub after_sales_request_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub tenant_id: String,
}

/// 售后单列表查询（owner 域），支持标准 offset 分页。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterSalesRequestListQuery {
    pub after_sales_request_id: Option<String>,
    pub after_sales_type: Option<String>,
    pub organization_id: Option<String>,
    pub order_id: Option<String>,
    pub owner_user_id: String,
    pub status: Option<String>,
    pub tenant_id: String,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterSalesEventListQuery {
    pub after_sales_request_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub tenant_id: String,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateAfterSalesRequestCommand {
    pub after_sales_type: String,
    pub currency_code: Option<String>,
    pub description: Option<String>,
    pub idempotency_key: String,
    pub items: Vec<CreateAfterSalesRequestItemInput>,
    pub order_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub reason_code: String,
    pub request_no: String,
    pub requested_amount: Option<String>,
    pub tenant_id: String,
}

/// 售后单行项输入：用于部分退款 / 换货时指定具体订单行项。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateAfterSalesRequestItemInput {
    pub order_item_id: String,
    pub quantity: i64,
    pub requested_amount: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateAfterSalesReturnShipmentCommand {
    pub after_sales_request_id: String,
    pub carrier_code: Option<String>,
    pub idempotency_key: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub request_no: String,
    pub tenant_id: String,
    pub tracking_no: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateAfterSalesRequestCommand {
    pub after_sales_request_id: String,
    pub approved_amount: Option<String>,
    pub currency_code: Option<String>,
    pub description: Option<String>,
    pub idempotency_key: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub reason_code: Option<String>,
    pub request_no: String,
    pub requested_amount: Option<String>,
    pub reviewer_note: Option<String>,
    pub status: Option<String>,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterSalesRequestView {
    pub after_sales_no: String,
    pub after_sales_request_id: String,
    pub after_sales_type: String,
    pub owner_user_id: String,
    pub currency_code: String,
    pub order_id: String,
    pub reason_code: String,
    pub requested_amount: CommerceMoney,
    /// Approved refund amount in smallest units, when the reviewer supplied
    /// one. Stored money conventions apply (integer minor units; legacy
    /// decimal major-unit values are normalized on read).
    pub approved_amount: Option<CommerceMoney>,
    /// Refund lifecycle mirrored from the refund execution
    /// (`none` / `processing` / `refunded` / `failed`).
    pub refund_status: String,
    /// `commerce_order_refund_request.id` created for this case, if any.
    pub refund_request_id: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterSalesReturnShipmentView {
    pub after_sales_request_id: String,
    pub return_shipment_id: String,
    pub return_shipment_no: String,
    pub status: String,
    pub tracking_no: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterSalesEventView {
    pub after_sales_request_id: String,
    pub event_id: String,
    pub event_no: String,
    pub event_type: String,
    pub to_status: String,
}

/// 售后单分页结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterSalesRequestPage {
    pub items: Vec<AfterSalesRequestView>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterSalesReturnShipmentListQuery {
    pub after_sales_request_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub status: Option<String>,
    pub tenant_id: String,
    pub page: i64,
    pub page_size: i64,
}

/// 售后退货物流分页结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterSalesReturnShipmentPage {
    pub items: Vec<AfterSalesReturnShipmentView>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

/// 售后单事件分页结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterSalesEventPage {
    pub items: Vec<AfterSalesEventView>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

impl AfterSalesRequestPage {
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

impl AfterSalesEventPage {
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

impl AfterSalesRequestDetailQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        after_sales_request_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("after_sales_request_id", after_sales_request_id)?;

        Ok(Self {
            after_sales_request_id: after_sales_request_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

impl AfterSalesRequestListQuery {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: Option<&str>,
        after_sales_type: Option<&str>,
        status: Option<&str>,
        after_sales_request_id: Option<&str>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;

        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;

        Ok(Self {
            after_sales_request_id: optional_text(after_sales_request_id),
            after_sales_type: optional_text(after_sales_type),
            organization_id: optional_text(organization_id),
            order_id: optional_text(order_id),
            owner_user_id: owner_user_id.trim().to_string(),
            status: optional_text(status),
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

impl AfterSalesEventListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        after_sales_request_id: &str,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("after_sales_request_id", after_sales_request_id)?;

        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;

        Ok(Self {
            after_sales_request_id: after_sales_request_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
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

impl AfterSalesReturnShipmentListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        after_sales_request_id: &str,
        status: Option<&str>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("after_sales_request_id", after_sales_request_id)?;

        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;

        Ok(Self {
            after_sales_request_id: after_sales_request_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            status: optional_text(status),
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

impl CreateAfterSalesRequestCommand {
    /// 创建售后单命令。
    ///
    /// - `requested_amount` / `currency_code`：整单退款金额与币种。
    ///   - 当 `items` 为空（整单退款）时，若 `requested_amount` 为 `None` 则由仓储层
    ///     回退到订单应付总额；若提供则必须同时提供 `currency_code`。
    ///   - 当 `items` 非空（行项退款）时，`currency_code` 必须提供。
    /// - `items`：行项退款输入。为空表示整单退款。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
        reason_code: &str,
        after_sales_type: &str,
        description: Option<&str>,
        requested_amount: Option<&str>,
        currency_code: Option<&str>,
        items: Vec<CreateAfterSalesRequestItemInput>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        crate::validation::require_non_empty("reason_code", reason_code)?;
        crate::validation::require_non_empty("after_sales_type", after_sales_type)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        let requested_amount = optional_text(requested_amount);
        let currency_code = optional_text(currency_code);

        // 整单退款时若显式提供金额，必须同时提供币种。
        if requested_amount.is_some() && currency_code.is_none() {
            return Err(CommerceServiceError::validation(
                "currency_code must be provided when requested_amount is specified",
            ));
        }
        // 行项退款时必须提供币种（行项金额继承该币种）。
        if !items.is_empty() && currency_code.is_none() {
            return Err(CommerceServiceError::validation(
                "currency_code must be provided for line-item after-sales requests",
            ));
        }
        // 校验每个行项。
        for item in &items {
            crate::validation::require_non_empty("order_item_id", &item.order_item_id)?;
            if item.quantity <= 0 {
                return Err(CommerceServiceError::validation(
                    "item quantity must be greater than zero",
                ));
            }
        }

        Ok(Self {
            after_sales_type: after_sales_type.trim().to_ascii_lowercase(),
            currency_code,
            description: optional_text(description),
            idempotency_key: idempotency_key.trim().to_string(),
            items,
            order_id: order_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            reason_code: reason_code.trim().to_string(),
            request_no: request_no.trim().to_string(),
            requested_amount,
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

impl CreateAfterSalesRequestItemInput {
    pub fn new(
        order_item_id: &str,
        quantity: i64,
        requested_amount: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("order_item_id", order_item_id)?;
        if quantity <= 0 {
            return Err(CommerceServiceError::validation(
                "quantity must be greater than zero",
            ));
        }
        Ok(Self {
            order_item_id: order_item_id.trim().to_string(),
            quantity,
            requested_amount: optional_text(requested_amount),
        })
    }
}

impl CreateAfterSalesReturnShipmentCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        after_sales_request_id: &str,
        tracking_no: Option<&str>,
        carrier_code: Option<&str>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("after_sales_request_id", after_sales_request_id)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            after_sales_request_id: after_sales_request_id.trim().to_string(),
            carrier_code: optional_text(carrier_code),
            idempotency_key: idempotency_key.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            request_no: request_no.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
            tracking_no: optional_text(tracking_no),
        })
    }
}

impl UpdateAfterSalesRequestCommand {
    /// Buyer-scoped update; rejects operator-only review fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new_for_owner(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        after_sales_request_id: &str,
        status: Option<&str>,
        reason_code: Option<&str>,
        description: Option<&str>,
        requested_amount: Option<&str>,
        currency_code: Option<&str>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Self::new(
            tenant_id,
            organization_id,
            owner_user_id,
            after_sales_request_id,
            status,
            reason_code,
            description,
            requested_amount,
            None,
            currency_code,
            None,
            request_no,
            idempotency_key,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        after_sales_request_id: &str,
        status: Option<&str>,
        reason_code: Option<&str>,
        description: Option<&str>,
        requested_amount: Option<&str>,
        approved_amount: Option<&str>,
        currency_code: Option<&str>,
        reviewer_note: Option<&str>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("after_sales_request_id", after_sales_request_id)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        let command = Self {
            after_sales_request_id: after_sales_request_id.trim().to_string(),
            approved_amount: optional_text(approved_amount),
            currency_code: optional_text(currency_code),
            description: merge_optional_text(description, reviewer_note),
            idempotency_key: idempotency_key.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            reason_code: optional_text(reason_code),
            request_no: request_no.trim().to_string(),
            requested_amount: optional_text(requested_amount),
            reviewer_note: optional_text(reviewer_note),
            status: optional_text(status),
            tenant_id: tenant_id.trim().to_string(),
        };
        if !command.has_updates() {
            return Err(CommerceServiceError::validation(
                "at least one after-sales request field must be provided",
            ));
        }
        Ok(command)
    }

    pub fn has_updates(&self) -> bool {
        self.status.is_some()
            || self.reason_code.is_some()
            || self.description.is_some()
            || self.requested_amount.is_some()
            || self.approved_amount.is_some()
            || self.currency_code.is_some()
            || self.reviewer_note.is_some()
    }
}

/// 运营端售后单列表查询（租户/组织范围，不限定买家）。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterSalesManagementListQuery {
    pub after_sales_request_id: Option<String>,
    pub after_sales_type: Option<String>,
    pub order_id: Option<String>,
    pub organization_id: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub status: Option<String>,
    pub tenant_id: String,
}

/// 运营端售后单详情查询。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterSalesManagementDetailQuery {
    pub after_sales_request_id: String,
    pub organization_id: Option<String>,
    pub tenant_id: String,
}

/// 运营端售后审核命令。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewAfterSalesRequestCommand {
    pub after_sales_request_id: String,
    pub approved_amount: Option<String>,
    pub exchange_status: Option<String>,
    pub idempotency_key: String,
    pub organization_id: Option<String>,
    pub reason_code: Option<String>,
    pub refund_status: Option<String>,
    pub request_no: String,
    pub return_status: Option<String>,
    pub review_action: String,
    pub review_comment: Option<String>,
    pub status: Option<String>,
    pub tenant_id: String,
}

impl AfterSalesManagementListQuery {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: Option<&str>,
        after_sales_type: Option<&str>,
        status: Option<&str>,
        after_sales_request_id: Option<&str>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;
        Ok(Self {
            after_sales_request_id: optional_text(after_sales_request_id),
            after_sales_type: optional_text(after_sales_type),
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

impl AfterSalesManagementDetailQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        after_sales_request_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("after_sales_request_id", after_sales_request_id)?;
        Ok(Self {
            after_sales_request_id: after_sales_request_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

impl ReviewAfterSalesRequestCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        after_sales_request_id: &str,
        review_action: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("after_sales_request_id", after_sales_request_id)?;
        crate::validation::require_non_empty("review_action", review_action)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;
        Ok(Self {
            after_sales_request_id: after_sales_request_id.trim().to_string(),
            approved_amount: None,
            exchange_status: None,
            idempotency_key: idempotency_key.trim().to_string(),
            organization_id: optional_text(organization_id),
            reason_code: None,
            refund_status: None,
            request_no: request_no.trim().to_string(),
            return_status: None,
            review_action: review_action.trim().to_string(),
            review_comment: None,
            status: None,
            tenant_id: tenant_id.trim().to_string(),
        })
    }

    pub fn with_status(mut self, status: Option<String>) -> Self {
        self.status = status;
        self
    }

    pub fn with_refund_status(mut self, refund_status: Option<String>) -> Self {
        self.refund_status = refund_status;
        self
    }

    pub fn with_return_status(mut self, return_status: Option<String>) -> Self {
        self.return_status = return_status;
        self
    }

    pub fn with_exchange_status(mut self, exchange_status: Option<String>) -> Self {
        self.exchange_status = exchange_status;
        self
    }

    pub fn with_approved_amount(mut self, approved_amount: Option<String>) -> Self {
        self.approved_amount = approved_amount;
        self
    }

    pub fn with_reason_code(mut self, reason_code: Option<String>) -> Self {
        self.reason_code = reason_code;
        self
    }

    pub fn with_review_comment(mut self, review_comment: Option<String>) -> Self {
        self.review_comment = review_comment;
        self
    }

    pub fn resolved_status(&self) -> String {
        if let Some(status) = optional_text(self.status.as_deref()) {
            return status;
        }
        match self.review_action.trim().to_ascii_lowercase().as_str() {
            "approve" | "approved" => "approved".to_owned(),
            "reject" | "rejected" => "rejected".to_owned(),
            other => other.to_owned(),
        }
    }
}

fn merge_optional_text(primary: Option<&str>, secondary: Option<&str>) -> Option<String> {
    optional_text(primary).or_else(|| optional_text(secondary))
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_item(order_item_id: &str, quantity: i64) -> CreateAfterSalesRequestItemInput {
        CreateAfterSalesRequestItemInput::new(order_item_id, quantity, None).unwrap()
    }

    #[test]
    fn update_after_sales_request_for_owner_rejects_operator_fields_via_constructor() {
        let command = UpdateAfterSalesRequestCommand::new_for_owner(
            "tenant-1",
            None,
            "user-1",
            "asr-1",
            Some("cancelled"),
            None,
            None,
            None,
            None,
            "REQ-1",
            "IDEM-1",
        )
        .unwrap();
        assert!(command.approved_amount.is_none());
        assert!(command.reviewer_note.is_none());
    }

    #[test]
    fn after_sales_request_list_query_rejects_invalid_page_size() {
        assert!(AfterSalesRequestListQuery::new(
            "tenant-1",
            None,
            "user-1",
            None,
            None,
            None,
            None,
            Some(0),
            Some(500),
        )
        .is_err());
    }

    #[test]
    fn after_sales_event_list_query_pagination_is_valid() {
        let query =
            AfterSalesEventListQuery::new("tenant-1", None, "user-1", "asr-1", Some(3), Some(10))
                .unwrap();
        assert_eq!(query.page, 3);
        assert_eq!(query.page_size, 10);
        assert_eq!(query.limit(), 10);
        assert_eq!(query.offset(), 20);
    }

    #[test]
    fn create_after_sales_request_requires_currency_when_amount_provided() {
        let result = CreateAfterSalesRequestCommand::new(
            "tenant-1",
            None,
            "user-1",
            "order-1",
            "reason",
            "refund",
            None,
            Some("100.00"),
            None,
            Vec::new(),
            "REQ-1",
            "IDEM-1",
        );
        assert!(result.is_err());
    }

    #[test]
    fn create_after_sales_request_requires_currency_when_items_provided() {
        let result = CreateAfterSalesRequestCommand::new(
            "tenant-1",
            None,
            "user-1",
            "order-1",
            "reason",
            "refund",
            None,
            None,
            None,
            vec![build_item("item-1", 1)],
            "REQ-1",
            "IDEM-1",
        );
        assert!(result.is_err());
    }

    #[test]
    fn create_after_sales_request_accepts_full_refund_without_amount() {
        let command = CreateAfterSalesRequestCommand::new(
            "tenant-1",
            None,
            "user-1",
            "order-1",
            "reason",
            "refund",
            None,
            None,
            None,
            Vec::new(),
            "REQ-1",
            "IDEM-1",
        )
        .unwrap();
        assert!(command.requested_amount.is_none());
        assert!(command.currency_code.is_none());
        assert!(command.items.is_empty());
    }

    #[test]
    fn create_after_sales_request_item_rejects_zero_quantity() {
        let result = CreateAfterSalesRequestItemInput::new("item-1", 0, None);
        assert!(result.is_err());
    }
}
