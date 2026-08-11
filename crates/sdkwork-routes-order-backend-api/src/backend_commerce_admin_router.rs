//! Backend admin routes for account value, after-sales, and shipment operations.

use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_repository_sqlx::{
    PostgresCommerceOrderStore, PostgresCommerceRechargeStore,
};
use sdkwork_order_service::{
    execute_account_value_request_review, physical_inventory_release_idempotency_key,
    AccountValueAssetCode, AccountValueCatalogListQuery, AccountValueLedgerPort,
    AccountValueOrderSubject, AccountValuePackageItem, AccountValuePackageListPage,
    AccountValueRequestListPage, AccountValueRequestListQuery, AccountValueRequestReviewAction,
    AccountValueRequestView, AfterSalesManagementDetailQuery, AfterSalesManagementListQuery,
    AfterSalesRequestView, CreateShipmentPackageCommand, NoopAccountValueLedgerPort,
    NoopPaymentPayoutExecutorPort, NoopPaymentRefundExecutorPort, OrderOwnerDetailQuery,
    PaymentPayoutExecutorPort, PaymentRefundExecutorPort, PhysicalInventoryReservationPort,
    ReleasePhysicalOrderInventoryRequest, RetireAccountValuePackageCommand,
    RetireTokenBankPlanCommand, ReviewAccountValueRequestCommand, ReviewAfterSalesRequestCommand,
    ShipmentManagementDetailQuery, ShipmentManagementListQuery, ShipmentPackageManagementListQuery,
    ShipmentPackageView, ShipmentView, TokenBankPlanItem, TokenBankPlanListPage,
    TokenBankPlanPeriod, UnavailablePhysicalInventoryReservationPort, UpdateShipmentPackageCommand,
    UpsertAccountValuePackageCommand, UpsertTokenBankPlanCommand,
};
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::api_response::{
    map_service_error, parse_offset_list_params_validated, success_command, success_created_item,
    success_item, success_items,
};
use crate::backend_acl::{require_backend_operator, BackendOperatorScope};
use crate::backend_command_headers::resolve_required_backend_write_command_headers;

mod permissions {
    pub const ACCOUNT_VALUE_MANAGE: &str = "commerce.accountValue.manage";
    pub const ACCOUNT_VALUE_READ: &str = "commerce.accountValue.read";
    pub const ACCOUNT_VALUE_REVIEW: &str = "commerce.accountValue.review";
    pub const AFTER_SALES_READ: &str = "commerce.afterSales.read";
    pub const AFTER_SALES_REVIEW: &str = "commerce.afterSales.review";
    pub const ORDERS_READ: &str = "commerce.orders.read";
    pub const ORDERS_MANAGE: &str = "commerce.orders.manage";
}

#[derive(Clone)]
enum BackendCommerceAdminStore {
    Postgres {
        orders: Arc<PostgresCommerceOrderStore>,
        recharge: Arc<PostgresCommerceRechargeStore>,
    },
}

