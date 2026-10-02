use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_repository_sqlx::PostgresCommerceOrderStore;
use sdkwork_order_service::{
    FulfillmentDetailQuery, FulfillmentListPage, FulfillmentListQuery, FulfillmentView,
};
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::api_response::{
    map_service_error, not_found, offset_list_page_params_from_query, success_item, success_items,
    unauthorized, validation,
};
use crate::subject::app_runtime_subject_from_contexts;

pub type CommerceFulfillmentFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommerceFulfillmentStore: Send + Sync {
    fn list_owner_fulfillments<'a>(
        &'a self,
        query: FulfillmentListQuery,
    ) -> CommerceFulfillmentFuture<'a, FulfillmentListPage>;

    fn retrieve_owner_fulfillment<'a>(
        &'a self,
        query: FulfillmentDetailQuery,
    ) -> CommerceFulfillmentFuture<'a, Option<FulfillmentView>>;
}

#[derive(Clone)]
struct AppFulfillmentState {
    store: Arc<dyn CommerceFulfillmentStore>,
}

#[derive(Debug, Deserialize)]
struct FulfillmentListParams {
    order_id: Option<String>,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FulfillmentResponse {
    fulfillment_id: String,
    fulfillment_no: String,
    order_id: String,
    fulfillment_type: String,
    status: String,
}

impl CommerceFulfillmentStore for PostgresCommerceOrderStore {
    fn list_owner_fulfillments<'a>(
        &'a self,
        query: FulfillmentListQuery,
    ) -> CommerceFulfillmentFuture<'a, FulfillmentListPage> {
        Box::pin(async move { self.list_owner_fulfillments(query).await })
    }

    fn retrieve_owner_fulfillment<'a>(
        &'a self,
        query: FulfillmentDetailQuery,
    ) -> CommerceFulfillmentFuture<'a, Option<FulfillmentView>> {
        Box::pin(async move { self.retrieve_owner_fulfillment(query).await })
    }
}

pub fn app_fulfillment_router_with_postgres_pool(pool: PgPool) -> Router {
    build_app_fulfillment_router(Arc::new(PostgresCommerceOrderStore::new(pool)))
}

pub fn build_app_fulfillment_router(store: Arc<dyn CommerceFulfillmentStore>) -> Router {
    Router::new()
        .route("/app/v3/api/fulfillments", get(list_fulfillments))
        .route(
            "/app/v3/api/fulfillments/{fulfillmentId}",
            get(retrieve_fulfillment),
        )
        .with_state(AppFulfillmentState { store })
}

async fn list_fulfillments(
    State(state): State<AppFulfillmentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Query(params): Query<FulfillmentListParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match FulfillmentListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        params.order_id.as_deref(),
        params.status.as_deref(),
        params.page,
        params.page_size,
    ) {
        Ok(query) => query,
        Err(error) => return validation(ctx, error.message()),
    };

    match state.store.list_owner_fulfillments(query).await {
        Ok(page) => {
            let page_params = offset_list_page_params_from_query(page.page, page.page_size);
            let mapped = page
                .items
                .into_iter()
                .map(map_fulfillment)
                .collect::<Vec<_>>();
            success_items(ctx, mapped, page.total, page_params)
        }
        Err(error) => map_service_error(ctx, error),
    }
}

async fn retrieve_fulfillment(
    State(state): State<AppFulfillmentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(fulfillment_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match FulfillmentDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &fulfillment_id,
    ) {
        Ok(query) => query,
        Err(error) => return validation(ctx, error.message()),
    };

    match state.store.retrieve_owner_fulfillment(query).await {
        Ok(Some(fulfillment)) => success_item(ctx, map_fulfillment(fulfillment)),
        Ok(None) => not_found(ctx, "fulfillment was not found"),
        Err(error) => map_service_error(ctx, error),
    }
}

fn map_fulfillment(value: FulfillmentView) -> FulfillmentResponse {
    FulfillmentResponse {
        fulfillment_id: value.fulfillment_id,
        fulfillment_no: value.fulfillment_no,
        order_id: value.order_id,
        fulfillment_type: value.fulfillment_type,
        status: value.status,
    }
}
