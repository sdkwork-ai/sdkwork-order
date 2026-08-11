use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_repository_sqlx::PostgresCommerceOrderStore;
use sdkwork_order_service::{
    physical_inventory_reserve_idempotency_key, CheckoutLineInput, CheckoutQuoteView,
    CheckoutSessionDetailQuery, CheckoutSessionView, CreateCheckoutQuoteCommand,
    CreateCheckoutSessionCommand, CreateOwnerOrderCommand, CreateOwnerOrderOutcome,
    PhysicalCheckoutResolverPort, PhysicalInventoryReservationPort,
    ReservePhysicalOrderInventoryRequest, ResolvePhysicalCheckoutLine,
    ResolvePhysicalCheckoutRequest, ShippingAddressSnapshot,
    UnavailablePhysicalCheckoutResolverPort, UnavailablePhysicalInventoryReservationPort,
};
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::api_response::{
    map_service_error, not_found, success_created_item, success_item, unauthorized, validation,
};
use crate::command_headers::required_app_write_command_headers;
use crate::subject::{app_runtime_subject_from_contexts, AppRuntimeSubject};

pub type CommerceCheckoutFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommerceCheckoutStore: Send + Sync {
    fn create_checkout_session<'a>(
        &'a self,
        command: CreateCheckoutSessionCommand,
    ) -> CommerceCheckoutFuture<'a, CheckoutSessionView>;

    fn retrieve_checkout_session<'a>(
        &'a self,
        query: CheckoutSessionDetailQuery,
    ) -> CommerceCheckoutFuture<'a, Option<CheckoutSessionView>>;

    fn create_checkout_quote<'a>(
        &'a self,
        command: CreateCheckoutQuoteCommand,
    ) -> CommerceCheckoutFuture<'a, CheckoutQuoteView>;

    fn create_owner_order<'a>(
        &'a self,
        command: CreateOwnerOrderCommand,
    ) -> CommerceCheckoutFuture<'a, CreateOwnerOrderOutcome>;

    fn mark_owner_order_inventory_reserved<'a>(
        &'a self,
        tenant_id: &'a str,
        owner_user_id: &'a str,
        order_id: &'a str,
    ) -> CommerceCheckoutFuture<'a, ()>;

    fn mark_owner_order_inventory_failed<'a>(
        &'a self,
        tenant_id: &'a str,
        owner_user_id: &'a str,
        order_id: &'a str,
    ) -> CommerceCheckoutFuture<'a, ()>;
}

#[derive(Clone)]
struct AppCheckoutState {
    store: Arc<dyn CommerceCheckoutStore>,
    checkout_resolver: Arc<dyn PhysicalCheckoutResolverPort>,
    inventory: Arc<dyn PhysicalInventoryReservationPort>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckoutLineRequest {
    sku_id: String,
    quantity: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCheckoutSessionRequest {
    items: Vec<CheckoutLineRequest>,
    currency_code: Option<String>,
    shipping_address: Option<ShippingAddressRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShippingAddressRequest {
    receiver_name: String,
    receiver_phone: String,
    country_code: String,
    province: String,
    city: String,
    district: Option<String>,
    detail_address: String,
    postal_code: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckoutSessionResponse {
    checkout_session_id: String,
    status: String,
    currency_code: String,
    original_amount: String,
    discount_amount: String,
    payable_amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    quote_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckoutQuoteResponse {
    checkout_session_id: String,
    quote_id: String,
    currency_code: String,
    original_amount: String,
    discount_amount: String,
    payable_amount: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckoutOrderResponse {
    order_id: String,
    order_no: String,
    order_sn: String,
    status: String,
    total_amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    partner_id: Option<String>,
}

impl CommerceCheckoutStore for PostgresCommerceOrderStore {
    fn create_checkout_session<'a>(
        &'a self,
        command: CreateCheckoutSessionCommand,
    ) -> CommerceCheckoutFuture<'a, CheckoutSessionView> {
        Box::pin(async move { self.create_checkout_session(command).await })
    }

    fn retrieve_checkout_session<'a>(
        &'a self,
        query: CheckoutSessionDetailQuery,
    ) -> CommerceCheckoutFuture<'a, Option<CheckoutSessionView>> {
        Box::pin(async move { self.retrieve_checkout_session(query).await })
    }

    fn create_checkout_quote<'a>(
        &'a self,
        command: CreateCheckoutQuoteCommand,
    ) -> CommerceCheckoutFuture<'a, CheckoutQuoteView> {
        Box::pin(async move { self.create_checkout_quote(command).await })
    }

