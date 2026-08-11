use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_repository_sqlx::PostgresCommerceOrderStore;
use sdkwork_order_service::{
    CancelManagementOrderCommand, CloseManagementOrderCommand, OrderCancellationListQuery,
    OrderCancellationPage, OrderCancellationView, OrderManagementDetailQuery,
    OrderManagementEventListQuery, OrderManagementEventPage, OrderManagementEventView,
    OrderManagementListPage, OrderManagementListQuery, OrderOwnerDetail, OrderOwnerSummary,
    PhysicalInventoryReservationPort,
};
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::api_response::{
    conflict as api_conflict, map_service_error, not_found as api_not_found,
    offset_list_page_params_from_query, success_command, success_item, success_items, validation,
};
use crate::backend_acl::require_backend_operator;
use crate::backend_command_headers::resolve_required_backend_write_command_headers;
use crate::backend_management_lifecycle::{
    cancel_management_order_with_payments, close_management_order_with_payments,
    BackendManagementOrderStore, BackendManagementPaymentStore,
};

/// Permission codes enforced on the backend order admin API surface.
///
/// The codes are domain-scoped (`commerce.orders.*`) so they can be granted
/// through standard SDKWork IAM permission catalogs without colliding with
/// other commerce capabilities (payment, catalog, etc.).
mod permissions {
    /// Read access to orders, events, and cancellations.
    pub const READ: &str = "commerce.orders.read";
    /// Write access: cancel, close, or otherwise mutate order state.
    pub const MANAGE: &str = "commerce.orders.manage";
}

#[derive(Clone)]
struct BackendOrderAdminState {
    orders: BackendManagementOrderStore,
    payments: BackendManagementPaymentStore,
    inventory: Arc<dyn PhysicalInventoryReservationPort>,
}

