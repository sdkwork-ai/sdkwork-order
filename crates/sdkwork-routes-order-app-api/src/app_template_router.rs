//! App-template purchase routes: buy a published template and list what the
//! buyer owns.
//!
//! The deployments module owns the template catalog; this router owns the
//! trade. `appTemplateOrders.create` prices the listing, records the order (and
//! for a paid listing starts payment through the same payment seam membership
//! uses), and `appTemplateOrders.list` returns the buyer's orders — which are
//! the entitlement, because a settled order is what allows the install.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_repository_sqlx::PostgresCommerceAppTemplateOrderStore;
use sdkwork_order_service::{
    AppTemplateOrderListQuery, AppTemplateOrderPage, AppTemplateOrderSummary,
    CreateAppTemplateOrderCommand, CreateAppTemplateOrderOutcome, APP_TEMPLATE_ORDER_SUBJECT,
};
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_payment_service::{
    PayOwnerOrderCommand, PayOwnerOrderCommandInput, PayOwnerOrderOutcome,
};
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api_response::{
    map_service_error, parse_offset_list_params_validated, success_created_item, success_items,
    unauthorized, validation,
};
use crate::command_headers::required_app_write_command_headers;
use crate::membership_router::{
    payment_expire_seconds, payment_scene, provider_qr_code, stable_header_token, stable_hex_token,
    DEFAULT_PAYMENT_PRODUCT,
};
use crate::order_router::OwnerOrderPaymentStore;
use crate::owner_order_payment_enrich::enriched_postgres_owner_order_payments;
use crate::subject::{app_runtime_subject_from_contexts, AppRuntimeSubject};

const PLATFORM_ORGANIZATION_SCOPE_SENTINEL: &str = "0";
const ALLOWED_PAYMENT_METHODS: &[&str] = &["wechat_pay", "alipay", "balance"];
/// Stored snapshot value for an order paid through the H5 cashier, which picks
/// its provider after the order exists.
const CASHIER_PAYMENT_METHOD: &str = "cashier";

pub type CommerceAppTemplateOrderFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommerceAppTemplateOrderStore: Send + Sync {
    fn create_app_template_order<'a>(
        &'a self,
        command: CreateAppTemplateOrderCommand,
    ) -> CommerceAppTemplateOrderFuture<'a, CreateAppTemplateOrderOutcome>;

    fn list_app_template_orders<'a>(
        &'a self,
        query: AppTemplateOrderListQuery,
    ) -> CommerceAppTemplateOrderFuture<'a, AppTemplateOrderPage>;
}

#[derive(Clone)]
struct AppTemplateOrderState {
    store: Arc<dyn CommerceAppTemplateOrderStore>,
    payments: Option<Arc<dyn OwnerOrderPaymentStore>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateAppTemplateOrderRequest {
    template_uuid: Option<String>,
    payment_method: Option<String>,
    payment_product: Option<String>,
    client_request_no: Option<String>,
    source: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateAppTemplateOrderResponse {
    amount: String,
    cashier_url: String,
    currency_code: String,
    expires_at: String,
    order_id: String,
    order_no: String,
    out_trade_no: String,
    payment_method: String,
    payment_product: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    payment_id: Option<String>,
    payment_params: BTreeMap<String, String>,
    qr_code: String,
    qr_code_type: String,
    reused: bool,
    status: String,
    template_name: String,
    template_uuid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    version_uuid: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppTemplateOrderSummaryResponse {
    amount: String,
    created_at: String,
    currency_code: String,
    fulfillment_status: String,
    order_id: String,
    order_no: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    paid_at: Option<String>,
    status: String,
    template_name: String,
    template_uuid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    version_uuid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AppTemplateOrderListParams {
    page: Option<i64>,
    page_size: Option<i64>,
}
impl CommerceAppTemplateOrderStore for PostgresCommerceAppTemplateOrderStore {
    fn create_app_template_order<'a>(
        &'a self,
        command: CreateAppTemplateOrderCommand,
    ) -> CommerceAppTemplateOrderFuture<'a, CreateAppTemplateOrderOutcome> {
        Box::pin(async move { self.create_app_template_order(command).await })
    }

    fn list_app_template_orders<'a>(
        &'a self,
        query: AppTemplateOrderListQuery,
    ) -> CommerceAppTemplateOrderFuture<'a, AppTemplateOrderPage> {
        Box::pin(async move { self.list_app_template_orders(&query).await })
    }
}

