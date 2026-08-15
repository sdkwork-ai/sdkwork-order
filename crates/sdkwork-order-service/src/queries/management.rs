use sdkwork_contract_service::CommerceServiceError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderManagementListQuery {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub status: Option<String>,
    pub q: Option<String>,
    /// Inclusive lower bound on the order creation timestamp.
    pub created_from: Option<String>,
    /// Inclusive upper bound on the order creation timestamp.
    pub created_to: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderManagementDetailQuery {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub order_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CancelManagementOrderCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub order_id: String,
    pub cancel_reason: Option<String>,
    pub cancel_type: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloseManagementOrderCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub order_id: String,
    pub close_reason: Option<String>,
    pub close_type: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderManagementEventListQuery {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub order_id: String,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderCancellationListQuery {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderManagementEventView {
    pub id: String,
    pub event_type: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub message: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderCancellationView {
    pub id: String,
    pub order_id: String,
    pub status: String,
    pub reason_code: String,
    pub reason_message: Option<String>,
    pub created_at: String,
}

/// 管理端订单分页结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderManagementListPage {
    pub items: Vec<crate::OrderOwnerSummary>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

/// 管理端订单事件分页结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderManagementEventPage {
    pub items: Vec<OrderManagementEventView>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

/// 订单取消单分页结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderCancellationPage {
    pub items: Vec<OrderCancellationView>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

impl OrderManagementListPage {
    pub fn has_more(&self) -> bool {
        self.page.saturating_mul(self.page_size) < self.total
    }

    pub fn total_pages(&self) -> i64 {
        if self.page_size <= 0 {
            return 0;
        }
        (self.total + self.page_size - 1) / self.page_size
    }

    pub fn empty_for(query: &OrderManagementListQuery) -> Self {
        Self {
            items: Vec::new(),
            page: query.page,
            page_size: query.page_size,
            total: 0,
        }
    }
}

impl OrderManagementEventPage {
    pub fn has_more(&self) -> bool {
        self.page.saturating_mul(self.page_size) < self.total
    }

    pub fn total_pages(&self) -> i64 {
        if self.page_size <= 0 {
            return 0;
        }
        (self.total + self.page_size - 1) / self.page_size
    }

    pub fn empty_for(query: &OrderManagementEventListQuery) -> Self {
        Self {
            items: Vec::new(),
            page: query.page,
            page_size: query.page_size,
            total: 0,
        }
    }
}

impl OrderCancellationPage {
    pub fn has_more(&self) -> bool {
        self.page.saturating_mul(self.page_size) < self.total
    }

    pub fn total_pages(&self) -> i64 {
        if self.page_size <= 0 {
            return 0;
        }
        (self.total + self.page_size - 1) / self.page_size
    }

    pub fn empty_for(query: &OrderCancellationListQuery) -> Self {
        Self {
            items: Vec::new(),
            page: query.page,
            page_size: query.page_size,
            total: 0,
        }
    }
}

impl OrderManagementListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        status: Option<&str>,
        q: Option<&str>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        Self::with_created_range(
            tenant_id,
            organization_id,
            status,
            q,
            None,
            None,
            page,
            page_size,
        )
    }

    pub fn with_created_range(
        tenant_id: &str,
        organization_id: Option<&str>,
        status: Option<&str>,
        q: Option<&str>,
        created_from: Option<&str>,
        created_to: Option<&str>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;
        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            status: optional_text(status),
            q: optional_text(q),
            created_from: optional_text(created_from),
            created_to: optional_text(created_to),
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

impl OrderManagementDetailQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            order_id: order_id.trim().to_string(),
        })
    }
}

impl CancelManagementOrderCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: &str,
        cancel_reason: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        Self::with_cancel_type(tenant_id, organization_id, order_id, cancel_reason, None)
    }

    pub fn with_cancel_type(
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: &str,
        cancel_reason: Option<&str>,
        cancel_type: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            order_id: order_id.trim().to_string(),
            cancel_reason: optional_text(cancel_reason),
            cancel_type: optional_text(cancel_type),
        })
    }
}

impl CloseManagementOrderCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: &str,
        close_reason: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        Self::with_close_type(tenant_id, organization_id, order_id, close_reason, None)
    }

    pub fn with_close_type(
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: &str,
        close_reason: Option<&str>,
        close_type: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            order_id: order_id.trim().to_string(),
            close_reason: optional_text(close_reason),
            close_type: optional_text(close_type),
        })
    }
}

impl OrderManagementEventListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: &str,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;
        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            order_id: order_id.trim().to_string(),
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

impl OrderCancellationListQuery {
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
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            status: optional_text(status),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_query_rejects_invalid_page_size() {
        assert!(
            OrderManagementListQuery::new("t1", None, None, None, Some(1), Some(1000)).is_err()
        );
        assert!(OrderManagementListQuery::new("t1", None, None, None, Some(1), Some(0)).is_err());
    }

    #[test]
    fn event_list_query_offset_matches_page() {
        let query =
            OrderManagementEventListQuery::new("t1", None, "o1", Some(3), Some(15)).unwrap();
        assert_eq!(query.limit(), 15);
        assert_eq!(query.offset(), 30);
    }

    #[test]
    fn cancellation_list_query_rejects_invalid_page_size() {
        assert!(OrderCancellationListQuery::new("t1", None, None, Some(1), Some(0)).is_err());
    }

    #[test]
    fn cancel_command_with_cancel_type_round_trips() {
        let cmd = CancelManagementOrderCommand::with_cancel_type(
            "t1",
            None,
            "o1",
            Some("user_request"),
            Some("user_initiated"),
        )
        .unwrap();
        assert_eq!(cmd.cancel_type.as_deref(), Some("user_initiated"));
    }

    #[test]
    fn close_command_with_close_type_round_trips() {
        let cmd =
            CloseManagementOrderCommand::with_close_type("t1", None, "o1", None, Some("system"))
                .unwrap();
        assert_eq!(cmd.close_type.as_deref(), Some("system"));
    }
}