    fn create_owner_order<'a>(
        &'a self,
        command: CreateOwnerOrderCommand,
    ) -> CommerceCheckoutFuture<'a, CreateOwnerOrderOutcome> {
        Box::pin(async move { self.create_owner_order(command).await })
    }

    fn mark_owner_order_inventory_reserved<'a>(
        &'a self,
        tenant_id: &'a str,
        owner_user_id: &'a str,
        order_id: &'a str,
    ) -> CommerceCheckoutFuture<'a, ()> {
        Box::pin(async move {
            self.mark_owner_order_inventory_reserved(tenant_id, owner_user_id, order_id)
                .await
        })
    }

    fn mark_owner_order_inventory_failed<'a>(
        &'a self,
        tenant_id: &'a str,
        owner_user_id: &'a str,
        order_id: &'a str,
    ) -> CommerceCheckoutFuture<'a, ()> {
        Box::pin(async move {
            self.mark_owner_order_inventory_failed(tenant_id, owner_user_id, order_id)
                .await
        })
    }
}

pub fn app_checkout_router_with_postgres_pool(pool: PgPool) -> Router {
    build_app_checkout_router_with_integrations(
        Arc::new(PostgresCommerceOrderStore::new(pool)),
        Arc::new(UnavailablePhysicalCheckoutResolverPort),
        Arc::new(UnavailablePhysicalInventoryReservationPort),
    )
}

pub fn build_app_checkout_router(store: Arc<dyn CommerceCheckoutStore>) -> Router {
    build_app_checkout_router_with_integrations(
        store,
        Arc::new(UnavailablePhysicalCheckoutResolverPort),
        Arc::new(UnavailablePhysicalInventoryReservationPort),
    )
}

pub fn build_app_checkout_router_with_integrations(
    store: Arc<dyn CommerceCheckoutStore>,
    checkout_resolver: Arc<dyn PhysicalCheckoutResolverPort>,
    inventory: Arc<dyn PhysicalInventoryReservationPort>,
) -> Router {
    Router::new()
        .route(
            "/app/v3/api/checkout/sessions",
            post(create_checkout_session),
        )
        .route(
            "/app/v3/api/checkout/sessions/{checkoutSessionId}",
            get(retrieve_checkout_session),
        )
        .route(
            "/app/v3/api/checkout/sessions/{checkoutSessionId}/quotes",
            post(create_checkout_quote),
        )
        .route(
            "/app/v3/api/checkout/sessions/{checkoutSessionId}/orders",
            post(create_checkout_order),
        )
        .with_state(AppCheckoutState {
            store,
            checkout_resolver,
            inventory,
        })
}

