use std::future::Future;
use std::pin::Pin;
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
    CancelOwnerOrderCommand, OrderOwnerDetail, OrderOwnerDetailQuery, OrderOwnerEventListQuery,
    OrderOwnerEventPage, OrderOwnerEventView, OrderOwnerListPage, OrderOwnerListQuery,
    OrderOwnerPaymentStatus, OrderOwnerStatistics, OrderOwnerSummary, PayOwnerOrderCommand,
    PayOwnerOrderCommandInput, PayOwnerOrderOutcome, PhysicalInventoryReservationPort,
    UnavailablePhysicalInventoryReservationPort,
};
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_payment_repository_sqlx::{
    PostgresCommercePaymentRecordStore,
};
use sdkwork_payment_service::{
    PaymentRecordItem, PaymentRecordOrderListPage, PaymentRecordOrderListQuery,
};
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::api_response::{
    conflict, map_service_error, not_found, offset_list_page_params_from_query, success_command,
    success_created_item, success_item, success_items, unauthorized, validation,
};
use crate::command_headers::required_app_write_command_headers;
use crate::owner_order_cancel::cancel_owner_order_with_payments_and_inventory;
use crate::subject::{app_runtime_subject_from_contexts, AppRuntimeSubject};

/// 允许的支付方式白名单，避免硬编码单一渠道。
///
/// `wechat_pay`/`alipay` 是收银台展示层方法（落到 native 扫码），
/// `wechat_jsapi`（微信 App 内拉起，需 openid）与 `alipay_wap`（H5/WAP 跳转）
/// 是环境差异化的渠道方法。
const ALLOWED_PAYMENT_METHODS: &[&str] = &[
    "wechat_pay",
    "wechat_jsapi",
    "alipay",
    "alipay_wap",
    "balance",
];

pub type CommerceOrderFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommerceOrderStore: Send + Sync {
    fn list_owner_orders<'a>(
        &'a self,
        query: OrderOwnerListQuery,
    ) -> CommerceOrderFuture<'a, OrderOwnerListPage>;

    fn retrieve_owner_order<'a>(
        &'a self,
        query: OrderOwnerDetailQuery,
    ) -> CommerceOrderFuture<'a, Option<OrderOwnerDetail>>;

    fn retrieve_owner_order_payment_status<'a>(
        &'a self,
        query: OrderOwnerDetailQuery,
    ) -> CommerceOrderFuture<'a, Option<OrderOwnerPaymentStatus>>;

    fn retrieve_owner_order_fulfillment_status<'a>(
        &'a self,
        query: OrderOwnerDetailQuery,
    ) -> CommerceOrderFuture<'a, Option<String>>;

    fn retrieve_owner_order_statistics<'a>(
        &'a self,
        tenant_id: String,
        organization_id: Option<String>,
        owner_user_id: String,
    ) -> CommerceOrderFuture<'a, OrderOwnerStatistics>;

    fn list_owner_order_events<'a>(
        &'a self,
        query: OrderOwnerEventListQuery,
    ) -> CommerceOrderFuture<'a, OrderOwnerEventPage>;

    fn cancel_owner_order<'a>(
        &'a self,
        command: CancelOwnerOrderCommand,
    ) -> CommerceOrderFuture<'a, ()>;

    fn confirm_owner_order_receipt<'a>(
        &'a self,
        query: OrderOwnerDetailQuery,
        request_no: String,
    ) -> CommerceOrderFuture<'a, ()>;
}

#[derive(Clone)]
struct AppOrderState {
    store: Arc<dyn CommerceOrderStore>,
    payments: Arc<dyn OwnerOrderPaymentStore>,
    payment_records: Arc<dyn OrderPaymentRecordStore>,
    inventory: Arc<dyn PhysicalInventoryReservationPort>,
}

pub trait OrderPaymentRecordStore: Send + Sync {
    fn list_payment_records_by_order<'a>(
        &'a self,
        query: PaymentRecordOrderListQuery,
    ) -> CommerceOrderFuture<'a, PaymentRecordOrderListPage>;
}

pub trait OwnerOrderPaymentStore: Send + Sync {
    fn pay_owner_order<'a>(
        &'a self,
        command: PayOwnerOrderCommand,
    ) -> CommerceOrderFuture<'a, PayOwnerOrderOutcome>;

    fn cancel_owner_order_payments<'a>(
        &'a self,
        command: CancelOwnerOrderCommand,
    ) -> CommerceOrderFuture<'a, ()>;
}

