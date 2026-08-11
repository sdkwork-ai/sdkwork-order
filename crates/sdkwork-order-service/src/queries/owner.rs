use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

pub use sdkwork_payment_service::{
    PayOwnerOrderCommand, PayOwnerOrderCommandInput, PayOwnerOrderOutcome,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderOwnerListQuery {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub status: Option<String>,
    pub subject: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

/// 订单事件列表查询（owner 域）。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderOwnerEventListQuery {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
    pub page: i64,
    pub page_size: i64,
}

impl OrderOwnerEventListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;
        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
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

/// 订单事件视图（owner 域）。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderOwnerEventView {
    pub event_id: String,
    pub order_id: String,
    pub event_type: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub actor_type: Option<String>,
    pub actor_id: Option<String>,
    pub message: Option<String>,
    pub created_at: String,
}

/// 订单事件分页结果（owner 域）。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderOwnerEventPage {
    pub items: Vec<OrderOwnerEventView>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

impl OrderOwnerEventPage {
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

/// Paginated result of [`OrderOwnerListQuery`].
///
/// `total` is the unconditional count of rows matching the filter (independent
/// of the current page) so the API surface can render `hasMore` and total
/// page metadata. The repository computes both in a single SQL round-trip
/// using a `COUNT(*) OVER()` window function to avoid the N+1 / double-query
/// pattern used by older pagination shims.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderOwnerListPage {
    pub items: Vec<OrderOwnerSummary>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

impl OrderOwnerListPage {
    /// `true` when another page of results exists beyond the current one.
    pub fn has_more(&self) -> bool {
        self.page.saturating_mul(self.page_size) < self.total
    }

    /// 1-based total page count derived from `total` and `page_size`.
    pub fn total_pages(&self) -> i64 {
        if self.page_size <= 0 {
            return 0;
        }
        (self.total + self.page_size - 1) / self.page_size
    }

    /// Build an empty page for the given query — used when the underlying
    /// read model is missing or filtered to nothing.
    pub fn empty_for(query: &OrderOwnerListQuery) -> Self {
        Self {
            items: Vec::new(),
            page: query.page,
            page_size: query.page_size,
            total: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderOwnerDetailQuery {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderOwnerSummary {
    pub order_id: String,
    pub order_sn: String,
    pub status: String,
    pub subject: String,
    pub total_amount: CommerceMoney,
    pub paid_amount: Option<CommerceMoney>,
    pub discount_amount: Option<CommerceMoney>,
    pub currency_code: String,
    pub quantity: i64,
    pub created_at: String,
    pub pay_time: Option<String>,
    pub expire_time: Option<String>,
    pub payment_method: Option<String>,
    /// Points credited for `points_recharge` orders when available from line-item snapshot or payment callback.
    pub points: Option<i64>,
    /// Partner facts snapshotted at order creation, when the order's customer
    /// was bound to a partner.
    pub partner: Option<crate::OrderPartnerSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderOwnerItem {
    pub id: String,
    pub product_name: String,
    pub quantity: i64,
    pub unit_price: CommerceMoney,
    pub total_amount: CommerceMoney,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderOwnerDetail {
    pub summary: OrderOwnerSummary,
    pub payment_status: Option<String>,
    pub items: Vec<OrderOwnerItem>,
    pub out_trade_no: Option<String>,
    pub transaction_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderOwnerPaymentStatus {
    pub status: String,
    pub payment_status: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderOwnerStatistics {
    pub total_orders: i64,
    pub pending_payment: i64,
    pub pending_shipment: i64,
    pub pending_receipt: i64,
    pub completed: i64,
    pub total_amount: CommerceMoney,
}

impl OrderOwnerListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        status: Option<&str>,
        page: Option<i64>,
        page_size: Option<i64>,
        subject: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;

        let (page, page_size) = crate::validation::offset_list_params(page, page_size)?;

        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            status: optional_text(status),
            subject: optional_text(subject),
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

impl OrderOwnerDetailQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;

        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            order_id: order_id.trim().to_string(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CancelOwnerOrderCommand {
    pub cancel_reason: Option<String>,
    pub cancel_type: Option<String>,
    pub order_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub tenant_id: String,
}

impl CancelOwnerOrderCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
        cancel_reason: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        Self::with_cancel_type(
            tenant_id,
            organization_id,
            owner_user_id,
            order_id,
            cancel_reason,
            None,
        )
    }

    pub fn with_cancel_type(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
        cancel_reason: Option<&str>,
        cancel_type: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;

        Ok(Self {
            cancel_reason: optional_text(cancel_reason),
            cancel_type: optional_text(cancel_type),
            order_id: order_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateOwnerOrderCommand {
    pub checkout_session_id: String,
    pub idempotency_key: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub request_no: String,
    pub tenant_id: String,
    /// Partner facts snapshotted onto the order (resolved before insert).
    pub partner_snapshot: Option<crate::OrderPartnerSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateOwnerOrderOutcome {
    pub inventory_lines: Vec<crate::PhysicalInventoryLine>,
    pub merchant_organization_id: Option<String>,
    pub order_id: String,
    pub order_sn: String,
    pub status: String,
    pub total_amount: CommerceMoney,
    /// Payment countdown deadline (unix seconds) for the created order.
    pub expires_at: Option<String>,
    /// Partner facts snapshotted onto the created order, if any.
    pub partner_snapshot: Option<crate::OrderPartnerSnapshot>,
}

impl CreateOwnerOrderCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        checkout_session_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("checkout_session_id", checkout_session_id)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            checkout_session_id: checkout_session_id.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            request_no: request_no.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
            partner_snapshot: None,
        })
    }

    /// Attaches the partner snapshot resolved by `OrderPartnerRelationPort`.
    pub fn with_partner_snapshot(mut self, snapshot: Option<crate::OrderPartnerSnapshot>) -> Self {
        self.partner_snapshot = snapshot;
        self
    }
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