pub fn app_app_template_order_router_with_postgres_pool(pool: PgPool) -> Router {
    let credentials = ProviderCredentialBundle::from_env();
    let registry = Arc::new(PaymentProviderRegistry::from_credentials(
        credentials.clone(),
    ));
    let payments = enriched_postgres_owner_order_payments(pool.clone(), registry, credentials);
    build_app_app_template_order_router(
        Arc::new(PostgresCommerceAppTemplateOrderStore::new(pool)),
        payments,
    )
}

pub fn build_app_app_template_order_router(
    store: Arc<dyn CommerceAppTemplateOrderStore>,
    payments: Arc<dyn OwnerOrderPaymentStore>,
) -> Router {
    build_app_app_template_order_router_state(store, Some(payments))
}

fn build_app_app_template_order_router_state(
    store: Arc<dyn CommerceAppTemplateOrderStore>,
    payments: Option<Arc<dyn OwnerOrderPaymentStore>>,
) -> Router {
    Router::new()
        .route(
            "/app/v3/api/app_template_orders",
            post(create_app_template_order).get(list_app_template_orders),
        )
        .with_state(AppTemplateOrderState { store, payments })
}

async fn create_app_template_order(
    State(state): State<AppTemplateOrderState>,
    runtime_context: Option<axum::extract::Extension<IamAppContext>>,
    request_context: Option<axum::extract::Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(request): Json<CreateAppTemplateOrderRequest>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let template_uuid = match validate_template_uuid(request.template_uuid.as_deref()) {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let payment_product = match validate_payment_product(request.payment_product.as_deref()) {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let method = match validate_payment_method(request.payment_method.as_deref(), &payment_product)
    {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let write_headers = match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
        fallback_request_no(&subject, &template_uuid, &method, idempotency_key)
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let command = match build_create_app_template_order_command(
        &subject,
        &template_uuid,
        &method,
        &payment_product,
        &write_headers.request_no,
        &write_headers.idempotency_key,
        request.client_request_no.as_deref(),
        request.source.as_deref(),
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    let outcome = match state.store.create_app_template_order(command).await {
        Ok(outcome) => outcome,
        Err(error) => return map_service_error(ctx, error),
    };

    // A zero-amount listing is already settled by the store: there is nothing
    // to collect and no provider to call.
    if outcome.status != "pending_payment" || payment_product == DEFAULT_PAYMENT_PRODUCT {
        return success_created_item(ctx, map_create_outcome(outcome, &payment_product, None));
    }

    let Some(payments) = state.payments.as_ref() else {
        return map_service_error(
            ctx,
            CommerceServiceError::provider_unavailable(
                "app template order payment orchestration is not configured",
            ),
        );
    };
    let payment_execution_token = stable_hex_token(&format!(
        "app-template-payment|{}|{}|{}|{}",
        subject.tenant_id, outcome.order_id, method, payment_product
    ));
    let pay_command = match PayOwnerOrderCommand::new(PayOwnerOrderCommandInput {
        tenant_id: subject.tenant_id.clone(),
        organization_id: Some(organization_scope(subject.organization_id.as_deref())),
        owner_user_id: subject.user_id.clone(),
        order_id: outcome.order_id.clone(),
        payment_method: method,
        payment_scene: Some(payment_scene(&payment_product).to_string()),
        payment_attempt_callback_payload: None,
        payment_metadata: serde_json::json!({}),
        request_no: format!("app-template-payment-{payment_execution_token}"),
        idempotency_key: format!("app-template-payment:{payment_execution_token}"),
    }) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };
    match payments.pay_owner_order(pay_command).await {
        Ok(payment) if provider_qr_code(&payment.payment_params).is_some() => success_created_item(
            ctx,
            map_create_outcome(outcome, &payment_product, Some(payment)),
        ),
        Ok(_) => map_service_error(
            ctx,
            CommerceServiceError::provider_unavailable(format!(
                "payment provider did not return a QR code for {payment_product}"
            )),
        ),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn list_app_template_orders(
    State(state): State<AppTemplateOrderState>,
    runtime_context: Option<axum::extract::Extension<IamAppContext>>,
    request_context: Option<axum::extract::Extension<WebRequestContext>>,
    Query(params): Query<AppTemplateOrderListParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let params = match parse_offset_list_params_validated(ctx, params.page, params.page_size) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let query = match AppTemplateOrderListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        Some(params.page),
        Some(params.page_size),
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };
    match state.store.list_app_template_orders(query).await {
        Ok(page) => {
            let items = page
                .items
                .into_iter()
                .map(map_summary)
                .collect::<Vec<AppTemplateOrderSummaryResponse>>();
            success_items(ctx, items, page.total, params)
        }
        Err(error) => map_service_error(ctx, error),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_create_app_template_order_command(
    subject: &AppRuntimeSubject,
    template_uuid: &str,
    method: &str,
    payment_product: &str,
    request_no: &str,
    idempotency_key: &str,
    client_request_no: Option<&str>,
    source: Option<&str>,
) -> Result<CreateAppTemplateOrderCommand, CommerceServiceError> {
    let requested_at = sdkwork_order_service::canonical_now_timestamp();
    let expire_at =
        sdkwork_order_service::canonical_timestamp_after_seconds(payment_expire_seconds());
    let order_id = Uuid::new_v4().to_string();
    let order_item_id = Uuid::new_v4().to_string();
    let token = stable_hex_token(&format!(
        "{}|{}|{}|{}|{}|{}",
        subject.tenant_id,
        subject.organization_id.as_deref().unwrap_or(""),
        subject.user_id,
        template_uuid,
        request_no,
        idempotency_key,
    ));
    let order_no = format!("AT{token}");
    let out_trade_no = format!("APPTEMPLATE{token}");

    CreateAppTemplateOrderCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        template_uuid,
        method,
        payment_product,
        &order_id,
        &order_item_id,
        &order_no,
        &out_trade_no,
        &requested_at,
        &expire_at,
        idempotency_key,
        client_request_no,
        source,
    )
}

fn map_create_outcome(
    value: CreateAppTemplateOrderOutcome,
    payment_product: &str,
    payment: Option<PayOwnerOrderOutcome>,
) -> CreateAppTemplateOrderResponse {
    let cashier_url = value.cashier_url;
    let (payment_id, payment_params, payment_status) = payment
        .map(|payment| {
            (
                Some(payment.payment_id),
                payment.payment_params,
                Some(payment.status),
            )
        })
        .unwrap_or_else(|| (None, BTreeMap::new(), None));
    let provider_qr_code = provider_qr_code(&payment_params);
    let (qr_code, qr_code_type) = if payment_product == DEFAULT_PAYMENT_PRODUCT {
        (cashier_url.clone(), "cashier_url")
    } else {
        (
            provider_qr_code.cloned().unwrap_or_default(),
            "provider_native",
        )
    };
    CreateAppTemplateOrderResponse {
        amount: value.amount.as_str().to_string(),
        cashier_url,
        currency_code: value.currency_code,
        expires_at: value.expires_at,
        order_id: value.order_id,
        order_no: value.order_no,
        out_trade_no: value.out_trade_no,
        payment_method: value.payment_method,
        payment_product: payment_product.to_string(),
        payment_id,
        payment_params,
        qr_code,
        qr_code_type: qr_code_type.to_string(),
        reused: value.reused,
        status: payment_status.unwrap_or(value.status),
        template_name: value.template_name,
        template_uuid: value.template_uuid,
        version_uuid: value.version_uuid,
    }
}

fn map_summary(item: AppTemplateOrderSummary) -> AppTemplateOrderSummaryResponse {
    AppTemplateOrderSummaryResponse {
        amount: item.amount.as_str().to_string(),
        created_at: item.created_at,
        currency_code: item.currency_code,
        fulfillment_status: item.fulfillment_status,
        order_id: item.order_id,
        order_no: item.order_no,
        paid_at: item.paid_at,
        status: item.status,
        template_name: item.template_name,
        template_uuid: item.template_uuid,
        version_uuid: item.version_uuid,
    }
}

fn validate_template_uuid(value: Option<&str>) -> Result<String, String> {
    let template_uuid = value.unwrap_or_default().trim();
    if template_uuid.is_empty() {
        return Err("templateUuid must be provided".to_string());
    }
    Ok(template_uuid.to_string())
}

fn validate_payment_product(value: Option<&str>) -> Result<String, String> {
    let product = value
        .unwrap_or(DEFAULT_PAYMENT_PRODUCT)
        .trim()
        .to_ascii_lowercase();
    if matches!(
        product.as_str(),
        "mobile_cashier_h5" | "wechat_native" | "alipay_native"
    ) {
        return Ok(product);
    }
    Err(
        "payment product must be one of: mobile_cashier_h5, wechat_native, alipay_native"
            .to_string(),
    )
}

fn validate_payment_method(value: Option<&str>, payment_product: &str) -> Result<String, String> {
    let method = value.unwrap_or_default().trim().to_ascii_lowercase();
    // The H5 cashier resolves its provider after the order exists and a free
    // listing never reaches a provider at all, so an absent method is stored as
    // the cashier sentinel rather than guessed at.
    if method.is_empty() && payment_product == DEFAULT_PAYMENT_PRODUCT {
        return Ok(CASHIER_PAYMENT_METHOD.to_string());
    }
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
    let expected = match payment_product {
        "wechat_native" => Some("wechat_pay"),
        "alipay_native" => Some("alipay"),
        _ => None,
    };
    if expected.is_some_and(|expected| expected != method) {
        return Err(format!(
            "payment product {payment_product} requires payment method {}",
            expected.unwrap_or_default()
        ));
    }
    Ok(method)
}

fn organization_scope(organization_id: Option<&str>) -> String {
    organization_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(PLATFORM_ORGANIZATION_SCOPE_SENTINEL)
        .to_owned()
}

fn fallback_request_no(
    subject: &AppRuntimeSubject,
    template_uuid: &str,
    method: &str,
    idempotency_key: &str,
) -> String {
    stable_header_token(&format!(
        "app-template-order-{}-{}-{}-{}",
        subject.user_id, template_uuid, method, idempotency_key
    ))
}

/// The subject literal this family writes, exposed for the route-manifest test
/// so a rename cannot silently orphan the settlement handler.
pub const APP_TEMPLATE_ORDER_ROUTE_SUBJECT: &str = APP_TEMPLATE_ORDER_SUBJECT;

#[cfg(test)]
mod tests {
    use super::{
        validate_payment_method, validate_payment_product, validate_template_uuid,
        DEFAULT_PAYMENT_PRODUCT,
    };

    #[test]
    fn template_uuid_is_required() {
        assert!(validate_template_uuid(None).is_err());
        assert!(validate_template_uuid(Some("  ")).is_err());
        assert_eq!(
            validate_template_uuid(Some(" template-1 ")).expect("uuid"),
            "template-1"
        );
    }

    #[test]
    fn payment_product_defaults_to_the_cashier() {
        assert_eq!(
            validate_payment_product(None).expect("product"),
            DEFAULT_PAYMENT_PRODUCT
        );
        assert!(validate_payment_product(Some("unknown_native")).is_err());
    }

    #[test]
    fn native_products_require_their_matching_method() {
        assert!(validate_payment_method(Some("alipay"), "wechat_native").is_err());
        assert_eq!(
            validate_payment_method(Some("wechat_pay"), "wechat_native").expect("method"),
            "wechat_pay"
        );
        assert!(validate_payment_method(Some("card"), DEFAULT_PAYMENT_PRODUCT).is_err());
        assert_eq!(
            validate_payment_method(None, DEFAULT_PAYMENT_PRODUCT).expect("cashier sentinel"),
            "cashier"
        );
    }
}