#[derive(Debug, Deserialize)]
struct OrderListQueryParams {
    page: Option<i64>,
    page_size: Option<i64>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OrderPaymentListQueryParams {
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderPaymentRecordResponse {
    payment_id: String,
    order_id: String,
    out_trade_no: String,
    payment_method: String,
    amount: String,
    created_at: String,
    status: String,
    status_name: String,
}

#[derive(Debug, Deserialize)]
struct OrderEventListQueryParams {
    page: Option<i64>,
    page_size: Option<i64>,
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
    remark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payment_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    product_image: Option<()>,
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
    event_id: String,
    order_id: String,
    event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    from_status: Option<String>,
    to_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    actor_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actor_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderStatusResponse {
    status: String,
    status_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderPaymentSuccessResponse {
    paid: bool,
    status: String,
    status_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CancelOrderRequest {
    #[serde(rename = "cancelReason", alias = "cancel_reason")]
    cancel_reason: Option<String>,
    #[serde(rename = "cancelType", alias = "cancel_type")]
    cancel_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PayOrderRequest {
    #[serde(rename = "paymentMethod", alias = "payment_method")]
    payment_method: Option<String>,
    #[serde(rename = "paymentPassword", alias = "payment_password")]
    payment_password: Option<String>,
    /// 微信 JSAPI 支付所需的 payer openid（微信 App 内经 OAuth 授权获得）。
    #[serde(rename = "openid", alias = "openId", alias = "payerOpenId")]
    openid: Option<String>,
}

impl PayOrderRequest {
    fn payment_method(&self) -> Option<&str> {
        self.payment_method
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
    }

    fn payment_password(&self) -> Option<&str> {
        self.payment_password
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
    }

    fn openid(&self) -> Option<&str> {
        self.openid
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
    }
}

/// 校验支付方式：必须非空且在白名单内，统一转小写以避免大小写歧义。
fn validate_payment_method(value: Option<&str>) -> Result<String, String> {
    let method = value.unwrap_or_default().trim().to_ascii_lowercase();
    if method.is_empty() {
        return Err("payment method must be provided".to_string());
    }
    if !ALLOWED_PAYMENT_METHODS
        .iter()
        .any(|allowed| *allowed == method)
    {
        return Err(format!(
            "payment method must be one of: {}",
            ALLOWED_PAYMENT_METHODS.join(", ")
        ));
    }
    Ok(method)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderPaymentParamsResponse {
    amount: String,
    order_id: String,
    out_trade_no: String,
    payment_id: String,
    payment_method: String,
    payment_params: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderStatisticsResponse {
    total_orders: i64,
    pending_payment: i64,
    pending_shipment: i64,
    pending_receipt: i64,
    completed: i64,
    total_amount: String,
}

use crate::owner_order_payment_enrich::{
    enriched_postgres_owner_order_payments,
};

pub fn app_order_router_with_postgres_pool(
    pool: PgPool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
) -> Router {
    build_app_order_router_with_inventory(
        Arc::new(PostgresCommerceOrderStore::new(pool.clone())),
        enriched_postgres_owner_order_payments(pool.clone(), registry, credentials),
        Arc::new(PostgresCommercePaymentRecordStore::new(pool)),
        Arc::new(UnavailablePhysicalInventoryReservationPort),
    )
}

pub fn app_order_router_with_postgres_pool_and_inventory(
    pool: PgPool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
    inventory: Arc<dyn PhysicalInventoryReservationPort>,
) -> Router {
    build_app_order_router_with_inventory(
        Arc::new(PostgresCommerceOrderStore::new(pool.clone())),
        enriched_postgres_owner_order_payments(pool.clone(), registry, credentials),
        Arc::new(PostgresCommercePaymentRecordStore::new(pool)),
        inventory,
    )
}

pub fn build_app_order_router(
    store: Arc<dyn CommerceOrderStore>,
    payments: Arc<dyn OwnerOrderPaymentStore>,
    payment_records: Arc<dyn OrderPaymentRecordStore>,
) -> Router {
    build_app_order_router_with_inventory(
        store,
        payments,
        payment_records,
        Arc::new(UnavailablePhysicalInventoryReservationPort),
    )
}

pub fn build_app_order_router_with_inventory(
    store: Arc<dyn CommerceOrderStore>,
    payments: Arc<dyn OwnerOrderPaymentStore>,
    payment_records: Arc<dyn OrderPaymentRecordStore>,
    inventory: Arc<dyn PhysicalInventoryReservationPort>,
) -> Router {
    Router::new()
        .route("/app/v3/api/orders", get(list_orders))
        .route("/app/v3/api/orders/statistics", get(fetch_order_statistics))
        .route("/app/v3/api/orders/{orderId}", get(fetch_order))
        .route(
            "/app/v3/api/orders/{orderId}/payments",
            get(list_order_payments).post(pay_order),
        )
        .route(
            "/app/v3/api/orders/{orderId}/status",
            get(fetch_order_status),
        )
        .route(
            "/app/v3/api/orders/{orderId}/payment_success",
            get(fetch_order_payment_success),
        )
        .route(
            "/app/v3/api/orders/{orderId}/events",
            get(fetch_order_events),
        )
        .route(
            "/app/v3/api/orders/{orderId}/cancellations",
            post(create_order_cancellation),
        )
        .route(
            "/app/v3/api/orders/{orderId}/receipt_confirmations",
            post(confirm_order_receipt),
        )
        .with_state(AppOrderState {
            store,
            payments,
            payment_records,
            inventory,
        })
}

async fn list_orders(
    State(state): State<AppOrderState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Query(params): Query<OrderListQueryParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match OrderOwnerListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        params.status.as_deref(),
        params.page,
        params.page_size,
        None,
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.list_owner_orders(query).await {
        Ok(page) => {
            let page_params = offset_list_page_params_from_query(page.page, page.page_size);
            let mapped = page.items.into_iter().map(map_order_summary).collect();
            success_items(ctx, mapped, page.total, page_params)
        }
        Err(error) => map_service_error(ctx, error),
    }
}

async fn fetch_order_statistics(
    State(state): State<AppOrderState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };

    match state
        .store
        .retrieve_owner_order_statistics(
            subject.tenant_id,
            subject.organization_id,
            subject.user_id,
        )
        .await
    {
        Ok(statistics) => success_item(ctx, map_statistics(statistics)),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn fetch_order(
    State(state): State<AppOrderState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(order_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match OrderOwnerDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_id,
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.retrieve_owner_order(query).await {
        Ok(Some(detail)) => success_item(ctx, map_order_detail(detail)),
        Ok(None) => not_found(ctx, "order was not found"),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn fetch_order_status(
    State(state): State<AppOrderState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(order_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match OrderOwnerDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_id,
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.retrieve_owner_order(query).await {
        Ok(Some(detail)) => {
            let summary = map_order_summary(detail.summary);
            success_item(
                ctx,
                OrderStatusResponse {
                    status: summary.status,
                    status_name: summary.status_name,
                },
            )
        }
        Ok(None) => not_found(ctx, "order was not found"),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn fetch_order_payment_success(
    State(state): State<AppOrderState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(order_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let status = match retrieve_payment_success_order(&*state.store, &subject, &order_id).await {
        Ok(status) => status,
        Err(error) => return map_service_error(ctx, error),
    };

    match status {
        Some(status) => {
            let paid = order_payment_succeeded(&status);
            let status_name = format_order_status_name(&status.status);
            success_item(
                ctx,
                OrderPaymentSuccessResponse {
                    paid,
                    status: status.status,
                    status_name,
                },
            )
        }
        None => not_found(ctx, "order was not found"),
    }
}

async fn retrieve_payment_success_order(
    store: &dyn CommerceOrderStore,
    subject: &AppRuntimeSubject,
    order_id: &str,
) -> Result<Option<OrderOwnerPaymentStatus>, CommerceServiceError> {
    for organization_id in payment_success_organization_scopes(subject.organization_id.as_deref()) {
        let query = OrderOwnerDetailQuery::new(
            &subject.tenant_id,
            organization_id.as_deref(),
            &subject.user_id,
            order_id,
        )?;
        if let Some(status) = store.retrieve_owner_order_payment_status(query).await? {
            return Ok(Some(status));
        }
    }
    Ok(None)
}

fn order_payment_succeeded(status: &OrderOwnerPaymentStatus) -> bool {
    matches!(
        status.status.trim().to_ascii_lowercase().as_str(),
        "paid" | "completed" | "fulfilled" | "awaiting_external_fulfillment"
    ) || status.payment_status.as_deref().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "success" | "succeeded" | "paid"
        )
    })
}

fn payment_success_organization_scopes(organization_id: Option<&str>) -> Vec<Option<String>> {
    let organization_id = organization_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    match organization_id {
        Some(organization_id) => vec![Some(organization_id)],
        None => vec![None, Some("0".to_owned())],
    }
}

async fn fetch_order_events(
    State(state): State<AppOrderState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(order_id): Path<String>,
    Query(params): Query<OrderEventListQueryParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match OrderOwnerEventListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_id,
        params.page,
        params.page_size,
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.list_owner_order_events(query).await {
        Ok(page) => {
            let page_params = offset_list_page_params_from_query(page.page, page.page_size);
            let mapped = page.items.into_iter().map(map_order_event).collect();
            success_items(ctx, mapped, page.total, page_params)
        }
        Err(error) => map_service_error(ctx, error),
    }
}

async fn create_order_cancellation(
    state: State<AppOrderState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    body: Option<Json<CancelOrderRequest>>,
) -> Response {
    cancel_order_impl(
        state,
        runtime_context,
        request_context,
        headers,
        order_id,
        body,
    )
    .await
}

async fn cancel_order_impl(
    State(state): State<AppOrderState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    order_id: String,
    body: Option<Json<CancelOrderRequest>>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let body = body.map(|Json(value)| value).unwrap_or(CancelOrderRequest {
        cancel_reason: None,
        cancel_type: None,
    });
    let write_headers = match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
        format!("order-cancel-{order_id}-{idempotency_key}")
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let cancel_reason = body.cancel_reason.clone();
    let cancel_type = body.cancel_type.clone();
    let command = match CancelOwnerOrderCommand::with_cancel_type(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_id,
        cancel_reason.as_deref(),
        cancel_type.as_deref(),
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match cancel_owner_order_with_payments_and_inventory(
        &*state.store,
        &*state.payments,
        &*state.inventory,
        command,
        &write_headers.request_no,
    )
    .await
    {
        Ok(()) => success_command(ctx, Some(order_id.clone()), Some("cancelled".to_string())),
        Err(error) => map_service_error(ctx, error),
    }
}

/// Buyer confirms physical receipt: `paid/fulfilled/shipped` orders advance
/// to `completed` (fulfillment `delivered`) with a lifecycle event
/// (idempotent for replays).
async fn confirm_order_receipt(
    State(state): State<AppOrderState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let write_headers = match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
        format!("order-receipt-{order_id}-{idempotency_key}")
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let query = match OrderOwnerDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_id,
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };
    match state
        .store
        .confirm_owner_order_receipt(query, write_headers.request_no)
        .await
    {
        Ok(()) => success_command(ctx, Some(order_id.clone()), Some("completed".to_string())),
        Err(error) if error.code() == "conflict" => conflict(ctx, error.message()),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn list_order_payments(
    State(state): State<AppOrderState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(order_id): Path<String>,
    Query(params): Query<OrderPaymentListQueryParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let (page_number, page_size) = match sdkwork_order_service::validation::offset_list_params(
        params.page,
        params.page_size,
    ) {
        Ok(value) => value,
        Err(error) => return map_service_error(ctx, error),
    };
    let query = match PaymentRecordOrderListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_id,
    ) {
        Ok(query) => query.with_paging((page_number - 1) * page_size, page_size),
        Err(error) => return map_service_error(ctx, error),
    };

    match state
        .payment_records
        .list_payment_records_by_order(query)
        .await
    {
        Ok(page) => {
            let page_params = offset_list_page_params_from_query(page_number, page_size);
            let items = page
                .items
                .into_iter()
                .map(map_order_payment_record)
                .collect::<Vec<_>>();
            success_items(ctx, items, page.total_items, page_params)
        }
        Err(error) => map_service_error(ctx, error),
    }
}

async fn pay_order(
    State(state): State<AppOrderState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    body: Option<Json<PayOrderRequest>>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let body = body.map(|Json(value)| value).unwrap_or(PayOrderRequest {
        payment_method: None,
        payment_password: None,
        openid: None,
    });
    let payment_method = match validate_payment_method(body.payment_method()) {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let write_headers = match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
        format!("pay-{order_id}-{idempotency_key}")
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let callback_payload = body
        .payment_password()
        .map(|password| serde_json::json!({ "paymentPassword": password }).to_string());
    // wechat_jsapi 适配器要求 metadata.openid；其余方法不需要额外元数据。
    let payment_metadata = match body.openid() {
        Some(openid) => serde_json::json!({ "openid": openid }),
        None => serde_json::json!({}),
    };
    let command = match PayOwnerOrderCommand::new(PayOwnerOrderCommandInput {
        tenant_id: subject.tenant_id.clone(),
        organization_id: subject.organization_id.clone(),
        owner_user_id: subject.user_id.clone(),
        order_id: order_id.clone(),
        payment_method,
        payment_scene: None,
        payment_attempt_callback_payload: callback_payload,
        payment_metadata,
        request_no: write_headers.request_no.clone(),
        idempotency_key: write_headers.idempotency_key.clone(),
    }) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.payments.pay_owner_order(command).await {
        Ok(outcome) => success_created_item(ctx, map_pay_outcome(outcome)),
        Err(error) => map_service_error(ctx, error),
    }
}

fn map_pay_outcome(value: PayOwnerOrderOutcome) -> OrderPaymentParamsResponse {
    OrderPaymentParamsResponse {
        amount: value.amount.as_str().to_owned(),
        order_id: value.order_id,
        out_trade_no: value.out_trade_no,
        payment_id: value.payment_id,
        payment_method: value.payment_method,
        payment_params: value.payment_params,
    }
}

fn map_order_event(value: OrderOwnerEventView) -> OrderEventResponse {
    OrderEventResponse {
        event_id: value.event_id,
        order_id: value.order_id,
        event_type: value.event_type,
        from_status: value.from_status,
        to_status: value.to_status,
        actor_type: value.actor_type,
        actor_id: value.actor_id,
        message: value.message,
        created_at: value.created_at,
    }
}

fn map_order_summary(value: OrderOwnerSummary) -> OrderSummaryResponse {
    let status_name = format_order_status_name(&value.status);
    OrderSummaryResponse {
        order_id: value.order_id,
        order_sn: value.order_sn,
        status: value.status,
        status_name,
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
        remark: None,
        payment_provider: None,
        product_image: None,
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

fn map_statistics(value: OrderOwnerStatistics) -> OrderStatisticsResponse {
    OrderStatisticsResponse {
        total_orders: value.total_orders,
        pending_payment: value.pending_payment,
        pending_shipment: value.pending_shipment,
        pending_receipt: value.pending_receipt,
        completed: value.completed,
        total_amount: value.total_amount.as_str().to_owned(),
    }
}

fn format_order_status_name(status: &str) -> String {
    match status.trim().to_ascii_lowercase().as_str() {
        "pending_payment" | "unpaid" | "wait_pay" => "Pending payment".to_owned(),
        "paid" => "Paid".to_owned(),
        "completed" | "finished" => "Completed".to_owned(),
        "cancelled" | "canceled" | "closed" => "Cancelled".to_owned(),
        "expired" | "timeout" => "Expired".to_owned(),
        "refunding" => "Refunding".to_owned(),
        "refunded" => "Refunded".to_owned(),
        "fulfilled" => "Fulfilled".to_owned(),
        other => other.to_owned(),
    }
}

fn map_order_payment_record(value: PaymentRecordItem) -> OrderPaymentRecordResponse {
    let status = map_order_payment_status_code(&value.status);
    OrderPaymentRecordResponse {
        payment_id: value.id,
        order_id: value.order_id,
        out_trade_no: value.order_no,
        payment_method: value.method,
        amount: value.amount.as_str().to_owned(),
        created_at: value.date,
        status: status.to_owned(),
        status_name: format_order_payment_status_name(status),
    }
}

fn map_order_payment_status_code(status: &str) -> &'static str {
    match status.trim().to_ascii_lowercase().as_str() {
        "success" | "succeeded" | "paid" => "SUCCESS",
        "failed" => "FAILED",
        "timeout" => "TIMEOUT",
        "closed" | "canceled" | "cancelled" => "CLOSED",
        _ => "PENDING",
    }
}

fn format_order_payment_status_name(status: &str) -> String {
    match status {
        "SUCCESS" => "Success".to_owned(),
        "FAILED" => "Failed".to_owned(),
        "TIMEOUT" => "Timeout".to_owned(),
        "CLOSED" => "Closed".to_owned(),
        _ => "Pending".to_owned(),
    }
}

impl CommerceOrderStore for PostgresCommerceOrderStore {
    fn list_owner_orders<'a>(
        &'a self,
        query: OrderOwnerListQuery,
    ) -> CommerceOrderFuture<'a, OrderOwnerListPage> {
        Box::pin(async move { self.list_owner_orders(query).await })
    }

    fn retrieve_owner_order<'a>(
        &'a self,
        query: OrderOwnerDetailQuery,
    ) -> CommerceOrderFuture<'a, Option<OrderOwnerDetail>> {
        Box::pin(async move { self.retrieve_owner_order(query).await })
    }

    fn retrieve_owner_order_payment_status<'a>(
        &'a self,
        query: OrderOwnerDetailQuery,
    ) -> CommerceOrderFuture<'a, Option<OrderOwnerPaymentStatus>> {
        Box::pin(async move { self.retrieve_owner_order_payment_status(query).await })
    }

    fn retrieve_owner_order_fulfillment_status<'a>(
        &'a self,
        query: OrderOwnerDetailQuery,
    ) -> CommerceOrderFuture<'a, Option<String>> {
        Box::pin(async move { self.retrieve_owner_order_fulfillment_status(query).await })
    }

    fn retrieve_owner_order_statistics<'a>(
        &'a self,
        tenant_id: String,
        organization_id: Option<String>,
        owner_user_id: String,
    ) -> CommerceOrderFuture<'a, OrderOwnerStatistics> {
        Box::pin(async move {
            self.retrieve_owner_order_statistics(
                tenant_id.as_str(),
                organization_id.as_deref(),
                owner_user_id.as_str(),
            )
            .await
        })
    }

    fn list_owner_order_events<'a>(
        &'a self,
        query: OrderOwnerEventListQuery,
    ) -> CommerceOrderFuture<'a, OrderOwnerEventPage> {
        Box::pin(async move { self.list_owner_order_events(query).await })
    }

    fn cancel_owner_order<'a>(
        &'a self,
        command: CancelOwnerOrderCommand,
    ) -> CommerceOrderFuture<'a, ()> {
        Box::pin(async move { self.cancel_owner_order(command).await })
    }

    fn confirm_owner_order_receipt<'a>(
        &'a self,
        query: OrderOwnerDetailQuery,
        request_no: String,
    ) -> CommerceOrderFuture<'a, ()> {
        Box::pin(async move {
            self.confirm_owner_order_receipt(query, &request_no).await
        })
    }
}

