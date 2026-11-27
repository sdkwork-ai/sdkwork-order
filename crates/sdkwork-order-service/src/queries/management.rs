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
    /// Commerce scene facet; `None` disables the predicate.
    pub scene: Option<crate::OrderScene>,
    /// Tag facet values; empty disables the predicate.
    pub tags: crate::OrderTagSet,
    /// How [`Self::tags`] combine.
    pub tag_match: crate::OrderTagMatchMode,
    pub page: i64,
    pub page_size: i64,
}

/// One tag bucket in the order-centre facet response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderTagFacetView {
    /// Canonical tag value.
    pub tag: String,
    /// Number of orders in the current scope carrying this tag.
    pub count: i64,
}

/// One scene bucket in the order-centre facet response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderSceneFacetView {
    /// Canonical scene value.
    pub scene: crate::OrderScene,
    /// Number of orders in the current scope with this scene.
    pub count: i64,
}

/// Aggregated facet counts for the order centre filter bar.
///
/// Counts are computed against the same tenant/org scope as the list but
/// **without** the tag/scene predicates applied, so the filter bar can show
/// "what you would get if you additionally selected this value" — the standard
/// faceted-search behaviour. They do respect `status` / `q` / `created_*` so the
/// numbers stay consistent with the visible result set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderManagementFacetView {
    pub scenes: Vec<OrderSceneFacetView>,
    pub tags: Vec<OrderTagFacetView>,
    /// Total orders in scope, before any tag/scene narrowing.
    pub total_orders: i64,
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

/// Adds tags to an order without touching the tags it already carries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddOrderTagsCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub order_id: String,
    pub tags: Vec<crate::OrderTag>,
    pub remark: Option<String>,
}

/// Replaces the whole tag set of an order (and optionally its scene).
///
/// `scene` is `None` when the caller only wants to rewrite tags; the scene is
/// otherwise switched. This is the single write path an operator UI uses when
/// it submits a token editor, so it is naturally idempotent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplaceOrderClassificationCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub order_id: String,
    pub tags: Vec<crate::OrderTag>,
    pub scene: Option<crate::OrderScene>,
    pub remark: Option<String>,
}

/// The classification facts of one order, returned after a write.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderClassificationView {
    pub order_id: String,
    pub scene: crate::OrderScene,
    pub tags: Vec<String>,
    pub remark: Option<String>,
    pub updated_at: String,
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

    // The argument list mirrors the documented management list facets
    // (tenant/org scope, filters, SdkWorkListQuery paging) one to one.
    #[allow(clippy::too_many_arguments)]
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
            scene: None,
            tags: crate::OrderTagSet::default(),
            tag_match: crate::OrderTagMatchMode::default(),
            page,
            page_size,
        })
    }

    /// Attaches the scene and tag facets.
    ///
    /// Kept as a chainable setter so the existing eight-argument constructor
    /// stays source-stable for callers that predate the facets.
    #[must_use]
    pub fn with_facets(
        mut self,
        scene: Option<crate::OrderScene>,
        tags: crate::OrderTagSet,
        tag_match: crate::OrderTagMatchMode,
    ) -> Self {
        self.scene = scene;
        self.tags = tags;
        self.tag_match = tag_match;
        self
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

/// Shared validation for the tag/classification write commands.
fn classify_write_scope(
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
) -> Result<(String, Option<String>, String), CommerceServiceError> {
    crate::validation::require_non_empty("tenant_id", tenant_id)?;
    crate::validation::require_non_empty("order_id", order_id)?;
    Ok((
        tenant_id.trim().to_string(),
        optional_text(organization_id),
        order_id.trim().to_string(),
    ))
}

/// Rejects an empty tag list: a write that changes nothing is a caller bug,
/// not a silent success, and would otherwise record a misleading audit event.
fn require_tags(tags: Vec<crate::OrderTag>) -> Result<Vec<crate::OrderTag>, CommerceServiceError> {
    if tags.is_empty() {
        return Err(CommerceServiceError::validation("tags must not be empty"));
    }
    if tags.len() > crate::ORDER_TAG_MAX_COUNT {
        return Err(CommerceServiceError::validation(format!(
            "an order accepts at most {} tags",
            crate::ORDER_TAG_MAX_COUNT
        )));
    }
    Ok(tags)
}

impl AddOrderTagsCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: &str,
        tags: Vec<crate::OrderTag>,
        remark: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        let (tenant_id, organization_id, order_id) =
            classify_write_scope(tenant_id, organization_id, order_id)?;
        Ok(Self {
            tenant_id,
            organization_id,
            order_id,
            tags: require_tags(tags)?,
            remark: crate::normalize_order_remark(remark)?,
        })
    }
}

