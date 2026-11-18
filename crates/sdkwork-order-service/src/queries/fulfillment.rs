use sdkwork_contract_service::CommerceServiceError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FulfillmentManagementListQuery {
    pub organization_id: Option<String>,
    pub order_id: Option<String>,
    pub status: Option<String>,
    pub tenant_id: String,
    pub page: i64,
    pub page_size: i64,
}

impl FulfillmentManagementListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: Option<&str>,
        status: Option<&str>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;
        Ok(Self {
            organization_id: optional_text(organization_id),
            order_id: optional_text(order_id),
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FulfillmentListQuery {
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: Option<String>,
    pub status: Option<String>,
    pub tenant_id: String,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FulfillmentDetailQuery {
    pub fulfillment_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FulfillmentView {
    pub fulfillment_id: String,
    pub fulfillment_no: String,
    pub fulfillment_type: String,
    pub order_id: String,
    pub status: String,
}

/// 履约分页结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FulfillmentListPage {
    pub items: Vec<FulfillmentView>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

impl FulfillmentListPage {
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

impl FulfillmentListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: Option<&str>,
        status: Option<&str>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;

        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;

        Ok(Self {
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            order_id: optional_text(order_id),
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

impl FulfillmentDetailQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        fulfillment_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("fulfillment_id", fulfillment_id)?;

        Ok(Self {
            fulfillment_id: fulfillment_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
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
    use super::FulfillmentManagementListQuery;

    #[test]
    fn management_query_defaults_to_the_first_page() {
        let query = FulfillmentManagementListQuery::new(
            "t-1",
            None,
            Some(" order-1 "),
            None,
            None,
            None,
        )
        .expect("valid query");
        assert_eq!(query.page, 1);
        assert!(query.page_size > 0);
        assert_eq!(query.offset(), 0);
        assert_eq!(query.order_id.as_deref(), Some("order-1"));
        assert_eq!(query.organization_id, None);
    }

    #[test]
    fn management_query_rejects_an_empty_tenant() {
        assert!(FulfillmentManagementListQuery::new("", None, None, None, None, None).is_err());
    }
}