impl OrderPaymentRecordStore for PostgresCommercePaymentRecordStore {
    fn list_payment_records_by_order<'a>(
        &'a self,
        query: PaymentRecordOrderListQuery,
    ) -> CommerceOrderFuture<'a, PaymentRecordOrderListPage> {
        Box::pin(async move { self.list_payment_records_by_order(query).await })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        order_payment_succeeded, payment_success_organization_scopes,
        retrieve_payment_success_order, AppRuntimeSubject,
    };
    use sdkwork_order_service::OrderOwnerPaymentStatus;

    #[test]
    fn payment_success_lookup_falls_back_to_platform_scope_only_when_unscoped() {
        assert_eq!(
            vec![None, Some("0".to_owned())],
            payment_success_organization_scopes(None)
        );
        assert_eq!(
            vec![Some("org-1".to_owned())],
            payment_success_organization_scopes(Some(" org-1 "))
        );
    }

    #[test]
    fn payment_success_accepts_order_or_payment_success_states() {
        assert!(order_payment_succeeded(&OrderOwnerPaymentStatus {
            status: "pending_payment".to_owned(),
            payment_status: Some("succeeded".to_owned()),
        }));
        assert!(order_payment_succeeded(&OrderOwnerPaymentStatus {
            status: "fulfilled".to_owned(),
            payment_status: None,
        }));
        assert!(!order_payment_succeeded(&OrderOwnerPaymentStatus {
            status: "pending_payment".to_owned(),
            payment_status: Some("pending".to_owned()),
        }));
    }


}
