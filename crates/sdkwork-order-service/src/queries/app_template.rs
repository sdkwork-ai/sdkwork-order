//! Buyer-facing queries over app-template orders (the entitlement inventory).
//!
//! The order center is the system of record for "who may install which
//! template": a settled `commerce_order` with subject
//! [`crate::APP_TEMPLATE_ORDER_SUBJECT`] is the entitlement, so this query reads
//! orders rather than a module-owned purchase table.

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppTemplateOrderListQuery {
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub page: i64,
    pub page_size: i64,
    pub tenant_id: String,
}

impl AppTemplateOrderListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;
        Ok(Self {
            organization_id: organization_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            owner_user_id: owner_user_id.trim().to_string(),
            page,
            page_size,
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

/// One app-template order as the buyer sees it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppTemplateOrderSummary {
    /// Settled amount; zero for a free listing.
    pub amount: CommerceMoney,
    pub created_at: String,
    pub currency_code: String,
    /// `fulfilled` once the order center has granted the template.
    pub fulfillment_status: String,
    pub order_id: String,
    pub order_no: String,
    pub paid_at: Option<String>,
    /// Wire status folded from the stored lifecycle (`paid` / `pending_payment`
    /// / `closed`).
    pub status: String,
    pub template_name: String,
    /// `deploy_app_template.uuid` — the listing this order entitles.
    pub template_uuid: String,
    /// Published version captured at order time, when the listing had one.
    pub version_uuid: Option<String>,
}

impl AppTemplateOrderSummary {
    /// `true` when the buyer may install the template this row entitles.
    pub fn is_entitled(&self) -> bool {
        self.status == crate::APP_TEMPLATE_ORDER_STATUS_PAID
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppTemplateOrderPage {
    pub items: Vec<AppTemplateOrderSummary>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

impl AppTemplateOrderPage {
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

#[cfg(test)]
mod tests {
    use super::AppTemplateOrderListQuery;

    #[test]
    fn list_query_rejects_a_missing_owner_scope() {
        assert!(AppTemplateOrderListQuery::new("tenant-1", Some("0"), "  ", None, None).is_err());
        assert!(AppTemplateOrderListQuery::new(" ", Some("0"), "user-1", None, None).is_err());
    }

    #[test]
    fn list_query_defaults_pagination() {
        let query = AppTemplateOrderListQuery::new("tenant-1", None, "user-1", None, None).unwrap();
        assert!(query.page >= 1);
        assert!(query.limit() > 0);
        assert_eq!(query.offset(), (query.page - 1) * query.page_size);
    }
}
