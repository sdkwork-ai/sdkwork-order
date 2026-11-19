//! Buyer cart app-api routes.
//!
//! Contract: `GET /app/v3/api/cart/current`, `POST /app/v3/api/cart/items`,
//! `PATCH /app/v3/api/cart/items/{cartItemId}`, `DELETE /app/v3/api/cart/items/{cartItemId}`.
//! These are exactly the operations the published commerce app contract declares
//! (`cart.current.retrieve`, `cart.items.create|update|delete`) and the ones every
//! storefront client calls, so the operation ids stay identical.
//!
//! `POST` accepts an optional `Idempotency-Key`: the SDK the storefront ships today
//! does not send one, so requiring it would break the live client, but a client that
//! does send it gets replay safety instead of a double increment on retry.

use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_repository_sqlx::PostgresCartStore;
use sdkwork_order_service::{
    CartItemMutation, CartLineView, CartView, CommerceCartStore, CreateCartItemCommand,
    CurrentCartQuery, DeleteCartItemCommand, UpdateCartItemCommand,
};
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::api_response::{map_service_error, success_created_item, success_item, unauthorized};
use crate::command_headers::IDEMPOTENCY_KEY_HEADER;
use crate::subject::app_runtime_subject_from_contexts;

#[derive(Clone)]
struct AppCartState {
    store: Arc<dyn CommerceCartStore>,
}

/// Wire body of `POST /app/v3/api/cart/items`.
///
/// `spuId` / `shopId` are not accepted: both are attribution, and the server
/// derives the product id from the SKU row instead of trusting the caller.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateCartItemRequest {
    sku_id: String,
    quantity: Option<i64>,
}

/// Wire body of `PATCH /app/v3/api/cart/items/{cartItemId}`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateCartItemRequest {
    quantity: Option<i64>,
    selected: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CartLineResponse {
    id: String,
    sku_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    spu_id: Option<String>,
    quantity: i32,
    selected: bool,
    available: bool,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sku_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<String>,
    unit_price: String,
    price_scale: i16,
    currency_code: String,
    line_total: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_unit_price: Option<String>,
    price_changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    available_stock: Option<i32>,
    stock_shortage: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CartResponse {
    id: String,
    status: String,
    currency_code: String,
    items: Vec<CartLineResponse>,
    item_count: i32,
    total_quantity: i32,
    selected_quantity: i32,
    total_amount: String,
    has_unavailable_selection: bool,
    has_stock_shortage: bool,
}

pub fn app_cart_router_with_postgres_pool(pool: PgPool) -> Router {
    build_app_cart_router(Arc::new(PostgresCartStore::new(pool)))
}

pub fn build_app_cart_router(store: Arc<dyn CommerceCartStore>) -> Router {
    Router::new()
        .route("/app/v3/api/cart/current", get(retrieve_current_cart))
        .route("/app/v3/api/cart/items", post(create_cart_item))
        .route(
            "/app/v3/api/cart/items/{cartItemId}",
            patch(update_cart_item).delete(delete_cart_item),
        )
        .with_state(AppCartState { store })
}

async fn retrieve_current_cart(
    State(state): State<AppCartState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match CurrentCartQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.retrieve_current_cart(query).await {
        Ok(cart) => match map_cart(cart) {
            Ok(cart) => success_item(ctx, cart),
            Err(error) => map_service_error(ctx, error),
        },
        Err(error) => map_service_error(ctx, error),
    }
}

async fn create_cart_item(
    State(state): State<AppCartState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(body): Json<CreateCartItemRequest>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let idempotency_key = optional_idempotency_key(&headers);
    let command = match CreateCartItemCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &body.sku_id,
        body.quantity.unwrap_or(1),
        idempotency_key.as_deref(),
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.create_cart_item(command).await {
        Ok(mutation) => match map_mutation(mutation) {
            Ok((line, replayed)) => {
                if replayed {
                    success_item(ctx, line)
                } else {
                    success_created_item(ctx, line)
                }
            }
            Err(error) => map_service_error(ctx, error),
        },
        Err(error) => map_service_error(ctx, error),
    }
}

async fn update_cart_item(
    State(state): State<AppCartState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(cart_item_id): Path<String>,
    Json(body): Json<UpdateCartItemRequest>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let command = match UpdateCartItemCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &cart_item_id,
        body.quantity,
        body.selected,
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.update_cart_item(command).await {
        Ok(mutation) => match map_mutation(mutation) {
            Ok((line, _)) => success_item(ctx, line),
            Err(error) => map_service_error(ctx, error),
        },
        Err(error) => map_service_error(ctx, error),
    }
}

async fn delete_cart_item(
    State(state): State<AppCartState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(cart_item_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let command = match DeleteCartItemCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &cart_item_id,
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.delete_cart_item(command).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => map_service_error(ctx, error),
    }
}

fn optional_idempotency_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn map_mutation(
    mutation: CartItemMutation,
) -> Result<(CartLineResponse, bool), CommerceServiceError> {
    Ok((map_cart_line(mutation.line)?, mutation.replayed))
}

fn map_cart(cart: CartView) -> Result<CartResponse, CommerceServiceError> {
    let item_count = cart.items.len();
    let total_quantity = cart.total_quantity();
    let selected_quantity = cart.selected_quantity();
    let total_amount = cart.selected_subtotal()?;
    let has_unavailable_selection = cart.has_unavailable_selection();
    let has_stock_shortage = cart.has_stock_shortage();
    let items = cart
        .items
        .into_iter()
        .map(map_cart_line)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CartResponse {
        id: cart.cart_id,
        status: cart.status,
        currency_code: cart.currency_code,
        items,
        item_count: quantity_to_wire(item_count as i64)?,
        total_quantity: quantity_to_wire(total_quantity)?,
        selected_quantity: quantity_to_wire(selected_quantity)?,
        total_amount: total_amount.as_str().to_owned(),
        has_unavailable_selection,
        has_stock_shortage,
    })
}

fn map_cart_line(line: CartLineView) -> Result<CartLineResponse, CommerceServiceError> {
    let line_total = line.line_total()?;
    Ok(CartLineResponse {
        id: line.cart_item_id,
        sku_id: line.sku_id,
        spu_id: line.spu_id,
        quantity: quantity_to_wire(line.quantity)?,
        selected: line.selected,
        available: line.available,
        title: line.title,
        sku_name: line.sku_name,
        image_url: line.image_url,
        unit_price: line.unit_price.as_str().to_owned(),
        price_scale: line.price_scale,
        currency_code: line.currency_code,
        line_total: line_total.as_str().to_owned(),
        current_unit_price: line
            .current_unit_price
            .map(|money| money.as_str().to_owned()),
        price_changed: line.price_changed,
        available_stock: line
            .available_stock
            .map(|stock| quantity_to_wire(stock))
            .transpose()?,
        stock_shortage: line.stock_shortage,
    })
}

/// Quantity is a small count, so it travels as `int32`; the int64 wire contract
/// applies to identifiers and amounts, which stay strings.
fn quantity_to_wire(quantity: i64) -> Result<i32, CommerceServiceError> {
    i32::try_from(quantity)
        .map_err(|_| CommerceServiceError::validation("cart quantity exceeds the wire range"))
}