#[derive(Debug, Deserialize)]
struct OrderListParams {
    status: Option<String>,
    q: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OrderEventListParams {
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CancellationListParams {
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CancelOrderBody {
    reason: Option<String>,
    /// 取消类型，例如 `user_request`、`admin_operation`、`system_timeout`。
    /// 透传到 `CancelManagementOrderCommand::with_cancel_type` 用于审计与事件溯源。
    cancel_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CloseOrderBody {
    reason: Option<String>,
    /// 关闭类型，例如 `timeout`、`manual`、`risk_control`。
    /// 透传到 `CloseManagementOrderCommand::with_close_type` 用于审计与事件溯源。
    close_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderSummaryResponse {
    order_id: String,
    order_sn: String,
    status: String,
    status_name: String,
    subject: String,
    total_amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    paid_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    discount_amount: Option<String>,
    quantity: i64,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pay_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expire_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payment_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    partner_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    partner_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    partner_level_no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    partner_status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderDetailResponse {
    #[serde(flatten)]
    summary: OrderSummaryResponse,
    items: Vec<OrderItemResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    out_trade_no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transaction_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderItemResponse {
    id: String,
    product_name: String,
    quantity: i64,
    unit_price: String,
    total_amount: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderEventResponse {
    id: String,
    event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    from_status: Option<String>,
    to_status: String,
    actor_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    actor_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderCancellationResponse {
    id: String,
    order_id: String,
    status: String,
    reason_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason_message: Option<String>,
    created_at: String,
}

pub fn backend_order_admin_router_with_postgres_pool(
    pool: PgPool,
    inventory: Arc<dyn PhysicalInventoryReservationPort>,
) -> Router {
    let credentials = ProviderCredentialBundle::from_env();
    let registry = Arc::new(PaymentProviderRegistry::from_credentials(
        credentials.clone(),
    ));
    build_backend_order_admin_router(
        BackendManagementOrderStore::Postgres(Arc::new(PostgresCommerceOrderStore::new(
            pool.clone(),
        ))),
        BackendManagementPaymentStore::Postgres {
            pool,
            registry,
            credentials,
        },
        inventory,
    )
}

fn build_backend_order_admin_router(
    orders: BackendManagementOrderStore,
    payments: BackendManagementPaymentStore,
    inventory: Arc<dyn PhysicalInventoryReservationPort>,
) -> Router {
    let state = BackendOrderAdminState {
        orders,
        payments,
        inventory,
    };
    Router::new()
        .route("/backend/v3/api/orders", get(list_orders))
        .route(
            "/backend/v3/api/orders/cancellations",
            get(list_cancellations),
        )
        .route("/backend/v3/api/orders/{orderId}", get(retrieve_order))
        .route(
            "/backend/v3/api/orders/{orderId}/cancel",
            post(cancel_order),
        )
        .route("/backend/v3/api/orders/{orderId}/close", post(close_order))
        .route(
            "/backend/v3/api/orders/{orderId}/events",
            get(list_order_events),
        )
        .with_state(state)
}

impl BackendManagementOrderStore {
    async fn list_management_orders(
        &self,
        query: OrderManagementListQuery,
    ) -> Result<OrderManagementListPage, CommerceServiceError> {
        match self {
            Self::Postgres(store) => store.list_management_orders(query).await,        }
    }

    async fn retrieve_management_order(
        &self,
        query: OrderManagementDetailQuery,
    ) -> Result<Option<OrderOwnerDetail>, CommerceServiceError> {
        match self {
            Self::Postgres(store) => store.retrieve_management_order(query).await,        }
    }

    async fn list_management_order_events(
        &self,
        query: OrderManagementEventListQuery,
    ) -> Result<OrderManagementEventPage, CommerceServiceError> {
        match self {
            Self::Postgres(store) => store.list_management_order_events(query).await,        }
    }

    async fn list_order_cancellations(
        &self,
        query: OrderCancellationListQuery,
    ) -> Result<OrderCancellationPage, CommerceServiceError> {
        match self {
            Self::Postgres(store) => store.list_order_cancellations(query).await,        }
    }
}

async fn list_orders(
    State(state): State<BackendOrderAdminState>,
    Extension(runtime_context): Extension<IamAppContext>,
    request_context: Extension<WebRequestContext>,
    Query(params): Query<OrderListParams>,
) -> Response {
    let ctx = Some(&request_context.0);
    let subject = match require_backend_operator(ctx, runtime_context, permissions::READ) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let query = match OrderManagementListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        params.status.as_deref(),
        params.q.as_deref(),
        params.page,
        params.page_size,
    ) {
        Ok(query) => query,
        Err(error) => return validation(ctx, error.message()),
    };

    match state.orders.list_management_orders(query.clone()).await {
        Ok(page) => {
            let page_params = offset_list_page_params_from_query(query.page, query.page_size);
            success_items(
                ctx,
                page.items.into_iter().map(map_order_summary).collect(),
                page.total,
                page_params,
            )
        }
        Err(error) => map_service_error(ctx, error),
    }
}

async fn retrieve_order(
    State(state): State<BackendOrderAdminState>,
    Extension(runtime_context): Extension<IamAppContext>,
    request_context: Extension<WebRequestContext>,
    Path(order_id): Path<String>,
) -> Response {
    let ctx = Some(&request_context.0);
    let subject = match require_backend_operator(ctx, runtime_context, permissions::READ) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let query = match OrderManagementDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &order_id,
    ) {
        Ok(query) => query,
        Err(error) => return validation(ctx, error.message()),
    };

    match state.orders.retrieve_management_order(query).await {
        Ok(Some(detail)) => success_item(ctx, map_order_detail(detail)),
        Ok(None) => api_not_found(ctx, "order not found"),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn cancel_order(
    State(state): State<BackendOrderAdminState>,
    Extension(runtime_context): Extension<IamAppContext>,
    request_context: Extension<WebRequestContext>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    body: Option<Json<CancelOrderBody>>,
) -> Response {
    let ctx = Some(&request_context.0);
    let subject = match require_backend_operator(ctx, runtime_context, permissions::MANAGE) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let body = body.map(|Json(value)| value).unwrap_or(CancelOrderBody {
        reason: None,
        cancel_type: None,
    });
    let _write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("admin-cancel-{order_id}-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let command = match CancelManagementOrderCommand::with_cancel_type(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &order_id,
        body.reason.as_deref(),
        body.cancel_type.as_deref(),
    ) {
        Ok(command) => command,
        Err(error) => return validation(ctx, error.message()),
    };

    match cancel_management_order_with_payments(
        &state.orders,
        &state.payments,
        state.inventory.as_ref(),
        command,
    )
    .await
    {
        Ok(()) => success_command(ctx, Some(order_id), Some("cancelled".to_owned())),
        Err(error) if error.code() == "conflict" => api_conflict(ctx, error.message()),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn close_order(
    State(state): State<BackendOrderAdminState>,
    Extension(runtime_context): Extension<IamAppContext>,
    request_context: Extension<WebRequestContext>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    body: Option<Json<CloseOrderBody>>,
) -> Response {
    let ctx = Some(&request_context.0);
    let subject = match require_backend_operator(ctx, runtime_context, permissions::MANAGE) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let body = body.map(|Json(value)| value).unwrap_or(CloseOrderBody {
        reason: None,
        close_type: None,
    });
    let _write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("admin-close-{order_id}-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let command = match CloseManagementOrderCommand::with_close_type(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &order_id,
        body.reason.as_deref(),
        body.close_type.as_deref(),
    ) {
        Ok(command) => command,
        Err(error) => return validation(ctx, error.message()),
    };

    match close_management_order_with_payments(
        &state.orders,
        &state.payments,
        state.inventory.as_ref(),
        command,
    )
    .await
    {
        Ok(()) => success_command(ctx, Some(order_id), Some("closed".to_owned())),
        Err(error) if error.code() == "conflict" => api_conflict(ctx, error.message()),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn list_order_events(
    State(state): State<BackendOrderAdminState>,
    Extension(runtime_context): Extension<IamAppContext>,
    request_context: Extension<WebRequestContext>,
    Path(order_id): Path<String>,
    Query(params): Query<OrderEventListParams>,
) -> Response {
    let ctx = Some(&request_context.0);
    let subject = match require_backend_operator(ctx, runtime_context, permissions::READ) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let query = match OrderManagementEventListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &order_id,
        params.page,
        params.page_size,
    ) {
        Ok(query) => query,
        Err(error) => return validation(ctx, error.message()),
    };

    match state
        .orders
        .list_management_order_events(query.clone())
        .await
    {
        Ok(page) => {
            let page_params = offset_list_page_params_from_query(query.page, query.page_size);
            success_items(
                ctx,
                page.items.into_iter().map(map_order_event).collect(),
                page.total,
                page_params,
            )
        }
        Err(error) => map_service_error(ctx, error),
    }
}

async fn list_cancellations(
    State(state): State<BackendOrderAdminState>,
    Extension(runtime_context): Extension<IamAppContext>,
    request_context: Extension<WebRequestContext>,
    Query(params): Query<CancellationListParams>,
) -> Response {
    let ctx = Some(&request_context.0);
    let subject = match require_backend_operator(ctx, runtime_context, permissions::READ) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let query = match OrderCancellationListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        params.status.as_deref(),
        params.page,
        params.page_size,
    ) {
        Ok(query) => query,
        Err(error) => return validation(ctx, error.message()),
    };

    match state.orders.list_order_cancellations(query.clone()).await {
        Ok(page) => {
            let page_params = offset_list_page_params_from_query(query.page, query.page_size);
            success_items(
                ctx,
                page.items.into_iter().map(map_order_cancellation).collect(),
                page.total,
                page_params,
            )
        }
        Err(error) => map_service_error(ctx, error),
    }
}

fn map_order_summary(value: OrderOwnerSummary) -> OrderSummaryResponse {
    OrderSummaryResponse {
        order_id: value.order_id,
        order_sn: value.order_sn,
        status: value.status.clone(),
        status_name: format_order_status_name(&value.status),
        subject: value.subject,
        total_amount: value.total_amount.as_str().to_owned(),
        paid_amount: value.paid_amount.map(|amount| amount.as_str().to_owned()),
        discount_amount: value
            .discount_amount
            .map(|amount| amount.as_str().to_owned()),
        quantity: value.quantity,
        created_at: value.created_at,
        pay_time: value.pay_time,
        expire_time: value.expire_time,
        payment_method: value.payment_method,
        partner_id: value.partner.as_ref().map(|partner| partner.partner_id.clone()),
        partner_name: value.partner.as_ref().map(|partner| partner.name.clone()),
        partner_level_no: value
            .partner
            .as_ref()
            .map(|partner| partner.level_no.clone()),
        partner_status: value
            .partner
            .as_ref()
            .map(|partner| partner.status.clone()),
    }
}

fn map_order_detail(value: OrderOwnerDetail) -> OrderDetailResponse {
    OrderDetailResponse {
        summary: map_order_summary(value.summary),
        items: value
            .items
            .into_iter()
            .map(|item| OrderItemResponse {
                id: item.id,
                product_name: item.product_name,
                quantity: item.quantity,
                unit_price: item.unit_price.as_str().to_owned(),
                total_amount: item.total_amount.as_str().to_owned(),
            })
            .collect(),
        out_trade_no: value.out_trade_no,
        transaction_id: value.transaction_id,
    }
}

fn map_order_event(value: OrderManagementEventView) -> OrderEventResponse {
    OrderEventResponse {
        id: value.id,
        event_type: value.event_type,
        from_status: value.from_status,
        to_status: value.to_status,
        actor_type: value.actor_type,
        actor_id: value.actor_id,
        message: value.message,
        created_at: value.created_at,
    }
}

fn map_order_cancellation(value: OrderCancellationView) -> OrderCancellationResponse {
    OrderCancellationResponse {
        id: value.id,
        order_id: value.order_id,
        status: value.status,
        reason_code: value.reason_code,
        reason_message: value.reason_message,
        created_at: value.created_at,
    }
}

fn format_order_status_name(status: &str) -> String {
    match status.trim().to_ascii_lowercase().as_str() {
        "pending_payment" | "unpaid" | "wait_pay" => "Pending payment".to_owned(),
        "paid" => "Paid".to_owned(),
        "completed" | "finished" => "Completed".to_owned(),
        "cancelled" | "canceled" | "closed" => "Cancelled".to_owned(),
        "expired" | "timeout" => "Expired".to_owned(),
        other => other.to_owned(),
    }
}