impl ReplaceOrderClassificationCommand {
    /// Builds the replace command.
    ///
    /// An empty `tags` list is legal here (it clears the classification), which
    /// is the difference between this command and add/remove.
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: &str,
        tags: Vec<crate::OrderTag>,
        scene: Option<crate::OrderScene>,
        remark: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        let (tenant_id, organization_id, order_id) =
            classify_write_scope(tenant_id, organization_id, order_id)?;
        if tags.len() > crate::ORDER_TAG_MAX_COUNT {
            return Err(CommerceServiceError::validation(format!(
                "an order accepts at most {} tags",
                crate::ORDER_TAG_MAX_COUNT
            )));
        }
        Ok(Self {
            tenant_id,
            organization_id,
            order_id,
            tags,
            scene,
            remark: crate::normalize_order_remark(remark)?,
        })
    }
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
    fn list_query_defaults_disable_facets() {
        let query =
            OrderManagementListQuery::new("t1", None, Some("paid"), None, Some(1), Some(20))
                .unwrap();
        assert!(query.scene.is_none());
        assert!(query.tags.is_empty());
        assert_eq!(query.tag_match, crate::OrderTagMatchMode::Any);
    }

    #[test]
    fn list_query_carries_facets_through_the_builder() {
        let tags = crate::OrderTagSet::parse_many(["VIP", "risk"], "tags", 4).unwrap();
        let query =
            OrderManagementListQuery::new("t1", None, None, None, Some(1), Some(20))
                .unwrap()
                .with_facets(
                    Some(crate::OrderScene::H5),
                    tags,
                    crate::OrderTagMatchMode::All,
                );
        assert_eq!(query.scene, Some(crate::OrderScene::H5));
        assert_eq!(query.tags.to_values(), vec!["vip", "risk"]);
        assert_eq!(query.tag_match, crate::OrderTagMatchMode::All);
    }

    #[test]
    fn add_tags_command_rejects_empty_tag_list() {
        assert!(
            AddOrderTagsCommand::new("t1", None, "o1", Vec::new(), None).is_err(),
            "a no-op classification write must not silently succeed"
        );
        let tag = crate::OrderTag::normalize("vip").unwrap().unwrap();
        assert!(AddOrderTagsCommand::new("t1", None, "o1", vec![tag], Some("memo")).is_ok());
    }

    #[test]
    fn add_tags_command_trims_scope_and_remark() {
        let tag = crate::OrderTag::normalize("vip").unwrap().unwrap();
        let command =
            AddOrderTagsCommand::new(" t1 ", None, "  o1 ", vec![tag], Some("  note  ")).unwrap();
        assert_eq!(command.tenant_id, "t1");
        assert_eq!(command.order_id, "o1");
        assert_eq!(command.remark.as_deref(), Some("note"));
    }

    #[test]
    fn replace_classification_allows_clearing_tags() {
        let command = ReplaceOrderClassificationCommand::new(
            "t1",
            None,
            "o1",
            Vec::new(),
            Some(crate::OrderScene::Membership),
            None,
        )
        .unwrap();
        assert!(command.tags.is_empty());
        assert_eq!(command.scene, Some(crate::OrderScene::Membership));
    }

    #[test]
    fn tag_commands_reject_an_over_long_tag_set() {
        let tags = (0..=crate::ORDER_TAG_MAX_COUNT)
            .map(|index| crate::OrderTag::normalize(&format!("tag{index}")).unwrap().unwrap())
            .collect::<Vec<_>>();
        assert!(AddOrderTagsCommand::new("t1", None, "o1", tags.clone(), None).is_err());
        assert!(
            ReplaceOrderClassificationCommand::new("t1", None, "o1", tags, None, None).is_err()
        );
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