async fn create_checkout_session(
    State(state): State<AppCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    body: Json<CreateCheckoutSessionRequest>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let write_headers = match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
        fallback_request_no(&subject, "checkout-session", idempotency_key)
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let request = body.0;
    let lines = match parse_checkout_lines(&request) {
        Ok(lines) => lines,
        Err(message) => return validation(ctx, message),
    };
    let currency_code = request
        .currency_code
        .as_deref()
        .unwrap_or("CNY")
        .trim()
        .to_ascii_uppercase();
    let resolved_checkout = if let Some(shipping_address) = request.shipping_address {
        let shipping_address = match ShippingAddressSnapshot::new(
            &shipping_address.receiver_name,
            &shipping_address.receiver_phone,
            &shipping_address.country_code,
            &shipping_address.province,
            &shipping_address.city,
            shipping_address.district.as_deref(),
            &shipping_address.detail_address,
            shipping_address.postal_code.as_deref(),
        ) {
            Ok(value) => value,
            Err(error) => return validation(ctx, error.message()),
        };
        match state
            .checkout_resolver
            .resolve_physical_checkout(ResolvePhysicalCheckoutRequest {
                tenant_id: subject.tenant_id.clone(),
                owner_user_id: subject.user_id.clone(),
                currency_code: currency_code.clone(),
                lines: lines
                    .iter()
                    .map(|line| ResolvePhysicalCheckoutLine {
                        sku_id: line.sku_id.clone(),
                        quantity: line.quantity,
                    })
                    .collect(),
                shipping_address,
            })
            .await
        {
            Ok(value) => Some(value),
            Err(error) => return map_service_error(ctx, error),
        }
    } else {
        None
    };
    let mut command = match CreateCheckoutSessionCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &currency_code,
        lines,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return validation(ctx, error.message()),
    };
    if let Some(resolved_checkout) = resolved_checkout {
        command = match command.with_physical_checkout(resolved_checkout) {
            Ok(command) => command,
            Err(error) => return validation(ctx, error.message()),
        };
    }
    match state.store.create_checkout_session(command).await {
        Ok(session) => success_created_item(ctx, map_checkout_session(session)),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn retrieve_checkout_session(
    State(state): State<AppCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(checkout_session_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match CheckoutSessionDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &checkout_session_id,
    ) {
        Ok(query) => query,
        Err(error) => return validation(ctx, error.message()),
    };

    match state.store.retrieve_checkout_session(query).await {
        Ok(Some(session)) => success_item(ctx, map_checkout_session(session)),
        Ok(None) => not_found(ctx, "checkout session was not found"),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn create_checkout_quote(
    State(state): State<AppCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(checkout_session_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let write_headers = match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
        fallback_request_no(&subject, &checkout_session_id, idempotency_key)
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let command = match CreateCheckoutQuoteCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &checkout_session_id,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return validation(ctx, error.message()),
    };
    match state.store.create_checkout_quote(command).await {
        Ok(quote) => success_created_item(ctx, map_checkout_quote(quote)),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn create_checkout_order(
    State(state): State<AppCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(checkout_session_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let write_headers = match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
        fallback_request_no(&subject, &checkout_session_id, idempotency_key)
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let command = match CreateOwnerOrderCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &checkout_session_id,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return validation(ctx, error.message()),
    };
    match state.store.create_owner_order(command).await {
        Ok(mut outcome) => {
            let merchant_organization_id = match outcome.merchant_organization_id.as_deref() {
                Some(value) if !value.trim().is_empty() => value.to_owned(),
                _ => {
                    let _ = state
                        .store
                        .mark_owner_order_inventory_failed(
                            &subject.tenant_id,
                            &subject.user_id,
                            &outcome.order_id,
                        )
                        .await;
                    return validation(ctx, "physical order merchant organization is missing");
                }
            };
            if outcome.inventory_lines.is_empty() {
                let _ = state
                    .store
                    .mark_owner_order_inventory_failed(
                        &subject.tenant_id,
                        &subject.user_id,
                        &outcome.order_id,
                    )
                    .await;
                return validation(ctx, "physical order inventory lines are missing");
            }
            let reserve_result = state
                .inventory
                .reserve_physical_order_inventory(ReservePhysicalOrderInventoryRequest {
                    tenant_id: subject.tenant_id.clone(),
                    merchant_organization_id,
                    order_id: outcome.order_id.clone(),
                    request_no: write_headers.request_no.clone(),
                    idempotency_key: physical_inventory_reserve_idempotency_key(&outcome.order_id),
                    lines: outcome.inventory_lines.clone(),
                })
                .await;
            if let Err(error) = reserve_result {
                let close_result = state
                    .store
                    .mark_owner_order_inventory_failed(
                        &subject.tenant_id,
                        &subject.user_id,
                        &outcome.order_id,
                    )
                    .await;
                if let Err(close_error) = close_result {
                    return map_service_error(ctx, close_error);
                }
                return map_service_error(ctx, error);
            }
            if let Err(error) = state
                .store
                .mark_owner_order_inventory_reserved(
                    &subject.tenant_id,
                    &subject.user_id,
                    &outcome.order_id,
                )
                .await
            {
                return map_service_error(ctx, error);
            }
            outcome.status = "pending_payment".to_owned();
            success_created_item(ctx, map_checkout_order(outcome))
        }
        Err(error) => map_service_error(ctx, error),
    }
}

fn parse_checkout_lines(
    request: &CreateCheckoutSessionRequest,
) -> Result<Vec<CheckoutLineInput>, String> {
    if request.items.is_empty() {
        return Err("checkout session requires at least one line".to_owned());
    }
    request
        .items
        .iter()
        .map(|line| {
            CheckoutLineInput::new(&line.sku_id, line.quantity.unwrap_or(1).max(1))
                .map_err(|error| error.message().to_owned())
        })
        .collect()
}

fn map_checkout_session(value: CheckoutSessionView) -> CheckoutSessionResponse {
    CheckoutSessionResponse {
        checkout_session_id: value.checkout_session_id,
        status: value.status,
        currency_code: value.currency_code,
        original_amount: value.original_amount.as_str().to_owned(),
        discount_amount: value.discount_amount.as_str().to_owned(),
        payable_amount: value.payable_amount.as_str().to_owned(),
        quote_id: value.quote_id,
        expires_at: value.expires_at,
    }
}

fn map_checkout_quote(value: CheckoutQuoteView) -> CheckoutQuoteResponse {
    CheckoutQuoteResponse {
        checkout_session_id: value.checkout_session_id,
        quote_id: value.quote_id,
        currency_code: value.currency_code,
        original_amount: value.original_amount.as_str().to_owned(),
        discount_amount: value.discount_amount.as_str().to_owned(),
        payable_amount: value.payable_amount.as_str().to_owned(),
    }
}

fn map_checkout_order(value: CreateOwnerOrderOutcome) -> CheckoutOrderResponse {
    CheckoutOrderResponse {
        order_id: value.order_id.clone(),
        order_no: value.order_sn.clone(),
        order_sn: value.order_sn,
        status: value.status,
        total_amount: value.total_amount.as_str().to_owned(),
        expires_at: value.expires_at,
        partner_id: value.partner_snapshot.map(|partner| partner.partner_id),
    }
}

fn fallback_request_no(subject: &AppRuntimeSubject, suffix: &str, idempotency_key: &str) -> String {
    format!(
        "checkout-{}-{}-{}",
        subject.user_id, suffix, idempotency_key
    )
}
