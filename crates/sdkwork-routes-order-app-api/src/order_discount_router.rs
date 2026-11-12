//! App order discount application: promotion-owned discount + order-owned
//! repricing, in that order. A repricing failure compensates the promotion
//! application by releasing it, so a failed request cannot leave a reserved
//! budget with no order discount.

use axum::extract::{Extension, Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_repository_sqlx::PostgresCommerceOrderStore;
use sdkwork_order_service::{
    ApplyOrderDiscountCommand, OrderDiscountView, PromotionDiscountPort, PromotionDiscountRequest,
    PromotionDiscountSettlementRequest,
};
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::api_response::{
    map_service_error, success_created_item, unauthorized, validation,
};
use crate::command_headers::required_app_write_command_headers;
use crate::subject::app_runtime_subject_from_contexts;

pub type OrderDiscountFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommerceOrderDiscountStore: Send + Sync {
    fn apply_order_discount<'a>(
        &'a self,
        command: ApplyOrderDiscountCommand,
    ) -> OrderDiscountFuture<'a, OrderDiscountView>;
}

impl CommerceOrderDiscountStore for PostgresCommerceOrderStore {
    fn apply_order_discount<'a>(
        &'a self,
        command: ApplyOrderDiscountCommand,
    ) -> OrderDiscountFuture<'a, OrderDiscountView> {
        Box::pin(async move { self.apply_order_discount(command).await })
    }
}

#[derive(Clone)]
struct AppOrderDiscountState {
    store: Arc<dyn CommerceOrderDiscountStore>,
    promotion: Arc<dyn PromotionDiscountPort>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyOrderDiscountRequest {
    user_coupon_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderDiscountResponse {
    order_id: String,
    original_amount: String,
    discount_amount: String,
    payable_amount: String,
    currency_code: String,
}

impl From<OrderDiscountView> for OrderDiscountResponse {
    fn from(value: OrderDiscountView) -> Self {
        Self {
            order_id: value.order_id,
            original_amount: value.original_amount_minor,
            discount_amount: value.discount_amount_minor,
            payable_amount: value.payable_amount_minor,
            currency_code: value.currency_code,
        }
    }
}

pub fn build_app_order_discount_router(
    store: Arc<dyn CommerceOrderDiscountStore>,
    promotion: Arc<dyn PromotionDiscountPort>,
) -> Router {
    Router::new()
        .route(
            "/app/v3/api/orders/{orderId}/discount",
            post(apply_order_discount),
        )
        .with_state(AppOrderDiscountState { store, promotion })
}

pub fn app_order_discount_router_with_postgres_pool(
    pool: &PgPool,
    promotion: Arc<dyn PromotionDiscountPort>,
) -> Router {
    build_app_order_discount_router(
        Arc::new(PostgresCommerceOrderStore::new(pool.clone())),
        promotion,
    )
}

async fn apply_order_discount(
    State(state): State<AppOrderDiscountState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    Json(body): Json<ApplyOrderDiscountRequest>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let user_coupon_id = body.user_coupon_id.trim().to_owned();
    if user_coupon_id.is_empty() {
        return validation(ctx, "userCouponId must not be empty");
    }
    let write_headers = match required_app_write_command_headers(ctx, &headers, |_| {
        format!("order-discount-{order_id}-{user_coupon_id}")
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let request = PromotionDiscountRequest {
        tenant_id: subject.tenant_id.clone(),
        organization_id: subject.organization_id.clone(),
        owner_user_id: subject.user_id.clone(),
        order_id: order_id.clone(),
        user_coupon_id: user_coupon_id.clone(),
        request_no: write_headers.request_no.clone(),
        idempotency_key: write_headers.idempotency_key.clone(),
    };
    let outcome = match state.promotion.apply_discount(request).await {
        Ok(outcome) => outcome,
        Err(error) => return map_service_error(ctx, error),
    };
    let command = match ApplyOrderDiscountCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_id,
        outcome.discount_amount_minor,
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };
    match state.store.apply_order_discount(command).await {
        Ok(view) => success_created_item(ctx, OrderDiscountResponse::from(view)),
        Err(error) => {
            // The promotion application is reserved; leaving it behind would
            // charge the budget for an order that was not repriced.
            let _ = state
                .promotion
                .release_discounts(PromotionDiscountSettlementRequest {
                    tenant_id: subject.tenant_id,
                    organization_id: subject.organization_id,
                    order_id,
                })
                .await;
            map_service_error(ctx, error)
        }
    }
}