#[derive(Clone)]
struct BackendCommerceAdminState {
    store: BackendCommerceAdminStore,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    payment_refund_executor_port: Arc<dyn PaymentRefundExecutorPort>,
    payment_payout_executor_port: Arc<dyn PaymentPayoutExecutorPort>,
    inventory: Arc<dyn PhysicalInventoryReservationPort>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesListParams {
    status: Option<String>,
    #[serde(rename = "afterSalesType", alias = "after_sales_type")]
    after_sales_type: Option<String>,
    #[serde(rename = "orderId", alias = "order_id")]
    order_id: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ShipmentListParams {
    status: Option<String>,
    #[serde(rename = "orderId", alias = "order_id")]
    order_id: Option<String>,
    #[serde(rename = "fulfillmentId", alias = "fulfillment_id")]
    fulfillment_id: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PackageListParams {
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AccountValueCatalogListParams {
    target_asset: Option<String>,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AccountValueRequestListParams {
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewAfterSalesRequestBody {
    review_action: String,
    status: Option<String>,
    refund_status: Option<String>,
    return_status: Option<String>,
    exchange_status: Option<String>,
    approved_amount: Option<String>,
    reason_code: Option<String>,
    #[serde(alias = "reasonDetail")]
    reason_detail: Option<String>,
    review_comment: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateShipmentPackageBody {
    package_type: String,
    package_no: Option<String>,
    tracking_no: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateShipmentPackageBody {
    package_type: Option<String>,
    tracking_no: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct AccountValuePackageWriteBody {
    package_code: Option<String>,
    display_name: Option<String>,
    target_asset: Option<String>,
    grant_amount: Option<String>,
    bonus_amount: Option<String>,
    price_amount: Option<String>,
    currency_code: Option<String>,
    status: Option<String>,
    sort_weight: Option<i64>,
    valid_from: Option<String>,
    valid_to: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct TokenBankPlanWriteBody {
    plan_code: Option<String>,
    display_name: Option<String>,
    plan_period: Option<String>,
    grant_amount: Option<String>,
    bonus_amount: Option<String>,
    price_amount: Option<String>,
    currency_code: Option<String>,
    renewal_policy: Option<String>,
    status: Option<String>,
    sort_weight: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct AccountValueRequestReviewBody {
    reason_code: Option<String>,
    review_comment: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AfterSalesRequestResponse {
    after_sales_request_id: String,
    after_sales_no: String,
    order_id: String,
    after_sales_type: String,
    reason_code: String,
    requested_amount: String,
    currency_code: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountValuePackageResponse {
    package_id: String,
    package_code: String,
    display_name: String,
    target_asset: String,
    grant_amount: String,
    bonus_amount: String,
    price_amount: String,
    currency_code: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenBankPlanResponse {
    plan_code: String,
    display_name: String,
    plan_period: String,
    grant_amount: String,
    bonus_amount: String,
    price_amount: String,
    currency_code: String,
    renewal_policy: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountValueRequestResponse {
    account_value_request_id: String,
    request_no: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_order_id: Option<String>,
    owner_user_id: String,
    subject: String,
    target_asset: String,
    amount: String,
    currency_code: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_reference_id: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShipmentResponse {
    shipment_id: String,
    shipment_no: String,
    fulfillment_id: String,
    carrier_code: String,
    tracking_no: Option<String>,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShipmentPackageResponse {
    package_id: String,
    shipment_id: String,
    package_no: String,
    package_type: String,
    tracking_no: Option<String>,
    status: String,
}

impl BackendCommerceAdminStore {
    async fn list_management_after_sales(
        &self,
        query: AfterSalesManagementListQuery,
    ) -> Result<sdkwork_order_service::AfterSalesRequestPage, CommerceServiceError> {
        match self {
            Self::Postgres { orders, .. } => {
                orders.list_management_after_sales_requests(query).await
            }
        }
    }

    async fn retrieve_management_after_sales(
        &self,
        query: AfterSalesManagementDetailQuery,
    ) -> Result<Option<AfterSalesRequestView>, CommerceServiceError> {
        match self {
            Self::Postgres { orders, .. } => {
                orders.retrieve_management_after_sales_request(query).await
            }
        }
    }

    async fn review_after_sales(
        &self,
        command: ReviewAfterSalesRequestCommand,
    ) -> Result<AfterSalesRequestView, CommerceServiceError> {
        match self {
            Self::Postgres { orders, .. } => orders.review_after_sales_request(command).await,
        }
    }

    async fn resolve_management_order_owner_user_id(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: &str,
    ) -> Result<Option<String>, CommerceServiceError> {
        match self {
            Self::Postgres { orders, .. } => {
                orders
                    .resolve_management_order_owner_user_id(tenant_id, organization_id, order_id)
                    .await
            }
        }
    }

    async fn retrieve_owner_order_fulfillment_status(
        &self,
        query: OrderOwnerDetailQuery,
    ) -> Result<Option<String>, CommerceServiceError> {
        match self {
            Self::Postgres { orders, .. } => {
                orders.retrieve_owner_order_fulfillment_status(query).await
            }
        }
    }

    async fn list_management_shipments(
        &self,
        query: ShipmentManagementListQuery,
    ) -> Result<sdkwork_order_service::ShipmentManagementListPage, CommerceServiceError> {
        match self {
            Self::Postgres { orders, .. } => orders.list_management_shipments(query).await,
        }
    }

    async fn retrieve_management_shipment(
        &self,
        query: ShipmentManagementDetailQuery,
    ) -> Result<Option<ShipmentView>, CommerceServiceError> {
        match self {
            Self::Postgres { orders, .. } => orders.retrieve_management_shipment(query).await,
        }
    }

    async fn list_management_packages(
        &self,
        query: ShipmentPackageManagementListQuery,
    ) -> Result<sdkwork_order_service::ShipmentPackagePage, CommerceServiceError> {
        match self {
            Self::Postgres { orders, .. } => orders.list_management_shipment_packages(query).await,
        }
    }

    async fn create_management_package(
        &self,
        command: CreateShipmentPackageCommand,
    ) -> Result<ShipmentPackageView, CommerceServiceError> {
        match self {
            Self::Postgres { orders, .. } => {
                orders.create_management_shipment_package(command).await
            }
        }
    }

    async fn update_management_package(
        &self,
        command: UpdateShipmentPackageCommand,
    ) -> Result<ShipmentPackageView, CommerceServiceError> {
        match self {
            Self::Postgres { orders, .. } => {
                orders.update_management_shipment_package(command).await
            }
        }
    }

    async fn list_account_value_packages(
        &self,
        query: AccountValueCatalogListQuery,
    ) -> Result<AccountValuePackageListPage, CommerceServiceError> {
        match self {
            Self::Postgres { recharge, .. } => recharge.list_account_value_packages(query).await,
        }
    }

    async fn upsert_account_value_package(
        &self,
        command: UpsertAccountValuePackageCommand,
    ) -> Result<AccountValuePackageItem, CommerceServiceError> {
        match self {
            Self::Postgres { recharge, .. } => recharge.upsert_account_value_package(command).await,
        }
    }

    async fn retire_account_value_package(
        &self,
        command: RetireAccountValuePackageCommand,
    ) -> Result<(), CommerceServiceError> {
        match self {
            Self::Postgres { recharge, .. } => recharge.retire_account_value_package(command).await,
        }
    }

    async fn list_token_bank_plans(
        &self,
        query: AccountValueCatalogListQuery,
    ) -> Result<TokenBankPlanListPage, CommerceServiceError> {
        match self {
            Self::Postgres { recharge, .. } => recharge.list_token_bank_plans(query).await,
        }
    }

    async fn upsert_token_bank_plan(
        &self,
        command: UpsertTokenBankPlanCommand,
    ) -> Result<TokenBankPlanItem, CommerceServiceError> {
        match self {
            Self::Postgres { recharge, .. } => recharge.upsert_token_bank_plan(command).await,
        }
    }

    async fn retire_token_bank_plan(
        &self,
        command: RetireTokenBankPlanCommand,
    ) -> Result<(), CommerceServiceError> {
        match self {
            Self::Postgres { recharge, .. } => recharge.retire_token_bank_plan(command).await,
        }
    }

    async fn list_account_value_requests(
        &self,
        query: AccountValueRequestListQuery,
        subject: AccountValueOrderSubject,
    ) -> Result<AccountValueRequestListPage, CommerceServiceError> {
        match (self, subject) {
            (Self::Postgres { recharge, .. }, AccountValueOrderSubject::RefundRequest) => {
                recharge.list_order_refund_requests(query).await
            }
            (Self::Postgres { recharge, .. }, AccountValueOrderSubject::CashWithdrawal) => {
                recharge.list_cash_withdrawal_requests(query).await
            }
            _ => Err(CommerceServiceError::validation(
                "unsupported account value request subject",
            )),
        }
    }

    async fn execute_account_value_request_review(
        &self,
        ledger_port: &dyn AccountValueLedgerPort,
        refund_executor: &dyn PaymentRefundExecutorPort,
        payout_executor: &dyn PaymentPayoutExecutorPort,
        inventory: &dyn PhysicalInventoryReservationPort,
        command: ReviewAccountValueRequestCommand,
    ) -> Result<AccountValueRequestView, CommerceServiceError> {
        match self {
            Self::Postgres { recharge, .. } => {
                execute_account_value_request_review(
                    recharge.as_ref(),
                    ledger_port,
                    refund_executor,
                    payout_executor,
                    inventory,
                    command,
                )
                .await
            }
        }
    }
}

pub fn backend_commerce_admin_router_with_postgres_pool(pool: PgPool) -> Router {
    backend_commerce_admin_router_with_postgres_pool_and_ports(
        pool,
        Arc::new(NoopAccountValueLedgerPort),
        Arc::new(NoopPaymentRefundExecutorPort),
        Arc::new(NoopPaymentPayoutExecutorPort),
        Arc::new(UnavailablePhysicalInventoryReservationPort),
    )
}

pub fn backend_commerce_admin_router_with_postgres_pool_and_ports(
    pool: PgPool,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    payment_refund_executor_port: Arc<dyn PaymentRefundExecutorPort>,
    payment_payout_executor_port: Arc<dyn PaymentPayoutExecutorPort>,
    inventory: Arc<dyn PhysicalInventoryReservationPort>,
) -> Router {
    build_backend_commerce_admin_router(
        BackendCommerceAdminStore::Postgres {
            orders: Arc::new(PostgresCommerceOrderStore::new(pool.clone())),
            recharge: Arc::new(PostgresCommerceRechargeStore::new(pool)),
        },
        account_value_ledger_port,
        payment_refund_executor_port,
        payment_payout_executor_port,
        inventory,
    )
}

fn build_backend_commerce_admin_router(
    store: BackendCommerceAdminStore,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    payment_refund_executor_port: Arc<dyn PaymentRefundExecutorPort>,
    payment_payout_executor_port: Arc<dyn PaymentPayoutExecutorPort>,
    inventory: Arc<dyn PhysicalInventoryReservationPort>,
) -> Router {
    Router::new()
        .route(
            "/backend/v3/api/account_value_packages",
            get(list_account_value_packages).post(create_account_value_package),
        )
        .route(
            "/backend/v3/api/account_value_packages/{packageId}",
            patch(update_account_value_package),
        )
        .route(
            "/backend/v3/api/account_value_packages/{packageId}/retire",
            post(retire_account_value_package),
        )
        .route(
            "/backend/v3/api/token_bank_plans",
            get(list_token_bank_plans).post(create_token_bank_plan),
        )
        .route(
            "/backend/v3/api/token_bank_plans/{planCode}",
            patch(update_token_bank_plan),
        )
        .route(
            "/backend/v3/api/token_bank_plans/{planCode}/retire",
            post(retire_token_bank_plan),
        )
        .route("/backend/v3/api/refund_requests", get(list_refund_requests))
        .route(
            "/backend/v3/api/refund_requests/{refundRequestId}/approve",
            post(approve_refund_request),
        )
        .route(
            "/backend/v3/api/refund_requests/{refundRequestId}/reject",
            post(reject_refund_request),
        )
        .route(
            "/backend/v3/api/refund_requests/{refundRequestId}/retry",
            post(retry_refund_request),
        )
        .route(
            "/backend/v3/api/withdrawal_requests",
            get(list_withdrawal_requests),
        )
        .route(
            "/backend/v3/api/withdrawal_requests/{withdrawalRequestId}/approve",
            post(approve_withdrawal_request),
        )
        .route(
            "/backend/v3/api/withdrawal_requests/{withdrawalRequestId}/reject",
            post(reject_withdrawal_request),
        )
        .route(
            "/backend/v3/api/withdrawal_requests/{withdrawalRequestId}/retry",
            post(retry_withdrawal_request),
        )
        .route(
            "/backend/v3/api/after_sales/requests",
            get(list_management_after_sales),
        )
        .route(
            "/backend/v3/api/after_sales/requests/{afterSalesRequestId}",
            get(retrieve_management_after_sales),
        )
        .route(
            "/backend/v3/api/after_sales/requests/{afterSalesRequestId}/reviews",
            post(review_after_sales_request),
        )
        .route("/backend/v3/api/shipments", get(list_management_shipments))
        .route(
            "/backend/v3/api/shipments/{shipmentId}",
            get(retrieve_management_shipment),
        )
        .route(
            "/backend/v3/api/shipments/{shipmentId}/packages",
            get(list_management_shipment_packages).post(create_management_shipment_package),
        )
        .route(
            "/backend/v3/api/shipments/{shipmentId}/packages/{packageId}",
            patch(update_management_shipment_package),
        )
        .with_state(BackendCommerceAdminState {
            store,
            account_value_ledger_port,
            payment_refund_executor_port,
            payment_payout_executor_port,
            inventory,
        })
}

async fn list_account_value_packages(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Query(params): Query<AccountValueCatalogListParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match require_admin_subject(ctx, runtime_context, permissions::ACCOUNT_VALUE_READ)
    {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let page_params = match parse_offset_list_params_validated(ctx, params.page, params.page_size) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let target_asset = match params
        .target_asset
        .as_deref()
        .map(AccountValueAssetCode::parse)
        .transpose()
    {
        Ok(value) => value,
        Err(error) => return map_service_error(ctx, error),
    };
    let query = match AccountValueCatalogListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        target_asset,
        params.status.as_deref(),
        Some(page_params.page),
        Some(page_params.page_size),
    ) {
        Ok(query) => query,
        Err(error) => return crate::api_response::validation(ctx, error.message()),
    };

    match state.store.list_account_value_packages(query).await {
        Ok(page) => success_items(
            ctx,
            page.items
                .into_iter()
                .map(map_account_value_package)
                .collect(),
            page.total,
            page_params,
        ),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn create_account_value_package(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    body: Option<Json<AccountValuePackageWriteBody>>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject =
        match require_admin_subject(ctx, runtime_context, permissions::ACCOUNT_VALUE_MANAGE) {
            Ok(subject) => subject,
            Err(response) => return *response,
        };
    let body = body.map(|Json(value)| value).unwrap_or_default();
    let write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("account-value-package-create-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let command = match build_account_value_package_command(
        &subject,
        None,
        &body,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.upsert_account_value_package(command).await {
        Ok(item) => success_created_item(ctx, map_account_value_package(item)),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn update_account_value_package(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(package_id): Path<String>,
    body: Option<Json<AccountValuePackageWriteBody>>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject =
        match require_admin_subject(ctx, runtime_context, permissions::ACCOUNT_VALUE_MANAGE) {
            Ok(subject) => subject,
            Err(response) => return *response,
        };
    let body = body.map(|Json(value)| value).unwrap_or_default();
    let write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("account-value-package-update-{package_id}-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let command = match build_account_value_package_command(
        &subject,
        Some(&package_id),
        &body,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.upsert_account_value_package(command).await {
        Ok(item) => success_item(ctx, map_account_value_package(item)),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn retire_account_value_package(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(package_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject =
        match require_admin_subject(ctx, runtime_context, permissions::ACCOUNT_VALUE_MANAGE) {
            Ok(subject) => subject,
            Err(response) => return *response,
        };
    let write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("account-value-package-retire-{package_id}-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let command = match RetireAccountValuePackageCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &package_id,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.retire_account_value_package(command).await {
        Ok(()) => success_command(ctx, Some(package_id), Some("retired".to_owned())),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn list_token_bank_plans(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Query(params): Query<AccountValueCatalogListParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match require_admin_subject(ctx, runtime_context, permissions::ACCOUNT_VALUE_READ)
    {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let page_params = match parse_offset_list_params_validated(ctx, params.page, params.page_size) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let query = match AccountValueCatalogListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        Some(AccountValueAssetCode::TokenBank),
        params.status.as_deref(),
        Some(page_params.page),
        Some(page_params.page_size),
    ) {
        Ok(query) => query,
        Err(error) => return crate::api_response::validation(ctx, error.message()),
    };

    match state.store.list_token_bank_plans(query).await {
        Ok(page) => success_items(
            ctx,
            page.items.into_iter().map(map_token_bank_plan).collect(),
            page.total,
            page_params,
        ),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn create_token_bank_plan(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    body: Option<Json<TokenBankPlanWriteBody>>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject =
        match require_admin_subject(ctx, runtime_context, permissions::ACCOUNT_VALUE_MANAGE) {
            Ok(subject) => subject,
            Err(response) => return *response,
        };
    let body = body.map(|Json(value)| value).unwrap_or_default();
    let write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("token-bank-plan-create-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let command = match build_token_bank_plan_command(
        &subject,
        None,
        &body,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.upsert_token_bank_plan(command).await {
        Ok(item) => success_created_item(ctx, map_token_bank_plan(item)),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn update_token_bank_plan(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(plan_code): Path<String>,
    body: Option<Json<TokenBankPlanWriteBody>>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject =
        match require_admin_subject(ctx, runtime_context, permissions::ACCOUNT_VALUE_MANAGE) {
            Ok(subject) => subject,
            Err(response) => return *response,
        };
    let body = body.map(|Json(value)| value).unwrap_or_default();
    let write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("token-bank-plan-update-{plan_code}-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let command = match build_token_bank_plan_command(
        &subject,
        Some(&plan_code),
        &body,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.upsert_token_bank_plan(command).await {
        Ok(item) => success_item(ctx, map_token_bank_plan(item)),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn retire_token_bank_plan(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(plan_code): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject =
        match require_admin_subject(ctx, runtime_context, permissions::ACCOUNT_VALUE_MANAGE) {
            Ok(subject) => subject,
            Err(response) => return *response,
        };
    let write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("token-bank-plan-retire-{plan_code}-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let command = match RetireTokenBankPlanCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &plan_code,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.retire_token_bank_plan(command).await {
        Ok(()) => success_command(ctx, Some(plan_code), Some("retired".to_owned())),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn list_refund_requests(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Query(params): Query<AccountValueRequestListParams>,
) -> Response {
    list_account_value_requests_by_subject(
        state,
        runtime_context,
        request_context,
        params,
        AccountValueOrderSubject::RefundRequest,
    )
    .await
}

async fn list_withdrawal_requests(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Query(params): Query<AccountValueRequestListParams>,
) -> Response {
    list_account_value_requests_by_subject(
        state,
        runtime_context,
        request_context,
        params,
        AccountValueOrderSubject::CashWithdrawal,
    )
    .await
}

async fn approve_refund_request(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(refund_request_id): Path<String>,
    body: Option<Json<AccountValueRequestReviewBody>>,
) -> Response {
    review_account_value_request_by_action(
        state,
        runtime_context,
        request_context,
        headers,
        refund_request_id,
        body.map(|Json(value)| value).unwrap_or_default(),
        AccountValueOrderSubject::RefundRequest,
        AccountValueRequestReviewAction::Approve,
        "refund-approve",
    )
    .await
}

async fn reject_refund_request(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(refund_request_id): Path<String>,
    body: Option<Json<AccountValueRequestReviewBody>>,
) -> Response {
    review_account_value_request_by_action(
        state,
        runtime_context,
        request_context,
        headers,
        refund_request_id,
        body.map(|Json(value)| value).unwrap_or_default(),
        AccountValueOrderSubject::RefundRequest,
        AccountValueRequestReviewAction::Reject,
        "refund-reject",
    )
    .await
}

async fn retry_refund_request(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(refund_request_id): Path<String>,
    body: Option<Json<AccountValueRequestReviewBody>>,
) -> Response {
    review_account_value_request_by_action(
        state,
        runtime_context,
        request_context,
        headers,
        refund_request_id,
        body.map(|Json(value)| value).unwrap_or_default(),
        AccountValueOrderSubject::RefundRequest,
        AccountValueRequestReviewAction::Retry,
        "refund-retry",
    )
    .await
}

async fn approve_withdrawal_request(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(withdrawal_request_id): Path<String>,
    body: Option<Json<AccountValueRequestReviewBody>>,
) -> Response {
    review_account_value_request_by_action(
        state,
        runtime_context,
        request_context,
        headers,
        withdrawal_request_id,
        body.map(|Json(value)| value).unwrap_or_default(),
        AccountValueOrderSubject::CashWithdrawal,
        AccountValueRequestReviewAction::Approve,
        "withdrawal-approve",
    )
    .await
}

async fn reject_withdrawal_request(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(withdrawal_request_id): Path<String>,
    body: Option<Json<AccountValueRequestReviewBody>>,
) -> Response {
    review_account_value_request_by_action(
        state,
        runtime_context,
        request_context,
        headers,
        withdrawal_request_id,
        body.map(|Json(value)| value).unwrap_or_default(),
        AccountValueOrderSubject::CashWithdrawal,
        AccountValueRequestReviewAction::Reject,
        "withdrawal-reject",
    )
    .await
}

async fn retry_withdrawal_request(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(withdrawal_request_id): Path<String>,
    body: Option<Json<AccountValueRequestReviewBody>>,
) -> Response {
    review_account_value_request_by_action(
        state,
        runtime_context,
        request_context,
        headers,
        withdrawal_request_id,
        body.map(|Json(value)| value).unwrap_or_default(),
        AccountValueOrderSubject::CashWithdrawal,
        AccountValueRequestReviewAction::Retry,
        "withdrawal-retry",
    )
    .await
}

async fn list_account_value_requests_by_subject(
    state: BackendCommerceAdminState,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    params: AccountValueRequestListParams,
    request_subject: AccountValueOrderSubject,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match require_admin_subject(ctx, runtime_context, permissions::ACCOUNT_VALUE_READ)
    {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let page_params = match parse_offset_list_params_validated(ctx, params.page, params.page_size) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let query = match AccountValueRequestListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        None,
        Some(request_subject),
        params.status.as_deref(),
        Some(page_params.page),
        Some(page_params.page_size),
    ) {
        Ok(query) => query,
        Err(error) => return crate::api_response::validation(ctx, error.message()),
    };

    match state
        .store
        .list_account_value_requests(query, request_subject)
        .await
    {
        Ok(page) => success_items(
            ctx,
            page.items
                .into_iter()
                .map(map_account_value_request)
                .collect(),
            page.total,
            page_params,
        ),
        Err(error) => map_service_error(ctx, error),
    }
}

#[allow(clippy::too_many_arguments)]
async fn review_account_value_request_by_action(
    state: BackendCommerceAdminState,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    request_id: String,
    body: AccountValueRequestReviewBody,
    request_subject: AccountValueOrderSubject,
    action: AccountValueRequestReviewAction,
    fallback_scope: &'static str,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject =
        match require_admin_subject(ctx, runtime_context, permissions::ACCOUNT_VALUE_REVIEW) {
            Ok(subject) => subject,
            Err(response) => return *response,
        };
    let write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("{fallback_scope}-{request_id}-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let command = match ReviewAccountValueRequestCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        request_subject,
        &request_id,
        action,
        body.reason_code.as_deref(),
        body.review_comment.as_deref(),
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match state
        .store
        .execute_account_value_request_review(
            state.account_value_ledger_port.as_ref(),
            state.payment_refund_executor_port.as_ref(),
            state.payment_payout_executor_port.as_ref(),
            state.inventory.as_ref(),
            command,
        )
        .await
    {
        Ok(view) => success_command(ctx, Some(request_id), Some(view.status)),
        Err(error) => map_service_error(ctx, error),
    }
}

fn require_admin_subject(
    ctx: Option<&WebRequestContext>,
    runtime_context: Option<Extension<IamAppContext>>,
    permission: &'static str,
) -> Result<BackendOperatorScope, Box<Response>> {
    match runtime_context {
        Some(Extension(context)) => require_backend_operator(ctx, context, permission),
        None => Err(Box::new(crate::api_response::unauthorized(
            ctx,
            "authentication is required",
        ))),
    }
}

fn build_account_value_package_command(
    subject: &BackendOperatorScope,
    package_id: Option<&str>,
    body: &AccountValuePackageWriteBody,
    request_no: &str,
    idempotency_key: &str,
) -> Result<UpsertAccountValuePackageCommand, CommerceServiceError> {
    let package_code = required_body_text(body.package_code.as_deref(), "packageCode")?;
    let display_name = required_body_text(body.display_name.as_deref(), "displayName")?;
    let target_asset = AccountValueAssetCode::parse(&required_body_text(
        body.target_asset.as_deref(),
        "targetAsset",
    )?)?;
    let grant_amount = required_money(body.grant_amount.as_deref(), "grantAmount")?;
    let bonus_amount = optional_money(body.bonus_amount.as_deref(), "bonusAmount")?;
    let price_amount = required_money(body.price_amount.as_deref(), "priceAmount")?;
    let currency_code = required_body_text(body.currency_code.as_deref(), "currencyCode")?;

    UpsertAccountValuePackageCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        package_id,
        &package_code,
        &display_name,
        target_asset,
        grant_amount,
        bonus_amount,
        price_amount,
        &currency_code,
        body.status.as_deref(),
        body.sort_weight,
        body.valid_from.as_deref(),
        body.valid_to.as_deref(),
        request_no,
        idempotency_key,
    )
}

fn build_token_bank_plan_command(
    subject: &BackendOperatorScope,
    route_plan_code: Option<&str>,
    body: &TokenBankPlanWriteBody,
    request_no: &str,
    idempotency_key: &str,
) -> Result<UpsertTokenBankPlanCommand, CommerceServiceError> {
    let plan_code = route_plan_code
        .map(str::to_owned)
        .or_else(|| body.plan_code.as_deref().map(str::to_owned))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CommerceServiceError::validation("planCode is required"))?;
    let display_name = required_body_text(body.display_name.as_deref(), "displayName")?;
    let plan_period = TokenBankPlanPeriod::parse(&required_body_text(
        body.plan_period.as_deref(),
        "planPeriod",
    )?)?;
    let grant_amount = required_money(body.grant_amount.as_deref(), "grantAmount")?;
    let bonus_amount = optional_money(body.bonus_amount.as_deref(), "bonusAmount")?;
    let price_amount = required_money(body.price_amount.as_deref(), "priceAmount")?;
    let currency_code = required_body_text(body.currency_code.as_deref(), "currencyCode")?;

    UpsertTokenBankPlanCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &plan_code,
        &display_name,
        plan_period,
        grant_amount,
        bonus_amount,
        price_amount,
        &currency_code,
        body.renewal_policy.as_deref(),
        body.status.as_deref(),
        body.sort_weight,
        request_no,
        idempotency_key,
    )
}

fn required_body_text(
    value: Option<&str>,
    field_name: &'static str,
) -> Result<String, CommerceServiceError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| CommerceServiceError::validation(format!("{field_name} is required")))
}

fn required_money(
    value: Option<&str>,
    field_name: &'static str,
) -> Result<CommerceMoney, CommerceServiceError> {
    let amount = required_body_text(value, field_name)?;
    CommerceMoney::new(&amount)
        .map_err(|message| CommerceServiceError::validation(format!("{field_name} {message}")))
}

fn optional_money(
    value: Option<&str>,
    field_name: &'static str,
) -> Result<CommerceMoney, CommerceServiceError> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => CommerceMoney::new(value)
            .map_err(|message| CommerceServiceError::validation(format!("{field_name} {message}"))),
        None => CommerceMoney::new("0").map_err(CommerceServiceError::validation),
    }
}

async fn list_management_after_sales(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Query(params): Query<AfterSalesListParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match require_admin_subject(ctx, runtime_context, permissions::AFTER_SALES_READ) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let page_params = match parse_offset_list_params_validated(ctx, params.page, params.page_size) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let query = match AfterSalesManagementListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        params.order_id.as_deref(),
        params.after_sales_type.as_deref(),
        params.status.as_deref(),
        None,
        Some(page_params.page),
        Some(page_params.page_size),
    ) {
        Ok(query) => query,
        Err(error) => return crate::api_response::validation(ctx, error.message()),
    };

    match state.store.list_management_after_sales(query).await {
        Ok(page) => success_items(
            ctx,
            page.items
                .into_iter()
                .map(map_after_sales_request)
                .collect(),
            page.total,
            page_params,
        ),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn retrieve_management_after_sales(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(after_sales_request_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match require_admin_subject(ctx, runtime_context, permissions::AFTER_SALES_READ) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let query = match AfterSalesManagementDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &after_sales_request_id,
    ) {
        Ok(query) => query,
        Err(error) => return crate::api_response::validation(ctx, error.message()),
    };

    match state.store.retrieve_management_after_sales(query).await {
        Ok(Some(request)) => success_item(ctx, map_after_sales_request(request)),
        Ok(None) => crate::api_response::not_found(ctx, "after sales request was not found"),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn review_after_sales_request(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(after_sales_request_id): Path<String>,
    Json(body): Json<ReviewAfterSalesRequestBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match require_admin_subject(ctx, runtime_context, permissions::AFTER_SALES_REVIEW)
    {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("review-{after_sales_request_id}-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let command = match ReviewAfterSalesRequestCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &after_sales_request_id,
        &body.review_action,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command
            .with_status(body.status)
            .with_refund_status(body.refund_status)
            .with_return_status(body.return_status)
            .with_exchange_status(body.exchange_status)
            .with_approved_amount(body.approved_amount)
            .with_reason_code(body.reason_code)
            .with_review_comment(body.review_comment.or(body.reason_detail)),
        Err(error) => return crate::api_response::validation(ctx, error.message()),
    };

    match state.store.review_after_sales(command).await {
        Ok(request) => {
            if request.status.eq_ignore_ascii_case("completed") {
                // Completed physical returns restore inventory: shipped
                // orders restock consumed reservations, unshipped orders
                // release their reservations. Best-effort — a restock
                // failure must not roll back the review itself.
                maybe_restock_physical_return(
                    &state,
                    &subject.tenant_id,
                    subject.organization_id.as_deref(),
                    &request.order_id,
                )
                .await;
            }
            success_created_item(ctx, map_after_sales_request(request))
        }
        Err(error) => map_service_error(ctx, error),
    }
}

/// Restores physical inventory after a completed return review. The decision
/// follows the order's stored `fulfillment_status`:
/// - `awaiting_shipment`/`shipped`/`delivered` (stock already consumed) →
///   `restock_consumed_order_inventory`
/// - `inventory_reserved` (paid but never shipped) → `release_physical_order_inventory`
/// - anything else (virtual orders, no reservation) → no inventory action
async fn maybe_restock_physical_return(
    state: &BackendCommerceAdminState,
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
) {
    let owner_user_id = match state
        .store
        .resolve_management_order_owner_user_id(tenant_id, organization_id, order_id)
        .await
    {
        Ok(Some(value)) => value,
        _ => return,
    };
    let fulfillment_status = match state
        .store
        .retrieve_owner_order_fulfillment_status(OrderOwnerDetailQuery {
            tenant_id: tenant_id.to_owned(),
            organization_id: organization_id.map(str::to_owned),
            owner_user_id,
            order_id: order_id.to_owned(),
        })
        .await
    {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(
                target = "order.after_sales",
                order_id,
                error = ?error,
                "failed to resolve fulfillment status for physical return restock"
            );
            return;
        }
    };
    let request = ReleasePhysicalOrderInventoryRequest {
        tenant_id: tenant_id.to_owned(),
        order_id: order_id.to_owned(),
        reason_code: "buyer_return".to_owned(),
        request_no: format!("restock-{order_id}"),
        idempotency_key: physical_inventory_release_idempotency_key(order_id),
    };
    let outcome = match fulfillment_status
        .as_deref()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("awaiting_shipment" | "shipped" | "delivered") => {
            state
                .inventory
                .restock_consumed_order_inventory(request)
                .await
        }
        Some("inventory_reserved") => {
            state
                .inventory
                .release_physical_order_inventory(request)
                .await
        }
        _ => return,
    };
    if let Err(error) = outcome {
        tracing::warn!(
            target = "order.after_sales",
            order_id,
            error = ?error,
            "failed to restore physical inventory after return review"
        );
    }
}

async fn list_management_shipments(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Query(params): Query<ShipmentListParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match require_admin_subject(ctx, runtime_context, permissions::ORDERS_READ) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let page_params = match parse_offset_list_params_validated(ctx, params.page, params.page_size) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let query = match ShipmentManagementListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        params.order_id.as_deref(),
        params.fulfillment_id.as_deref(),
        params.status.as_deref(),
        Some(page_params.page),
        Some(page_params.page_size),
    ) {
        Ok(query) => query,
        Err(error) => return crate::api_response::validation(ctx, error.message()),
    };

    match state.store.list_management_shipments(query).await {
        Ok(page) => success_items(
            ctx,
            page.items.into_iter().map(map_shipment).collect(),
            page.total,
            page_params,
        ),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn retrieve_management_shipment(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(shipment_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match require_admin_subject(ctx, runtime_context, permissions::ORDERS_READ) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let query = match ShipmentManagementDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &shipment_id,
    ) {
        Ok(query) => query,
        Err(error) => return crate::api_response::validation(ctx, error.message()),
    };

    match state.store.retrieve_management_shipment(query).await {
        Ok(Some(shipment)) => success_item(ctx, map_shipment(shipment)),
        Ok(None) => crate::api_response::not_found(ctx, "shipment was not found"),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn list_management_shipment_packages(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(shipment_id): Path<String>,
    Query(params): Query<PackageListParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match require_admin_subject(ctx, runtime_context, permissions::ORDERS_READ) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let page_params = match parse_offset_list_params_validated(ctx, params.page, params.page_size) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let query = match ShipmentPackageManagementListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &shipment_id,
        Some(page_params.page),
        Some(page_params.page_size),
    ) {
        Ok(query) => query,
        Err(error) => return crate::api_response::validation(ctx, error.message()),
    };

    match state.store.list_management_packages(query).await {
        Ok(page) => success_items(
            ctx,
            page.items.into_iter().map(map_shipment_package).collect(),
            page.total,
            page_params,
        ),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn create_management_shipment_package(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(shipment_id): Path<String>,
    Json(body): Json<CreateShipmentPackageBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match require_admin_subject(ctx, runtime_context, permissions::ORDERS_MANAGE) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("pkg-{shipment_id}-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let mut command = match CreateShipmentPackageCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &shipment_id,
        &body.package_type,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return crate::api_response::validation(ctx, error.message()),
    };
    command.package_no = body.package_no;
    command.tracking_no = body.tracking_no;
    command.status = body.status;

    match state.store.create_management_package(command).await {
        Ok(package) => success_created_item(ctx, map_shipment_package(package)),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn update_management_shipment_package(
    State(state): State<BackendCommerceAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path((shipment_id, package_id)): Path<(String, String)>,
    Json(body): Json<UpdateShipmentPackageBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match require_admin_subject(ctx, runtime_context, permissions::ORDERS_MANAGE) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };
    let write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("pkg-update-{package_id}-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let mut command = match UpdateShipmentPackageCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &shipment_id,
        &package_id,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return crate::api_response::validation(ctx, error.message()),
    };
    command.package_type = body.package_type;
    command.tracking_no = body.tracking_no;
    command.status = body.status;

    match state.store.update_management_package(command).await {
        Ok(package) => success_item(ctx, map_shipment_package(package)),
        Err(error) => map_service_error(ctx, error),
    }
}

fn map_after_sales_request(value: AfterSalesRequestView) -> AfterSalesRequestResponse {
    AfterSalesRequestResponse {
        after_sales_request_id: value.after_sales_request_id,
        after_sales_no: value.after_sales_no,
        order_id: value.order_id,
        after_sales_type: value.after_sales_type,
        reason_code: value.reason_code,
        requested_amount: value.requested_amount.as_str().to_owned(),
        currency_code: value.currency_code,
        status: value.status,
    }
}

fn map_account_value_package(value: AccountValuePackageItem) -> AccountValuePackageResponse {
    AccountValuePackageResponse {
        package_id: value.package_id,
        package_code: value.package_code,
        display_name: value.display_name,
        target_asset: value.target_asset.as_str().to_owned(),
        grant_amount: value.grant_amount.as_str().to_owned(),
        bonus_amount: value.bonus_amount.as_str().to_owned(),
        price_amount: value.price_amount.as_str().to_owned(),
        currency_code: value.currency_code,
        status: value.status,
    }
}

fn map_token_bank_plan(value: TokenBankPlanItem) -> TokenBankPlanResponse {
    TokenBankPlanResponse {
        plan_code: value.plan_code,
        display_name: value.display_name,
        plan_period: value.plan_period.as_str().to_owned(),
        grant_amount: value.grant_amount.as_str().to_owned(),
        bonus_amount: value.bonus_amount.as_str().to_owned(),
        price_amount: value.price_amount.as_str().to_owned(),
        currency_code: value.currency_code,
        renewal_policy: value.renewal_policy,
        status: value.status,
    }
}

fn map_account_value_request(value: AccountValueRequestView) -> AccountValueRequestResponse {
    AccountValueRequestResponse {
        account_value_request_id: value.request_id,
        request_no: value.request_no,
        original_order_id: value.original_order_id,
        owner_user_id: value.owner_user_id,
        subject: value.subject.as_str().to_owned(),
        target_asset: value.target_asset.as_str().to_owned(),
        amount: value.amount.as_str().to_owned(),
        currency_code: value.currency_code,
        status: value.status,
        provider_reference_id: value.provider_reference_id,
        created_at: value.created_at,
        updated_at: value.updated_at,
    }
}

fn map_shipment(value: ShipmentView) -> ShipmentResponse {
    ShipmentResponse {
        shipment_id: value.shipment_id,
        shipment_no: value.shipment_no,
        fulfillment_id: value.fulfillment_id,
        carrier_code: value.carrier_code,
        tracking_no: value.tracking_no,
        status: value.status,
    }
}

fn map_shipment_package(value: ShipmentPackageView) -> ShipmentPackageResponse {
    ShipmentPackageResponse {
        package_id: value.package_id,
        shipment_id: value.shipment_id,
        package_no: value.package_no,
        package_type: value.package_type,
        tracking_no: value.tracking_no,
        status: value.status,
    }
}
