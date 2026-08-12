use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use std::collections::BTreeMap;

use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_repository_sqlx::{PostgresCommerceOrderStore, PostgresCommerceRechargeStore};
use sdkwork_order_service::{
    default_fulfill_account_value_order_command, redeem_coupon_and_fulfill_account_value_order,
    redeem_coupon_and_fulfill_order, AccountValueAssetCode, AccountValueCatalogListQuery,
    AccountValueFulfillmentStore, AccountValueLedgerPort, AccountValueOrderSubject,
    AccountValueRequestDetailQuery, AccountValueRequestListPage, AccountValueRequestListQuery,
    AccountValueRequestView, CancelOwnerOrderCommand, CheckoutStatusQuery, CheckoutStatusSnapshot,
    CouponRedemptionBenefit, CouponRedemptionPort, CouponRedemptionRequest,
    CreateAccountRechargeOrderCommand, CreateAccountRechargeOrderOutcome,
    CreateCashWithdrawalRequestCommand, CreateCouponRechargeOrderCommand,
    CreateOrderRefundRequestCommand, CreatePointsRechargeOrderCommand,
    CreatePointsRechargeOrderOutcome, FulfillAccountValueOrderCommand,
    MembershipPurchaseFulfillmentPort, NoopAccountValueLedgerPort, NoopCouponRedemptionPort,
    NoopMembershipPurchaseFulfillmentPort, OrderOwnerListQuery, PayOwnerOrderCommand,
    PayOwnerOrderCommandInput, PayOwnerOrderOutcome, RechargeGrantPreview, RechargePackageItem,
    RechargePackageListPage, RechargePackageListQuery, RechargeSettingsQuery,
    RechargeSettingsSnapshot, TokenBankPlanItem, TokenBankPlanListPage, TokenBankPlanPeriod,
};
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_utils_rust::{add_minutes, format_datetime, now};
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::api_response::{
    map_service_error, not_found, offset_list_page_params_from_query,
    parse_offset_list_params_validated, success_command, success_created_item, success_item,
    success_items, unauthorized, validation,
};
use crate::command_headers::required_app_write_command_headers;
use crate::order_router::{CommerceOrderStore, OwnerOrderPaymentStore};
use crate::owner_order_cancel::{cancel_owner_order_with_payments, compensate_failed_recharge_pay};
use crate::owner_order_payment_enrich::enriched_postgres_owner_order_payments;
use crate::subject::{app_runtime_subject_from_contexts, AppRuntimeSubject};

const MAX_CHECKOUT_ORDER_NO_LEN: usize = 128;
const MAX_RECHARGE_CENTS: i64 = 1_000_000;
const PLATFORM_ORGANIZATION_SCOPE_SENTINEL: &str = "0";
const DEFAULT_PAYMENT_PRODUCT: &str = "mobile_cashier_h5";

/// Payment countdown in minutes, backed by the shared expiry window
/// (`SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS`, default 30 minutes).
fn payment_expire_minutes() -> i64 {
    sdkwork_order_service::payment_expire_seconds() / 60
}

/// 允许的支付方式白名单。新增支付方式时只需扩展此处。
const ALLOWED_PAYMENT_METHODS: &[&str] = &["wechat_pay", "alipay", "balance"];

pub type CommerceRechargeCheckoutFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommerceRechargeCheckoutStore: Send + Sync {
    fn list_recharge_packages<'a>(
        &'a self,
        query: RechargePackageListQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, RechargePackageListPage>;

    fn load_recharge_settings<'a>(
        &'a self,
        query: RechargeSettingsQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, RechargeSettingsSnapshot>;

    fn create_points_recharge_order<'a>(
        &'a self,
        command: CreatePointsRechargeOrderCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, CreatePointsRechargeOrderOutcome>;

    fn retrieve_checkout_status<'a>(
        &'a self,
        query: CheckoutStatusQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, Option<CheckoutStatusSnapshot>>;

    fn list_token_bank_plans<'a>(
        &'a self,
        query: AccountValueCatalogListQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, TokenBankPlanListPage>;

    fn create_account_recharge_order<'a>(
        &'a self,
        command: CreateAccountRechargeOrderCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, CreateAccountRechargeOrderOutcome>;

    fn create_coupon_recharge_order<'a>(
        &'a self,
        command: CreateCouponRechargeOrderCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, CreateAccountRechargeOrderOutcome>;

    fn load_account_value_order_by_idempotency<'a>(
        &'a self,
        tenant_id: &'a str,
        organization_id: Option<&'a str>,
        owner_user_id: &'a str,
        idempotency_key: &'a str,
    ) -> CommerceRechargeCheckoutFuture<'a, Option<CreateAccountRechargeOrderOutcome>>;

    fn list_order_refund_requests<'a>(
        &'a self,
        query: AccountValueRequestListQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, AccountValueRequestListPage>;

    fn create_order_refund_request<'a>(
        &'a self,
        command: CreateOrderRefundRequestCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, AccountValueRequestView>;

    fn retrieve_order_refund_request<'a>(
        &'a self,
        query: AccountValueRequestDetailQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, Option<AccountValueRequestView>>;

    fn create_cash_withdrawal_request<'a>(
        &'a self,
        command: CreateCashWithdrawalRequestCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, AccountValueRequestView>;

    fn retrieve_cash_withdrawal_request<'a>(
        &'a self,
        query: AccountValueRequestDetailQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, Option<AccountValueRequestView>>;
}

#[derive(Clone)]
struct AppRechargeCheckoutState {
    store: Arc<dyn CommerceRechargeCheckoutStore>,
    fulfillment_store: Arc<dyn AccountValueFulfillmentStore>,
    coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    membership_fulfillment_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
    orders: Arc<dyn CommerceOrderStore>,
    payments: Arc<dyn OwnerOrderPaymentStore>,
}

struct NoopAccountValueFulfillmentStore;

impl AccountValueFulfillmentStore for NoopAccountValueFulfillmentStore {
    fn load_account_value_fulfillment_context<'a>(
        &'a self,
        _command: &'a sdkwork_order_service::FulfillAccountValueOrderCommand,
    ) -> sdkwork_order_service::AccountValueFulfillmentFuture<
        'a,
        Option<sdkwork_order_service::AccountValueFulfillmentContext>,
    > {
        Box::pin(async move {
            Err(CommerceServiceError::unsupported_capability(
                "account value fulfillment store is not configured",
            ))
        })
    }

    fn reserve_account_value_fulfillment<'a>(
        &'a self,
        _command: &'a sdkwork_order_service::FulfillAccountValueOrderCommand,
        _context: &'a sdkwork_order_service::AccountValueFulfillmentContext,
    ) -> sdkwork_order_service::AccountValueFulfillmentFuture<'a, ()> {
        Box::pin(async move {
            Err(CommerceServiceError::unsupported_capability(
                "account value fulfillment store is not configured",
            ))
        })
    }

    fn release_account_value_fulfillment_reservation<'a>(
        &'a self,
        _command: &'a sdkwork_order_service::FulfillAccountValueOrderCommand,
        _context: &'a sdkwork_order_service::AccountValueFulfillmentContext,
    ) -> sdkwork_order_service::AccountValueFulfillmentFuture<'a, ()> {
        Box::pin(async move { Ok(()) })
    }

    fn commit_account_value_fulfillment<'a>(
        &'a self,
        _command: sdkwork_order_service::FulfillAccountValueOrderCommand,
        _context: &'a sdkwork_order_service::AccountValueFulfillmentContext,
    ) -> sdkwork_order_service::AccountValueFulfillmentFuture<
        'a,
        sdkwork_order_service::FulfillAccountValueOrderOutcome,
    > {
        Box::pin(async move {
            Err(CommerceServiceError::unsupported_capability(
                "account value fulfillment store is not configured",
            ))
        })
    }
}

#[derive(Debug, Deserialize)]
struct RechargeOrderListQueryParams {
    page: Option<i64>,
    page_size: Option<i64>,
    status: Option<String>,
    subject: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RechargePackageListQueryParams {
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AccountValueRequestListQueryParams {
    page: Option<i64>,
    page_size: Option<i64>,
    status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefundRequestCreateBody {
    original_order_id: Option<String>,
    target_asset: Option<String>,
    amount: Option<serde_json::Value>,
    currency_code: Option<String>,
    reason_code: Option<String>,
    reason_detail: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct WithdrawalRequestCreateBody {
    asset: Option<String>,
    amount: Option<serde_json::Value>,
    currency_code: Option<String>,
    payout_method: Option<String>,
    payout_account_ref: Option<String>,
    reason_code: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RechargeOrderSummaryResponse {
    order_id: String,
    order_no: String,
    status: String,
    subject: String,
    amount: String,
    points: i64,
    created_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmitRechargeRequest {
    subject: Option<String>,
    target_asset: Option<String>,
    amount: Option<serde_json::Value>,
    grant_amount: Option<serde_json::Value>,
    client_request_no: Option<String>,
    currency_code: Option<String>,
    package_id: Option<String>,
    plan_code: Option<String>,
    plan_period: Option<String>,
    coupon_code: Option<String>,
    payment_method: Option<String>,
    payment_product: Option<String>,
    payment_password: Option<String>,
    source: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CouponRedemptionCreateBody {
    coupon_code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CouponRedemptionResponse {
    order_id: String,
    order_no: String,
    status: String,
    replayed: bool,
    benefit: serde_json::Value,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct RechargeCancelRequest {
    #[serde(rename = "cancelReason", alias = "cancel_reason")]
    cancel_reason: Option<String>,
    #[serde(rename = "cancelType", alias = "cancel_type")]
    cancel_type: Option<String>,
}

struct CreateRechargeCommandInput<'a> {
    subject: &'a AppRuntimeSubject,
    amount: CommerceMoney,
    currency_code: &'a str,
    method: &'a str,
    request_no: &'a str,
    idempotency_key: &'a str,
    package_id: Option<&'a str>,
    client_request_no: Option<&'a str>,
    source: Option<&'a str>,
}

struct CreateAccountRechargeCommandInput<'a> {
    subject: &'a AppRuntimeSubject,
    order_subject: AccountValueOrderSubject,
    target_asset: AccountValueAssetCode,
    amount: CommerceMoney,
    grant_amount: CommerceMoney,
    currency_code: &'a str,
    method: Option<&'a str>,
    request_no: &'a str,
    idempotency_key: &'a str,
    package_id: Option<&'a str>,
    plan_code: Option<&'a str>,
    plan_period: Option<TokenBankPlanPeriod>,
    coupon_code: Option<&'a str>,
    client_request_no: Option<&'a str>,
}

impl SubmitRechargeRequest {
    fn subject(&self) -> Option<&str> {
        self.subject.as_deref()
    }

    fn target_asset(&self) -> Option<&str> {
        self.target_asset.as_deref()
    }

    fn amount_value(&self) -> Option<&serde_json::Value> {
        self.amount.as_ref()
    }

    fn grant_amount_value(&self) -> Option<&serde_json::Value> {
        self.grant_amount.as_ref()
    }

    fn currency_code(&self) -> Option<&str> {
        self.currency_code.as_deref()
    }

    fn package_id(&self) -> Option<&str> {
        self.package_id.as_deref()
    }

    fn plan_code(&self) -> Option<&str> {
        self.plan_code.as_deref()
    }

    fn plan_period(&self) -> Option<&str> {
        self.plan_period.as_deref()
    }

    fn coupon_code(&self) -> Option<&str> {
        self.coupon_code.as_deref()
    }

    fn client_request_no(&self) -> Option<&str> {
        self.client_request_no.as_deref()
    }

    fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    fn payment_method(&self) -> Option<&str> {
        self.payment_method.as_deref()
    }

    fn payment_product(&self) -> Option<&str> {
        self.payment_product.as_deref()
    }

    fn payment_password(&self) -> Option<&str> {
        self.payment_password.as_deref()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RechargePackageResponse {
    id: String,
    price_amount: String,
    currency_code: String,
    bonus_points: i64,
    grant_amount: i64,
    points: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RechargeGrantPreviewResponse {
    grant_amount: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RechargeSettingsResponse {
    base_currency_code: String,
    base_points_per_cny: String,
    currency_to_cny_rates: BTreeMap<String, String>,
    preview_examples: BTreeMap<String, BTreeMap<String, RechargeGrantPreviewResponse>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmitRechargeResponse {
    success: bool,
    order_id: String,
    order_no: String,
    out_trade_no: String,
    subject: String,
    target_asset: String,
    amount: String,
    grant_amount: String,
    currency_code: String,
    points: i64,
    provider_code: String,
    payment_method: String,
    payment_product: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<String>,
    status: String,
    next_action: String,
    cashier_url: String,
    qr_code_payload: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_payment_payload: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckoutStatusResponse {
    order_no: String,
    out_trade_no: String,
    amount: String,
    currency_code: String,
    points: i64,
    provider_code: String,
    payment_method: String,
    payment_product: String,
    order_status: String,
    payment_status: String,
    recharge_status: String,
    status: String,
    created_at: String,
    expires_at: String,
    paid_at: String,
    next_action: String,
    cashier_url: String,
    qr_code_payload: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_payment_payload: Option<String>,
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

impl CommerceRechargeCheckoutStore for PostgresCommerceRechargeStore {
    fn list_recharge_packages<'a>(
        &'a self,
        query: RechargePackageListQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, RechargePackageListPage> {
        Box::pin(async move { self.list_recharge_packages(query).await })
    }

    fn create_points_recharge_order<'a>(
        &'a self,
        command: CreatePointsRechargeOrderCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, CreatePointsRechargeOrderOutcome> {
        Box::pin(async move { self.create_points_recharge_order(command).await })
    }

    fn load_recharge_settings<'a>(
        &'a self,
        query: RechargeSettingsQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, RechargeSettingsSnapshot> {
        Box::pin(async move { self.load_recharge_settings(query).await })
    }

    fn retrieve_checkout_status<'a>(
        &'a self,
        query: CheckoutStatusQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, Option<CheckoutStatusSnapshot>> {
        Box::pin(async move { self.load_checkout_status(query).await })
    }

    fn list_token_bank_plans<'a>(
        &'a self,
        query: AccountValueCatalogListQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, TokenBankPlanListPage> {
        Box::pin(async move { self.list_token_bank_plans(query).await })
    }

    fn create_account_recharge_order<'a>(
        &'a self,
        command: CreateAccountRechargeOrderCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, CreateAccountRechargeOrderOutcome> {
        Box::pin(async move { self.create_account_recharge_order(command).await })
    }

    fn create_coupon_recharge_order<'a>(
        &'a self,
        command: CreateCouponRechargeOrderCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, CreateAccountRechargeOrderOutcome> {
        Box::pin(async move { self.create_coupon_recharge_order(command).await })
    }

    fn load_account_value_order_by_idempotency<'a>(
        &'a self,
        tenant_id: &'a str,
        organization_id: Option<&'a str>,
        owner_user_id: &'a str,
        idempotency_key: &'a str,
    ) -> CommerceRechargeCheckoutFuture<'a, Option<CreateAccountRechargeOrderOutcome>> {
        Box::pin(async move {
            self.load_account_value_order_by_idempotency(
                tenant_id,
                organization_id,
                owner_user_id,
                idempotency_key,
            )
            .await
        })
    }

    fn list_order_refund_requests<'a>(
        &'a self,
        query: AccountValueRequestListQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, AccountValueRequestListPage> {
        Box::pin(async move { self.list_order_refund_requests(query).await })
    }

    fn create_order_refund_request<'a>(
        &'a self,
        command: CreateOrderRefundRequestCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, AccountValueRequestView> {
        Box::pin(async move { self.create_order_refund_request(command).await })
    }

    fn retrieve_order_refund_request<'a>(
        &'a self,
        query: AccountValueRequestDetailQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, Option<AccountValueRequestView>> {
        Box::pin(async move { self.retrieve_order_refund_request(query).await })
    }

    fn create_cash_withdrawal_request<'a>(
        &'a self,
        command: CreateCashWithdrawalRequestCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, AccountValueRequestView> {
        Box::pin(async move { self.create_cash_withdrawal_request(command).await })
    }

    fn retrieve_cash_withdrawal_request<'a>(
        &'a self,
        query: AccountValueRequestDetailQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, Option<AccountValueRequestView>> {
        Box::pin(async move { self.retrieve_cash_withdrawal_request(query).await })
    }
}

pub fn app_recharge_checkout_router_with_postgres_pool(
    pool: PgPool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
) -> Router {
    let pool_for_orders = pool.clone();
    let store = Arc::new(PostgresCommerceRechargeStore::new(pool));
    build_app_recharge_checkout_router_with_integrations(
        store.clone(),
        store,
        Arc::new(NoopCouponRedemptionPort),
        Arc::new(NoopAccountValueLedgerPort),
        Arc::new(NoopMembershipPurchaseFulfillmentPort),
        Arc::new(PostgresCommerceOrderStore::new(pool_for_orders.clone())),
        enriched_postgres_owner_order_payments(pool_for_orders, registry, credentials),
    )
}

pub fn build_app_recharge_checkout_router(
    store: Arc<dyn CommerceRechargeCheckoutStore>,
    orders: Arc<dyn CommerceOrderStore>,
    payments: Arc<dyn OwnerOrderPaymentStore>,
) -> Router {
    build_app_recharge_checkout_router_with_integrations(
        store,
        Arc::new(NoopAccountValueFulfillmentStore),
        Arc::new(NoopCouponRedemptionPort),
        Arc::new(NoopAccountValueLedgerPort),
        Arc::new(NoopMembershipPurchaseFulfillmentPort),
        orders,
        payments,
    )
}

pub fn build_app_recharge_checkout_router_with_integrations(
    store: Arc<dyn CommerceRechargeCheckoutStore>,
    fulfillment_store: Arc<dyn AccountValueFulfillmentStore>,
    coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    membership_fulfillment_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
    orders: Arc<dyn CommerceOrderStore>,
    payments: Arc<dyn OwnerOrderPaymentStore>,
) -> Router {
    Router::new()
        .route(
            "/app/v3/api/recharges/packages",
            get(fetch_recharge_packages),
        )
        .route(
            "/app/v3/api/recharges/settings",
            get(fetch_recharge_settings),
        )
        .route("/app/v3/api/recharges/plans", get(list_token_bank_plans))
        .route(
            "/app/v3/api/recharges/orders",
            get(list_recharge_orders).post(submit_recharge),
        )
        .route(
            "/app/v3/api/recharges/orders/{orderId}",
            get(fetch_checkout_status),
        )
        .route(
            "/app/v3/api/recharges/orders/{orderId}/cancel",
            post(cancel_recharge_order),
        )
        .route(
            "/app/v3/api/orders/coupon_redemptions",
            post(create_coupon_redemption),
        )
        .route(
            "/app/v3/api/orders/refund_requests",
            get(list_refund_requests).post(create_refund_request),
        )
        .route(
            "/app/v3/api/orders/refund_requests/{refundRequestId}",
            get(retrieve_refund_request),
        )
        .route(
            "/app/v3/api/withdrawals/requests",
            post(create_withdrawal_request),
        )
        .route(
            "/app/v3/api/withdrawals/requests/{withdrawalRequestId}",
            get(retrieve_withdrawal_request),
        )
        .with_state(AppRechargeCheckoutState {
            store,
            fulfillment_store,
            coupon_redemption_port,
            account_value_ledger_port,
            membership_fulfillment_port,
            orders,
            payments,
        })
}

async fn list_token_bank_plans(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Query(params): Query<RechargePackageListQueryParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let page_params = match parse_offset_list_params_validated(ctx, params.page, params.page_size) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let query = match AccountValueCatalogListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        Some(AccountValueAssetCode::TokenBank),
        Some("active"),
        Some(page_params.page),
        Some(page_params.page_size),
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
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

async fn list_refund_requests(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Query(params): Query<AccountValueRequestListQueryParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let page_params = match parse_offset_list_params_validated(ctx, params.page, params.page_size) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let query = match AccountValueRequestListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        Some(&subject.user_id),
        Some(AccountValueOrderSubject::RefundRequest),
        params.status.as_deref(),
        Some(page_params.page),
        Some(page_params.page_size),
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };
    match state.store.list_order_refund_requests(query).await {
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

async fn create_refund_request(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(body): Json<RefundRequestCreateBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let write_headers = match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
        format!("refund-request-{}-{idempotency_key}", subject.user_id)
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let target_asset = match body
        .target_asset
        .as_deref()
        .ok_or_else(|| "targetAsset is required".to_string())
        .and_then(|value| {
            AccountValueAssetCode::parse(value).map_err(|error| error.message().to_string())
        }) {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let amount = match validate_positive_money_amount(body.amount.as_ref(), "refund amount") {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let currency_code = match validate_currency_code(body.currency_code.as_deref()) {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let original_order_id =
        match required_text(body.original_order_id.as_deref(), "originalOrderId") {
            Ok(value) => value,
            Err(message) => return validation(ctx, message),
        };
    let refund_request_id = stable_storage_id(&[
        "refund",
        &subject.tenant_id,
        &subject.user_id,
        &write_headers.idempotency_key,
    ]);
    let mut command = match CreateOrderRefundRequestCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &refund_request_id,
        &original_order_id,
        target_asset,
        amount,
        &currency_code,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };
    command.request_no = write_headers.request_no;
    command.reason_code = optional_string(body.reason_code.as_deref());
    command.reason_detail = optional_string(body.reason_detail.as_deref());
    match state.store.create_order_refund_request(command).await {
        Ok(view) => success_created_item(ctx, map_account_value_request(view)),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn retrieve_refund_request(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(refund_request_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match AccountValueRequestDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        Some(&subject.user_id),
        Some(AccountValueOrderSubject::RefundRequest),
        &refund_request_id,
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };
    match state.store.retrieve_order_refund_request(query).await {
        Ok(Some(view)) => success_item(ctx, map_account_value_request(view)),
        Ok(None) => not_found(ctx, "refund request was not found"),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn create_withdrawal_request(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(body): Json<WithdrawalRequestCreateBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let write_headers = match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
        format!("withdrawal-request-{}-{idempotency_key}", subject.user_id)
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let asset = match body
        .asset
        .as_deref()
        .unwrap_or(AccountValueAssetCode::Cash.as_str())
        .parse::<String>()
        .map_err(|_| "asset is invalid".to_string())
        .and_then(|value| {
            AccountValueAssetCode::parse(&value).map_err(|error| error.message().to_string())
        }) {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let amount = match validate_positive_money_amount(body.amount.as_ref(), "withdrawal amount") {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let currency_code = match validate_currency_code(body.currency_code.as_deref()) {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let withdrawal_request_id = stable_storage_id(&[
        "withdrawal",
        &subject.tenant_id,
        &subject.user_id,
        &write_headers.idempotency_key,
    ]);
    let mut command = match CreateCashWithdrawalRequestCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &withdrawal_request_id,
        asset,
        amount,
        &currency_code,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };
    command.request_no = write_headers.request_no;
    command.payout_method = optional_string(body.payout_method.as_deref());
    command.payout_account_ref = optional_string(body.payout_account_ref.as_deref());
    command.reason_code = optional_string(body.reason_code.as_deref());
    match state.store.create_cash_withdrawal_request(command).await {
        Ok(view) => success_created_item(ctx, map_account_value_request(view)),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn retrieve_withdrawal_request(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(withdrawal_request_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match AccountValueRequestDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        Some(&subject.user_id),
        Some(AccountValueOrderSubject::CashWithdrawal),
        &withdrawal_request_id,
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };
    match state.store.retrieve_cash_withdrawal_request(query).await {
        Ok(Some(view)) => success_item(ctx, map_account_value_request(view)),
        Ok(None) => not_found(ctx, "withdrawal request was not found"),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn fetch_recharge_packages(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Query(params): Query<RechargePackageListQueryParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match RechargePackageListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        params.page,
        params.page_size,
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.list_recharge_packages(query.clone()).await {
        Ok(page) => {
            let mapped = page
                .items
                .into_iter()
                .map(map_recharge_package)
                .collect::<Vec<_>>();
            let page_params = offset_list_page_params_from_query(query.page, query.page_size);
            success_items(ctx, mapped, page.total, page_params)
        }
        Err(error) => map_service_error(ctx, error),
    }
}

async fn fetch_recharge_settings(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query =
        match RechargeSettingsQuery::new(&subject.tenant_id, subject.organization_id.as_deref()) {
            Ok(query) => query,
            Err(error) => return map_service_error(ctx, error),
        };

    match state.store.load_recharge_settings(query).await {
        Ok(settings) => success_item(ctx, map_recharge_settings(settings)),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn list_recharge_orders(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Query(params): Query<RechargeOrderListQueryParams>,
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
        params.subject.as_deref().or(Some("points_recharge")),
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.orders.list_owner_orders(query).await {
        Ok(page) => {
            let page_params = offset_list_page_params_from_query(page.page, page.page_size);
            success_items(
                ctx,
                page.items
                    .into_iter()
                    .map(|item| RechargeOrderSummaryResponse {
                        order_id: item.order_id,
                        order_no: item.order_sn,
                        status: item.status,
                        subject: item.subject,
                        amount: item.total_amount.as_str().to_string(),
                        points: item.points.unwrap_or(0),
                        created_at: item.created_at,
                    })
                    .collect(),
                page.total,
                page_params,
            )
        }
        Err(error) => map_service_error(ctx, error),
    }
}

async fn cancel_recharge_order(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    body: Option<Json<RechargeCancelRequest>>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let body = body.map(|Json(value)| value).unwrap_or_default();
    let _write_headers =
        match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("recharge-cancel-{order_id}-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let cancel_reason = body
        .cancel_reason
        .as_deref()
        .or(Some("recharge cancelled by owner"));
    let command = match CancelOwnerOrderCommand::with_cancel_type(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_id,
        cancel_reason,
        body.cancel_type.as_deref(),
    ) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match cancel_owner_order_with_payments(&*state.orders, &*state.payments, command).await {
        Ok(()) => success_command(ctx, Some(order_id.clone()), Some("cancelled".to_string())),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn submit_recharge(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(request): Json<SubmitRechargeRequest>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let order_subject =
        match AccountValueOrderSubject::parse(request.subject().unwrap_or("points_recharge")) {
            Ok(value) => value,
            Err(error) => return map_service_error(ctx, error),
        };
    if matches!(
        order_subject,
        AccountValueOrderSubject::RefundRequest | AccountValueOrderSubject::CashWithdrawal
    ) {
        return validation(
            ctx,
            "refund_request and cash_withdrawal are managed by their dedicated request APIs",
        );
    }
    let amount = if matches!(order_subject, AccountValueOrderSubject::CouponRecharge) {
        match validate_non_negative_money_amount(request.amount_value(), "recharge amount") {
            Ok(amount) => amount,
            Err(message) => return validation(ctx, message),
        }
    } else {
        match validate_recharge_amount(request.amount_value()) {
            Ok(amount) => amount,
            Err(message) => return validation(ctx, message),
        }
    };
    let currency_code = match validate_currency_code(request.currency_code()) {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let payment_required = order_subject.requires_payment_collection()
        || (order_subject.payment_collection_is_optional() && amount.as_str() != "0");
    let method = if payment_required {
        match validate_payment_method(request.payment_method()) {
            Ok(value) => Some(value),
            Err(message) => return validation(ctx, message),
        }
    } else {
        match request
            .payment_method()
            .map(|value| validate_payment_method(Some(value)))
            .transpose()
        {
            Ok(value) => value,
            Err(message) => return validation(ctx, message),
        }
    };
    let write_headers = match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
        fallback_account_value_request_no(
            &subject,
            order_subject,
            amount.as_str(),
            method.as_deref(),
            idempotency_key,
        )
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };

    if matches!(order_subject, AccountValueOrderSubject::PointsRecharge) {
        let Some(method) = method.as_deref() else {
            return validation(ctx, "payment method must be provided");
        };
        let payment_product = match validate_payment_product(request.payment_product()) {
            Ok(value) => value,
            Err(message) => return validation(ctx, message),
        };
        if let Err(message) = validate_payment_method_for_product(method, &payment_product) {
            return validation(ctx, message);
        }
        return submit_points_recharge_order(
            state,
            ctx,
            &subject,
            &request,
            amount,
            &currency_code,
            method,
            &payment_product,
            &write_headers.request_no,
            &write_headers.idempotency_key,
        )
        .await;
    }

    let target_asset =
        match resolve_account_value_target_asset(order_subject, request.target_asset()) {
            Ok(value) => value,
            Err(error) => return map_service_error(ctx, error),
        };
    let (grant_amount, coupon_benefit) =
        if matches!(order_subject, AccountValueOrderSubject::CouponRecharge) {
            if request.grant_amount_value().is_some() {
                return validation(ctx, "coupon recharge grantAmount is server-controlled");
            }
            let coupon_code = match request
                .coupon_code()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                Some(value) => value,
                None => return validation(ctx, "coupon recharge requires couponCode"),
            };
            let preview = state
                .coupon_redemption_port
                .preview_coupon(CouponRedemptionRequest {
                    tenant_id: subject.tenant_id.clone(),
                    organization_id: subject.organization_id.clone(),
                    owner_user_id: subject.user_id.clone(),
                    coupon_code: coupon_code.to_owned(),
                    order_id: write_headers.request_no.clone(),
                    request_no: write_headers.request_no.clone(),
                    idempotency_key: format!(
                        "coupon-recharge:preview:{}",
                        write_headers.idempotency_key
                    ),
                })
                .await;
            match preview {
                Ok(value) if value.accepted => match value.benefit {
                    benefit @ CouponRedemptionBenefit::TokenBankCredit { .. }
                    | benefit @ CouponRedemptionBenefit::PointsCredit { .. }
                    | benefit @ CouponRedemptionBenefit::CashCredit { .. } => {
                        (benefit.grant_amount(), Some(benefit))
                    }
                    CouponRedemptionBenefit::Subscription { .. } => {
                        return map_service_error(
                            ctx,
                            CommerceServiceError::conflict(
                                "subscription coupons must use the coupon redemption API",
                            ),
                        )
                    }
                },
                Ok(_) => {
                    return map_service_error(
                        ctx,
                        CommerceServiceError::conflict(
                            "coupon does not grant a supported asset benefit",
                        ),
                    )
                }
                Err(error) => return map_service_error(ctx, error),
            }
        } else {
            match resolve_grant_amount(order_subject, &amount, request.grant_amount_value()) {
                Ok(value) => (value, None),
                Err(error) => return map_service_error(ctx, error),
            }
        };
    let plan_period = match request
        .plan_period()
        .map(TokenBankPlanPeriod::parse)
        .transpose()
    {
        Ok(value) => value,
        Err(error) => return map_service_error(ctx, error),
    };

    if matches!(order_subject, AccountValueOrderSubject::CouponRecharge) {
        let command = match build_create_coupon_recharge_command(
            CreateAccountRechargeCommandInput {
                subject: &subject,
                order_subject,
                target_asset,
                amount,
                grant_amount,
                currency_code: &currency_code,
                method: method.as_deref(),
                request_no: &write_headers.request_no,
                idempotency_key: &write_headers.idempotency_key,
                package_id: request.package_id(),
                plan_code: request.plan_code(),
                plan_period,
                coupon_code: request.coupon_code(),
                client_request_no: request.client_request_no(),
            },
            coupon_benefit.expect("coupon preview benefit must be available"),
        ) {
            Ok(command) => command,
            Err(error) => return map_service_error(ctx, error),
        };
        let command_for_payment = command.clone();
        return match state.store.create_coupon_recharge_order(command).await {
            Ok(outcome) if command_for_payment.payment_required => {
                let Some(method) = method.as_deref() else {
                    return validation(ctx, "payment method must be provided");
                };
                pay_coupon_recharge_order(
                    state,
                    ctx,
                    &subject,
                    &command_for_payment,
                    method,
                    request.payment_password(),
                    &write_headers.request_no,
                    &write_headers.idempotency_key,
                    outcome,
                )
                .await
            }
            Ok(mut outcome) => {
                let fulfill_command = match default_fulfill_account_value_order_command(
                    AccountValueOrderSubject::CouponRecharge,
                    &subject.tenant_id,
                    subject.organization_id.as_deref(),
                    &subject.user_id,
                    &outcome.order_id,
                    &write_headers.request_no,
                ) {
                    Ok(value) => value,
                    Err(error) => return map_service_error(ctx, error),
                };
                match redeem_coupon_and_fulfill_account_value_order(
                    &*state.fulfillment_store,
                    &*state.coupon_redemption_port,
                    &*state.account_value_ledger_port,
                    fulfill_command,
                )
                .await
                {
                    Ok(fulfillment) => {
                        outcome.status = fulfillment.fulfillment_status;
                        outcome.next_action = "completed".to_owned();
                        success_created_item(ctx, map_account_recharge_outcome(outcome))
                    }
                    Err(error) => map_service_error(ctx, error),
                }
            }
            Err(error) => map_service_error(ctx, error),
        };
    }

    let command = match build_create_account_recharge_command(CreateAccountRechargeCommandInput {
        subject: &subject,
        order_subject,
        target_asset,
        amount,
        grant_amount,
        currency_code: &currency_code,
        method: method.as_deref(),
        request_no: &write_headers.request_no,
        idempotency_key: &write_headers.idempotency_key,
        package_id: request.package_id(),
        plan_code: request.plan_code(),
        plan_period,
        coupon_code: request.coupon_code(),
        client_request_no: request.client_request_no(),
    }) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };
    let command_for_payment = command.clone();
    match state.store.create_account_recharge_order(command).await {
        Ok(outcome) => {
            let Some(method) = method.as_deref() else {
                return validation(ctx, "payment method must be provided");
            };
            pay_account_value_order(
                state,
                ctx,
                &subject,
                &command_for_payment,
                method,
                request.payment_password(),
                &write_headers.request_no,
                &write_headers.idempotency_key,
                outcome,
            )
            .await
        }
        Err(error) => map_service_error(ctx, error),
    }
}

async fn create_coupon_redemption(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(request): Json<CouponRedemptionCreateBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let coupon_code = request.coupon_code.trim();
    if coupon_code.is_empty() {
        return validation(ctx, "couponCode is required");
    }
    let zero = CommerceMoney::new("0").expect("zero must be valid commerce money");
    let write_headers = match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
        fallback_account_value_request_no(
            &subject,
            AccountValueOrderSubject::CouponRecharge,
            zero.as_str(),
            None,
            idempotency_key,
        )
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let existing_order = match state
        .store
        .load_account_value_order_by_idempotency(
            &subject.tenant_id,
            subject.organization_id.as_deref(),
            &subject.user_id,
            &write_headers.idempotency_key,
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return map_service_error(ctx, error),
    };
    if let Some(existing_order) = existing_order {
        let fulfill_command = match default_fulfill_account_value_order_command(
            AccountValueOrderSubject::CouponRecharge,
            &subject.tenant_id,
            subject.organization_id.as_deref(),
            &subject.user_id,
            &existing_order.order_id,
            &write_headers.request_no,
        ) {
            Ok(value) => value,
            Err(error) => return map_service_error(ctx, error),
        };
        let context = match state
            .fulfillment_store
            .load_account_value_fulfillment_context(&fulfill_command)
            .await
        {
            Ok(Some(value)) => value,
            Ok(None) => {
                return map_service_error(
                    ctx,
                    CommerceServiceError::not_found("coupon redemption order was not found"),
                )
            }
            Err(error) => return map_service_error(ctx, error),
        };
        if let Err(error) = validate_coupon_redemption_replay(
            existing_order.subject,
            context.coupon_code.as_deref(),
            coupon_code,
        ) {
            return map_service_error(ctx, error);
        }
        return fulfill_coupon_redemption_order(&state, ctx, fulfill_command).await;
    }
    let preview = match state
        .coupon_redemption_port
        .preview_coupon(CouponRedemptionRequest {
            tenant_id: subject.tenant_id.clone(),
            organization_id: subject.organization_id.clone(),
            owner_user_id: subject.user_id.clone(),
            coupon_code: coupon_code.to_owned(),
            order_id: write_headers.request_no.clone(),
            request_no: write_headers.request_no.clone(),
            idempotency_key: format!(
                "coupon-redemption:preview:{}",
                write_headers.idempotency_key
            ),
        })
        .await
    {
        Ok(value) if value.accepted => value,
        Ok(_) => {
            return map_service_error(
                ctx,
                CommerceServiceError::conflict("coupon redemption preview was rejected"),
            )
        }
        Err(error) => return map_service_error(ctx, error),
    };
    let benefit = preview.benefit;
    let target_asset = benefit.target_asset();
    let grant_amount = benefit.grant_amount();
    let command = match build_create_coupon_recharge_command(
        CreateAccountRechargeCommandInput {
            subject: &subject,
            order_subject: AccountValueOrderSubject::CouponRecharge,
            target_asset,
            amount: zero,
            grant_amount,
            currency_code: "CNY",
            method: None,
            request_no: &write_headers.request_no,
            idempotency_key: &write_headers.idempotency_key,
            package_id: None,
            plan_code: None,
            plan_period: None,
            coupon_code: Some(coupon_code),
            client_request_no: None,
        },
        benefit,
    ) {
        Ok(value) => value,
        Err(error) => return map_service_error(ctx, error),
    };
    let order_id = command.order_id.clone();
    match state.store.create_coupon_recharge_order(command).await {
        Ok(_) => {}
        Err(error) => return map_service_error(ctx, error),
    }
    let fulfill_command = match default_fulfill_account_value_order_command(
        AccountValueOrderSubject::CouponRecharge,
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_id,
        &write_headers.request_no,
    ) {
        Ok(value) => value,
        Err(error) => return map_service_error(ctx, error),
    };
    fulfill_coupon_redemption_order(&state, ctx, fulfill_command).await
}

fn validate_coupon_redemption_replay(
    existing_subject: AccountValueOrderSubject,
    stored_coupon_code: Option<&str>,
    requested_coupon_code: &str,
) -> Result<(), CommerceServiceError> {
    if existing_subject != AccountValueOrderSubject::CouponRecharge {
        return Err(CommerceServiceError::conflict(
            "idempotency key is already used by a different order subject",
        ));
    }
    if stored_coupon_code.map(str::trim) != Some(requested_coupon_code.trim()) {
        return Err(CommerceServiceError::conflict(
            "idempotency key was replayed with a different coupon code",
        ));
    }
    Ok(())
}

async fn fulfill_coupon_redemption_order(
    state: &AppRechargeCheckoutState,
    ctx: Option<&WebRequestContext>,
    command: FulfillAccountValueOrderCommand,
) -> Response {
    match redeem_coupon_and_fulfill_order(
        &*state.fulfillment_store,
        &*state.coupon_redemption_port,
        &*state.account_value_ledger_port,
        &*state.membership_fulfillment_port,
        command,
    )
    .await
    {
        Ok(outcome) => success_created_item(
            ctx,
            CouponRedemptionResponse {
                order_id: outcome.order_id,
                order_no: outcome.order_no,
                status: outcome.fulfillment_status,
                replayed: outcome.replayed,
                benefit: map_coupon_fulfilled_benefit(outcome.benefit),
            },
        ),
        Err(error) => map_service_error(ctx, error),
    }
}

fn map_coupon_fulfilled_benefit(
    benefit: sdkwork_order_service::CouponFulfilledBenefit,
) -> serde_json::Value {
    match benefit {
        sdkwork_order_service::CouponFulfilledBenefit::TokenBankCredit { grant_amount } => {
            serde_json::json!({
                "kind": "token_bank_credit",
                "targetAsset": "token_bank",
                "grantAmount": grant_amount.as_str(),
            })
        }
        sdkwork_order_service::CouponFulfilledBenefit::PointsCredit { grant_amount } => {
            serde_json::json!({
                "kind": "points_credit",
                "grantPoints": grant_amount.as_str(),
            })
        }
        sdkwork_order_service::CouponFulfilledBenefit::CashCredit { grant_amount } => {
            serde_json::json!({
                "kind": "cash_credit",
                "grantAmount": grant_amount.as_str(),
            })
        }
        sdkwork_order_service::CouponFulfilledBenefit::Subscription {
            product_id,
            sku_id,
            package_id,
            period,
            duration_days,
            daily_quota,
            total_quota,
            subscription_id,
            starts_at,
            expires_at,
        } => serde_json::json!({
            "kind": "subscription",
            "productId": product_id,
            "skuId": sku_id,
            "packageId": package_id.to_string(),
            "period": period,
            "durationDays": duration_days,
            "dailyQuota": daily_quota.to_string(),
            "totalQuota": total_quota.to_string(),
            "subscriptionId": subscription_id,
            "startsAt": starts_at,
            "expiresAt": expires_at,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
async fn pay_coupon_recharge_order(
    state: AppRechargeCheckoutState,
    ctx: Option<&WebRequestContext>,
    subject: &AppRuntimeSubject,
    command: &CreateCouponRechargeOrderCommand,
    method: &str,
    payment_password: Option<&str>,
    request_no: &str,
    idempotency_key: &str,
    outcome: CreateAccountRechargeOrderOutcome,
) -> Response {
    let callback_payload = coupon_recharge_callback_payload(command, payment_password);
    let persisted_order_id = outcome.order_id.clone();
    let persisted_organization_id =
        points_recharge_organization_scope(subject.organization_id.as_deref());
    let pay_command = match PayOwnerOrderCommand::new(PayOwnerOrderCommandInput {
        tenant_id: subject.tenant_id.clone(),
        organization_id: Some(persisted_organization_id.clone()),
        owner_user_id: subject.user_id.clone(),
        order_id: persisted_order_id.clone(),
        payment_method: method.to_owned(),
        payment_scene: None,
        payment_attempt_callback_payload: Some(callback_payload),
        payment_metadata: serde_json::json!({}),
        request_no: format!("{request_no}:pay"),
        idempotency_key: format!("{idempotency_key}:pay"),
    }) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };
    match state.payments.pay_owner_order(pay_command).await {
        Ok(pay_outcome) => success_created_item(
            ctx,
            map_account_recharge_outcome(merge_account_recharge_pay_outcome(outcome, pay_outcome)),
        ),
        Err(error) => {
            let rollback = CancelOwnerOrderCommand::new(
                &subject.tenant_id,
                subject.organization_id.as_deref(),
                &subject.user_id,
                &persisted_order_id,
                Some("auto-cancel: coupon recharge payment initiation failed"),
            );
            if let Ok(rollback_command) = rollback {
                compensate_failed_recharge_pay(&*state.orders, &*state.payments, rollback_command)
                    .await;
            }
            map_service_error(ctx, error)
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn submit_points_recharge_order(
    state: AppRechargeCheckoutState,
    ctx: Option<&WebRequestContext>,
    subject: &AppRuntimeSubject,
    request: &SubmitRechargeRequest,
    amount: CommerceMoney,
    currency_code: &str,
    method: &str,
    payment_product: &str,
    request_no: &str,
    idempotency_key: &str,
) -> Response {
    let command = match build_create_recharge_command(CreateRechargeCommandInput {
        subject,
        amount,
        currency_code,
        method,
        request_no,
        idempotency_key,
        package_id: request.package_id(),
        client_request_no: request.client_request_no(),
        source: request.source(),
    }) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    match state
        .store
        .create_points_recharge_order(command.clone())
        .await
    {
        Ok(mut outcome) => {
            if !payment_product_requires_provider_route(payment_product) {
                outcome.payment_product = payment_product.to_owned();
                outcome.next_action = "cashier".to_owned();
                outcome.qr_code_payload = outcome.cashier_url.clone();
                return success_created_item(ctx, map_recharge_outcome(outcome));
            }

            outcome.payment_product = payment_product.to_owned();
            let persisted_order_id = outcome.order_id.clone();
            let persisted_organization_id =
                points_recharge_organization_scope(subject.organization_id.as_deref());
            let callback_payload = serde_json::json!({
                "points": outcome.points,
                "packageId": command.package_id,
                "clientRequestNo": command.client_request_no,
                "source": command.source,
                "paymentPassword": request.payment_password(),
            })
            .to_string();
            let pay_command = match PayOwnerOrderCommand::new(PayOwnerOrderCommandInput {
                tenant_id: subject.tenant_id.clone(),
                organization_id: Some(persisted_organization_id.clone()),
                owner_user_id: subject.user_id.clone(),
                order_id: persisted_order_id.clone(),
                payment_method: method.to_owned(),
                payment_scene: Some(payment_scene(payment_product).to_owned()),
                payment_attempt_callback_payload: Some(callback_payload),
                payment_metadata: serde_json::json!({
                    "paymentProduct": payment_product,
                }),
                request_no: format!("{request_no}:pay"),
                idempotency_key: format!("{idempotency_key}:pay"),
            }) {
                Ok(command) => command,
                Err(error) => return map_service_error(ctx, error),
            };
            match state.payments.pay_owner_order(pay_command).await {
                Ok(pay_outcome) => {
                    outcome = merge_recharge_pay_outcome(outcome, pay_outcome, payment_product);
                    success_created_item(ctx, map_recharge_outcome(outcome))
                }
                Err(error) => {
                    let rollback = CancelOwnerOrderCommand::new(
                        &subject.tenant_id,
                        Some(&persisted_organization_id),
                        &subject.user_id,
                        &persisted_order_id,
                        Some("auto-cancel: recharge payment initiation failed"),
                    );
                    if let Ok(rollback_command) = rollback {
                        compensate_failed_recharge_pay(
                            &*state.orders,
                            &*state.payments,
                            rollback_command,
                        )
                        .await;
                    }
                    map_service_error(ctx, error)
                }
            }
        }
        Err(error) => map_service_error(ctx, error),
    }
}

#[allow(clippy::too_many_arguments)]
async fn pay_account_value_order(
    state: AppRechargeCheckoutState,
    ctx: Option<&WebRequestContext>,
    subject: &AppRuntimeSubject,
    command: &CreateAccountRechargeOrderCommand,
    method: &str,
    payment_password: Option<&str>,
    request_no: &str,
    idempotency_key: &str,
    outcome: CreateAccountRechargeOrderOutcome,
) -> Response {
    let callback_payload = account_value_callback_payload(command, payment_password);
    let persisted_order_id = outcome.order_id.clone();
    let persisted_organization_id =
        points_recharge_organization_scope(subject.organization_id.as_deref());
    let pay_command = match PayOwnerOrderCommand::new(PayOwnerOrderCommandInput {
        tenant_id: subject.tenant_id.clone(),
        organization_id: Some(persisted_organization_id.clone()),
        owner_user_id: subject.user_id.clone(),
        order_id: persisted_order_id.clone(),
        payment_method: method.to_owned(),
        payment_scene: None,
        payment_attempt_callback_payload: Some(callback_payload),
        payment_metadata: serde_json::json!({}),
        request_no: format!("{request_no}:pay"),
        idempotency_key: format!("{idempotency_key}:pay"),
    }) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };
    match state.payments.pay_owner_order(pay_command).await {
        Ok(pay_outcome) => success_created_item(
            ctx,
            map_account_recharge_outcome(merge_account_recharge_pay_outcome(outcome, pay_outcome)),
        ),
        Err(error) => {
            let rollback = CancelOwnerOrderCommand::new(
                &subject.tenant_id,
                subject.organization_id.as_deref(),
                &subject.user_id,
                &persisted_order_id,
                Some("auto-cancel: account value payment initiation failed"),
            );
            if let Ok(rollback_command) = rollback {
                compensate_failed_recharge_pay(&*state.orders, &*state.payments, rollback_command)
                    .await;
            }
            map_service_error(ctx, error)
        }
    }
}

fn points_recharge_organization_scope(organization_id: Option<&str>) -> String {
    organization_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(PLATFORM_ORGANIZATION_SCOPE_SENTINEL)
        .to_owned()
}

async fn fetch_checkout_status(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(order_lookup): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let order_lookup = match validate_recharge_order_lookup_key(order_lookup) {
        Ok(order_lookup) => order_lookup,
        Err(message) => return validation(ctx, message),
    };
    let query = match CheckoutStatusQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_lookup,
    ) {
        Ok(query) => query,
        Err(error) => return map_service_error(ctx, error),
    };

    match state.store.retrieve_checkout_status(query).await {
        Ok(Some(snapshot)) => success_item(ctx, map_checkout_status(snapshot)),
        Ok(None) => not_found(ctx, "checkout order was not found"),
        Err(error) => map_service_error(ctx, error),
    }
}

fn validate_recharge_amount(value: Option<&serde_json::Value>) -> Result<CommerceMoney, String> {
    validate_money_amount(value, "recharge amount", true)
}

fn validate_positive_money_amount(
    value: Option<&serde_json::Value>,
    field_name: &str,
) -> Result<CommerceMoney, String> {
    validate_money_amount(value, field_name, true)
}

fn validate_non_negative_money_amount(
    value: Option<&serde_json::Value>,
    field_name: &str,
) -> Result<CommerceMoney, String> {
    validate_money_amount(value, field_name, false)
}

fn validate_money_amount(
    value: Option<&serde_json::Value>,
    field_name: &str,
    require_positive: bool,
) -> Result<CommerceMoney, String> {
    let Some(value) = value else {
        return Err(format!("{field_name} must be provided"));
    };
    let raw = match value {
        serde_json::Value::String(value) => value.trim().to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        _ => return Err(format!("{field_name} must be a decimal amount")),
    };
    let cents = money_cents(&raw).map_err(|_| format!("{field_name} must be a decimal amount"))?;
    if require_positive && cents <= 0 {
        return Err(format!("{field_name} must be greater than zero"));
    }
    if !require_positive && cents < 0 {
        return Err(format!("{field_name} must be non-negative"));
    }
    if cents > MAX_RECHARGE_CENTS {
        return Err(format!("{field_name} must not exceed 10000.00"));
    }
    CommerceMoney::new(&cents.to_string()).map_err(str::to_string)
}

fn validate_currency_code(value: Option<&str>) -> Result<String, String> {
    let currency_code = value.unwrap_or_default().trim().to_ascii_uppercase();
    if currency_code.len() != 3
        || !currency_code
            .chars()
            .all(|character| character.is_ascii_uppercase())
    {
        return Err("currency code must be a 3-letter uppercase code".to_string());
    }
    Ok(currency_code)
}

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

fn validate_payment_method_for_product(
    payment_method: &str,
    payment_product: &str,
) -> Result<(), String> {
    let expected = match payment_product {
        "wechat_native" => Some("wechat_pay"),
        "alipay_native" => Some("alipay"),
        _ => None,
    };
    if expected.is_some_and(|expected| expected != payment_method) {
        return Err(format!(
            "payment product {payment_product} requires payment method {}",
            expected.unwrap_or_default()
        ));
    }
    Ok(())
}

fn payment_scene(payment_product: &str) -> &str {
    match payment_product {
        "wechat_native" => "wechat_native",
        "alipay_native" => "alipay_qr",
        _ => DEFAULT_PAYMENT_PRODUCT,
    }
}

fn payment_product_requires_provider_route(payment_product: &str) -> bool {
    payment_product != DEFAULT_PAYMENT_PRODUCT
}

fn validate_recharge_order_lookup_key(order_lookup: String) -> Result<String, String> {
    let order_lookup = order_lookup.trim().to_string();
    if order_lookup.is_empty() {
        return Err("order id must not be empty".to_string());
    }
    if order_lookup.chars().count() > MAX_CHECKOUT_ORDER_NO_LEN {
        return Err(format!(
            "order id length must not exceed {MAX_CHECKOUT_ORDER_NO_LEN} characters"
        ));
    }
    if !order_lookup
        .bytes()
        .all(|byte| (0x21..=0x7e).contains(&byte))
    {
        return Err("order id must contain only visible ASCII characters".to_string());
    }
    Ok(order_lookup)
}

fn build_create_recharge_command(
    input: CreateRechargeCommandInput<'_>,
) -> Result<CreatePointsRechargeOrderCommand, CommerceServiceError> {
    let requested_at_instant = now();
    let requested_at = format_datetime(requested_at_instant, None);
    let expire_at = format_datetime(
        add_minutes(requested_at_instant, payment_expire_minutes()),
        None,
    );
    let seed = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        input.subject.tenant_id,
        input.subject.organization_id.as_deref().unwrap_or(""),
        input.subject.user_id,
        input.amount.as_str(),
        input.method,
        input.request_no,
        input.idempotency_key,
    );
    let token = stable_hex_token(&seed);
    let order_no = format!("RC{}", token);
    let out_trade_no = format!("RECHARGE{}", token);

    CreatePointsRechargeOrderCommand::new(
        &input.subject.tenant_id,
        input.subject.organization_id.as_deref(),
        &input.subject.user_id,
        input.amount,
        input.currency_code,
        input.method,
        &format!("order-{token}"),
        &format!("order-item-{token}"),
        &format!("payment-intent-{token}"),
        &format!("payment-attempt-{token}"),
        &order_no,
        &out_trade_no,
        &requested_at,
        &expire_at,
        input.idempotency_key,
        input.package_id,
        input.client_request_no,
        input.source,
    )
}

fn build_create_account_recharge_command(
    input: CreateAccountRechargeCommandInput<'_>,
) -> Result<CreateAccountRechargeOrderCommand, CommerceServiceError> {
    if matches!(
        input.order_subject,
        AccountValueOrderSubject::PointsRecharge
            | AccountValueOrderSubject::CouponRecharge
            | AccountValueOrderSubject::RefundRequest
            | AccountValueOrderSubject::CashWithdrawal
    ) {
        return Err(CommerceServiceError::validation(
            "account recharge command requires a paid account value order subject",
        ));
    }
    let requested_at_instant = now();
    let requested_at = format_datetime(requested_at_instant, None);
    let expire_at = format_datetime(
        add_minutes(requested_at_instant, payment_expire_minutes()),
        None,
    );
    let seed = account_value_command_seed(&input);
    let token = stable_hex_token(&seed);
    let order_no = format!("AV{}", token);
    let out_trade_no = format!("ACCOUNT{}", token);
    let plan = match (input.plan_code, input.plan_period) {
        (Some(plan_code), Some(period)) => Some((plan_code, period)),
        _ => None,
    };
    let mut command = CreateAccountRechargeOrderCommand::new(
        &input.subject.tenant_id,
        input.subject.organization_id.as_deref(),
        &input.subject.user_id,
        input.order_subject,
        input.target_asset,
        input.amount,
        input.currency_code,
        &format!("order-{token}"),
        &format!("order-item-{token}"),
        &order_no,
        &out_trade_no,
        &requested_at,
        &expire_at,
        input.idempotency_key,
        input.package_id,
        plan,
        input.client_request_no,
    )?;
    command.grant_amount = input.grant_amount;
    Ok(command)
}

fn build_create_coupon_recharge_command(
    input: CreateAccountRechargeCommandInput<'_>,
    benefit: CouponRedemptionBenefit,
) -> Result<CreateCouponRechargeOrderCommand, CommerceServiceError> {
    if !matches!(
        input.order_subject,
        AccountValueOrderSubject::CouponRecharge
    ) {
        return Err(CommerceServiceError::validation(
            "coupon recharge command requires subject coupon_recharge",
        ));
    }
    let coupon_code = input
        .coupon_code
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CommerceServiceError::validation("coupon recharge requires couponCode"))?;
    if input.grant_amount.as_str() == "0" {
        return Err(CommerceServiceError::validation(
            "coupon recharge requires positive grantAmount",
        ));
    }
    let seed = account_value_command_seed(&input);
    let token = stable_hex_token(&seed);
    let order_no = format!("CP{}", token);
    let out_trade_no = format!("COUPON{}", token);
    let payment_required = input.amount.as_str() != "0" || input.method.is_some();
    let mut command = CreateCouponRechargeOrderCommand::new_with_benefit(
        &input.subject.tenant_id,
        input.subject.organization_id.as_deref(),
        &input.subject.user_id,
        input.target_asset,
        input.amount,
        input.currency_code,
        &format!("order-{token}"),
        &format!("order-item-{token}"),
        &order_no,
        &out_trade_no,
        coupon_code,
        input.idempotency_key,
        payment_required,
        benefit,
    )?;
    command.grant_amount = input.grant_amount;
    Ok(command)
}

fn account_value_command_seed(input: &CreateAccountRechargeCommandInput<'_>) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        input.subject.tenant_id,
        input.subject.organization_id.as_deref().unwrap_or(""),
        input.subject.user_id,
        input.order_subject.as_str(),
        input.target_asset.as_str(),
        input.amount.as_str(),
        input.grant_amount.as_str(),
        input.currency_code,
        input.method.unwrap_or(""),
        input.package_id.unwrap_or(""),
        input.plan_code.unwrap_or(""),
        input.coupon_code.unwrap_or(""),
        input.request_no,
        input.idempotency_key,
    )
}

fn resolve_account_value_target_asset(
    subject: AccountValueOrderSubject,
    target_asset: Option<&str>,
) -> Result<AccountValueAssetCode, CommerceServiceError> {
    let asset = match subject.fixed_target_asset() {
        Some(fixed) => {
            if let Some(value) = target_asset {
                let requested = AccountValueAssetCode::parse(value)?;
                subject.validate_target_asset(requested)?;
            }
            fixed
        }
        None => {
            let value = target_asset
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    CommerceServiceError::validation("account value recharge requires targetAsset")
                })?;
            AccountValueAssetCode::parse(value)?
        }
    };
    subject.validate_target_asset(asset)?;
    Ok(asset)
}

fn resolve_grant_amount(
    subject: AccountValueOrderSubject,
    amount: &CommerceMoney,
    grant_amount: Option<&serde_json::Value>,
) -> Result<CommerceMoney, CommerceServiceError> {
    if matches!(subject, AccountValueOrderSubject::CouponRecharge) {
        return Err(CommerceServiceError::validation(
            "coupon recharge grantAmount is server-controlled",
        ));
    }
    if let Some(value) = grant_amount {
        return validate_positive_money_amount(Some(value), "grantAmount")
            .map_err(CommerceServiceError::validation);
    }
    if matches!(
        subject,
        AccountValueOrderSubject::TokenBankRecharge
            | AccountValueOrderSubject::TokenBankPlanPurchase
            | AccountValueOrderSubject::TokenBankPlanRenewal
            | AccountValueOrderSubject::AccountRechargePackage
    ) {
        return Err(CommerceServiceError::validation(
            "account value recharge requires grantAmount",
        ));
    }
    Ok(amount.clone())
}

fn account_value_callback_payload(
    command: &CreateAccountRechargeOrderCommand,
    payment_password: Option<&str>,
) -> String {
    serde_json::json!({
        "subject": command.subject.as_str(),
        "targetAsset": command.target_asset.as_str(),
        "assetCode": command.target_asset.as_str(),
        "assetUnitCode": command.target_asset.default_unit_code(),
        "grantAmount": command.grant_amount.as_str(),
        "packageId": command.package_id,
        "planCode": command.plan_code,
        "planPeriod": command.plan_period.map(TokenBankPlanPeriod::as_str),
        "clientRequestNo": command.client_request_no,
        "paymentPassword": payment_password,
    })
    .to_string()
}

fn coupon_recharge_callback_payload(
    command: &CreateCouponRechargeOrderCommand,
    payment_password: Option<&str>,
) -> String {
    serde_json::json!({
        "subject": command.subject.as_str(),
        "targetAsset": command.target_asset.as_str(),
        "assetCode": command.target_asset.as_str(),
        "assetUnitCode": command.target_asset.default_unit_code(),
        "grantAmount": command.grant_amount.as_str(),
        "couponCode": command.coupon_code,
        "paymentPassword": payment_password,
    })
    .to_string()
}

fn map_token_bank_plan(value: TokenBankPlanItem) -> TokenBankPlanResponse {
    TokenBankPlanResponse {
        plan_code: value.plan_code,
        display_name: value.display_name,
        plan_period: value.plan_period.as_str().to_string(),
        grant_amount: value.grant_amount.as_str().to_string(),
        bonus_amount: value.bonus_amount.as_str().to_string(),
        price_amount: value.price_amount.as_str().to_string(),
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
        subject: value.subject.as_str().to_string(),
        target_asset: value.target_asset.as_str().to_string(),
        amount: value.amount.as_str().to_string(),
        currency_code: value.currency_code,
        status: value.status,
        provider_reference_id: value.provider_reference_id,
        created_at: value.created_at,
        updated_at: value.updated_at,
    }
}

fn required_text(value: Option<&str>, field_name: &str) -> Result<String, String> {
    let value = value.unwrap_or_default().trim();
    if value.is_empty() {
        return Err(format!("{field_name} is required"));
    }
    Ok(value.to_string())
}

fn optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn map_recharge_package(value: RechargePackageItem) -> RechargePackageResponse {
    RechargePackageResponse {
        id: value.id,
        price_amount: format_money_minor(value.price_amount.as_str()),
        currency_code: value.currency_code,
        bonus_points: value.bonus_points,
        grant_amount: value.grant_amount,
        points: value.points,
    }
}

fn map_recharge_settings(value: RechargeSettingsSnapshot) -> RechargeSettingsResponse {
    RechargeSettingsResponse {
        base_currency_code: value.base_currency_code,
        base_points_per_cny: value.base_points_per_cny,
        currency_to_cny_rates: value.currency_to_cny_rates,
        preview_examples: value
            .preview_examples
            .into_iter()
            .map(|(currency_code, amount_map)| {
                (
                    currency_code,
                    amount_map
                        .into_iter()
                        .map(|(amount, preview)| (amount, map_recharge_preview(preview)))
                        .collect::<BTreeMap<_, _>>(),
                )
            })
            .collect(),
    }
}

fn map_recharge_preview(value: RechargeGrantPreview) -> RechargeGrantPreviewResponse {
    RechargeGrantPreviewResponse {
        grant_amount: value.grant_amount,
    }
}

fn merge_recharge_pay_outcome(
    mut order: CreatePointsRechargeOrderOutcome,
    pay: PayOwnerOrderOutcome,
    payment_product: &str,
) -> CreatePointsRechargeOrderOutcome {
    order.out_trade_no = pay.out_trade_no;
    order.payment_method = pay.payment_method;
    order.payment_product = payment_product.to_owned();
    if let Some(cashier_url) = pay.payment_params.get("cashierUrl") {
        order.cashier_url = cashier_url.clone();
    }
    if let Some(qr_code_payload) = provider_qr_code(&pay.payment_params) {
        order.qr_code_payload = qr_code_payload.clone();
    }
    order.next_action = pay
        .payment_params
        .get("nextAction")
        .cloned()
        .unwrap_or_else(|| "scan_qr".to_string());
    order
}

fn provider_qr_code(payment_params: &BTreeMap<String, String>) -> Option<&String> {
    payment_params
        .get("qrCodeUrl")
        .or_else(|| payment_params.get("qrCode"))
        .or_else(|| payment_params.get("codeUrl"))
        .filter(|value| !value.trim().is_empty())
}

fn merge_account_recharge_pay_outcome(
    mut order: CreateAccountRechargeOrderOutcome,
    pay: PayOwnerOrderOutcome,
) -> CreateAccountRechargeOrderOutcome {
    order.out_trade_no = pay.out_trade_no;
    order.payment_method = pay.payment_method;
    order.status = pay.status;
    if let Some(provider_code) = pay.payment_params.get("providerCode") {
        order.provider_code = provider_code.clone();
    }
    order.payment_product = pay
        .payment_params
        .get("paymentProduct")
        .cloned()
        .unwrap_or_else(|| payment_product_for_method(&order.payment_method).to_string());
    if let Some(cashier_url) = pay.payment_params.get("cashierUrl") {
        order.cashier_url = cashier_url.clone();
    }
    if let Some(qr_code_payload) = pay.payment_params.get("qrCodePayload") {
        order.qr_code_payload = qr_code_payload.clone();
    } else if !order.cashier_url.is_empty() {
        order.qr_code_payload = order.cashier_url.clone();
    }
    order.next_action = pay
        .payment_params
        .get("nextAction")
        .cloned()
        .unwrap_or_else(|| "cashier".to_string());
    order
}

fn map_recharge_outcome(value: CreatePointsRechargeOrderOutcome) -> SubmitRechargeResponse {
    SubmitRechargeResponse {
        success: value.success,
        order_id: value.order_id,
        order_no: value.order_no,
        out_trade_no: value.out_trade_no,
        subject: AccountValueOrderSubject::PointsRecharge
            .as_str()
            .to_string(),
        target_asset: AccountValueAssetCode::Points.as_str().to_string(),
        amount: format_money_minor(value.amount.as_str()),
        grant_amount: value.points.to_string(),
        currency_code: value.currency_code,
        points: value.points,
        provider_code: value.provider_code,
        payment_method: value.payment_method,
        payment_product: value.payment_product,
        expires_at: Some(value.expires_at),
        status: value.status,
        next_action: value.next_action,
        cashier_url: value.cashier_url,
        qr_code_payload: value.qr_code_payload,
        request_payment_payload: value.request_payment_payload,
    }
}

fn map_account_recharge_outcome(
    value: CreateAccountRechargeOrderOutcome,
) -> SubmitRechargeResponse {
    let points = if matches!(value.target_asset, AccountValueAssetCode::Points) {
        value.grant_amount.as_str().parse::<i64>().unwrap_or(0)
    } else {
        0
    };
    SubmitRechargeResponse {
        success: value.success,
        order_id: value.order_id,
        order_no: value.order_no,
        out_trade_no: value.out_trade_no,
        subject: value.subject.as_str().to_string(),
        target_asset: value.target_asset.as_str().to_string(),
        amount: value.amount.as_str().to_string(),
        grant_amount: value.grant_amount.as_str().to_string(),
        currency_code: value.currency_code,
        points,
        provider_code: value.provider_code,
        payment_method: value.payment_method,
        payment_product: value.payment_product,
        expires_at: None,
        status: value.status,
        next_action: value.next_action,
        cashier_url: value.cashier_url,
        qr_code_payload: value.qr_code_payload,
        request_payment_payload: value.request_payment_payload,
    }
}

fn map_checkout_status(value: CheckoutStatusSnapshot) -> CheckoutStatusResponse {
    CheckoutStatusResponse {
        order_no: value.order_no,
        out_trade_no: value.out_trade_no,
        amount: format_money_minor(value.amount.as_str()),
        currency_code: value.currency_code,
        points: value.points,
        provider_code: value.provider_code,
        payment_method: value.payment_method,
        payment_product: value.payment_product,
        order_status: value.order_status,
        payment_status: value.payment_status,
        recharge_status: value.recharge_status,
        status: value.status,
        created_at: value.created_at,
        expires_at: value.expires_at,
        paid_at: value.paid_at,
        next_action: value.next_action,
        cashier_url: value.cashier_url,
        qr_code_payload: value.qr_code_payload,
        request_payment_payload: value.request_payment_payload,
    }
}

fn format_money_minor(value: &str) -> String {
    let Ok(minor_units) = value.trim().parse::<i64>() else {
        return value.trim().to_string();
    };
    let sign = if minor_units < 0 { "-" } else { "" };
    let absolute = minor_units.unsigned_abs();
    format!("{sign}{}.{:02}", absolute / 100, absolute % 100)
}

fn fallback_account_value_request_no(
    subject: &AppRuntimeSubject,
    order_subject: AccountValueOrderSubject,
    amount: &str,
    method: Option<&str>,
    idempotency_key: &str,
) -> String {
    stable_header_token(&format!(
        "{}-{}-{}-{}-{}",
        order_subject.as_str(),
        subject.user_id,
        amount,
        method.unwrap_or("no-payment"),
        idempotency_key
    ))
}

fn payment_product_for_method(method: &str) -> &'static str {
    match method.trim().to_ascii_lowercase().as_str() {
        "wechat_pay" | "wechat" => "wechat_native",
        "alipay" => "alipay_page",
        "balance" | "wallet_balance" => "wallet_balance",
        _ => "",
    }
}

fn stable_header_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn stable_hex_token(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn stable_storage_id(parts: &[&str]) -> String {
    let prefix = parts.first().copied().unwrap_or("account-value");
    stable_header_token(&format!("{prefix}-{}", stable_hex_token(&parts.join("|"))))
}

fn money_cents(amount: &str) -> Result<i64, ()> {
    let value = amount.trim();
    let mut parts = value.split('.');
    let whole = parts
        .next()
        .unwrap_or_default()
        .parse::<i64>()
        .map_err(|_| ())?;
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some() || fraction.len() > 2 {
        return Err(());
    }
    let mut padded = fraction.to_string();
    while padded.len() < 2 {
        padded.push('0');
    }
    let cents = if padded.is_empty() {
        0
    } else {
        padded.parse::<i64>().map_err(|_| ())?
    };
    whole
        .checked_mul(100)
        .and_then(|amount| amount.checked_add(cents))
        .ok_or(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_internal_minor_units_as_api_major_amounts() {
        assert_eq!("0.00", format_money_minor("0"));
        assert_eq!("75.00", format_money_minor("7500"));
        assert_eq!("899.00", format_money_minor("89900"));
    }

    #[test]
    fn recharge_commands_use_iso_8601_utc_expiration_boundaries() {
        let subject = test_subject();
        let command = build_create_recharge_command(CreateRechargeCommandInput {
            subject: &subject,
            amount: CommerceMoney::new("5000").expect("recharge amount"),
            currency_code: "CNY",
            method: "wechat_pay",
            request_no: "REQ-RECHARGE-UTC-1",
            idempotency_key: "IDEMP-RECHARGE-UTC-1",
            package_id: Some("recharge-500"),
            client_request_no: None,
            source: Some("token-plan"),
        })
        .expect("recharge command");

        let requested_at = sdkwork_utils_rust::parse_datetime(&command.requested_at, None)
            .expect("requestedAt must be ISO 8601 UTC");
        let expire_at = sdkwork_utils_rust::parse_datetime(&command.expire_at, None)
            .expect("expiresAt must be ISO 8601 UTC");
        assert_eq!(
            payment_expire_minutes(),
            (expire_at - requested_at).num_minutes()
        );
        assert!(command.requested_at.ends_with('Z'));
        assert!(command.expire_at.ends_with('Z'));
    }

    #[test]
    fn builds_token_bank_plan_purchase_account_recharge_command() {
        let subject = test_subject();
        let command = build_create_account_recharge_command(CreateAccountRechargeCommandInput {
            subject: &subject,
            order_subject: AccountValueOrderSubject::TokenBankPlanPurchase,
            target_asset: AccountValueAssetCode::TokenBank,
            amount: CommerceMoney::new("9900").unwrap(),
            grant_amount: CommerceMoney::new("120000").unwrap(),
            currency_code: "CNY",
            method: Some("wechat_pay"),
            request_no: "REQ-PLAN-1",
            idempotency_key: "IDEMP-PLAN-1",
            package_id: None,
            plan_code: Some("token-bank-pro-monthly"),
            plan_period: Some(TokenBankPlanPeriod::Monthly),
            coupon_code: None,
            client_request_no: Some("client-plan-1"),
        })
        .expect("plan purchase command");

        assert_eq!(
            AccountValueOrderSubject::TokenBankPlanPurchase,
            command.subject
        );
        assert_eq!(AccountValueAssetCode::TokenBank, command.target_asset);
        assert_eq!("9900", command.amount.as_str());
        assert_eq!("120000", command.grant_amount.as_str());
        assert_eq!(Some("token-bank-pro-monthly"), command.plan_code.as_deref());
        assert_eq!(Some(TokenBankPlanPeriod::Monthly), command.plan_period);
        assert!(command.order_no.starts_with("AV"));
        assert!(command.out_trade_no.starts_with("ACCOUNT"));
    }

    #[test]
    fn builds_zero_amount_coupon_recharge_without_payment_required() {
        let subject = test_subject();
        let benefit = CouponRedemptionBenefit::TokenBankCredit {
            grant_amount: CommerceMoney::new("5000").unwrap(),
        };
        let command = build_create_coupon_recharge_command(
            CreateAccountRechargeCommandInput {
                subject: &subject,
                order_subject: AccountValueOrderSubject::CouponRecharge,
                target_asset: AccountValueAssetCode::TokenBank,
                amount: CommerceMoney::new("0").unwrap(),
                grant_amount: CommerceMoney::new("5000").unwrap(),
                currency_code: "CNY",
                method: None,
                request_no: "REQ-COUPON-1",
                idempotency_key: "IDEMP-COUPON-1",
                package_id: None,
                plan_code: None,
                plan_period: None,
                coupon_code: Some("WELCOME-TOKEN-BANK"),
                client_request_no: Some("client-coupon-1"),
            },
            benefit,
        )
        .expect("coupon recharge command");

        assert_eq!(AccountValueOrderSubject::CouponRecharge, command.subject);
        assert_eq!(AccountValueAssetCode::TokenBank, command.target_asset);
        assert_eq!("0", command.amount.as_str());
        assert_eq!("5000", command.grant_amount.as_str());
        assert_eq!("WELCOME-TOKEN-BANK", command.coupon_code);
        assert!(!command.payment_required);
        assert!(command.order_no.starts_with("CP"));
        assert!(command.out_trade_no.starts_with("COUPON"));
    }

    #[test]
    fn coupon_recharge_grant_is_not_resolved_from_client_input() {
        let amount = CommerceMoney::new("0").expect("zero amount");
        let client_grant = serde_json::json!(5000);

        let error = resolve_grant_amount(
            AccountValueOrderSubject::CouponRecharge,
            &amount,
            Some(&client_grant),
        )
        .expect_err("coupon grant must be supplied by Promotion preview instead");

        assert_eq!("validation", error.code());
    }

    #[test]
    fn coupon_redemption_replay_requires_the_original_coupon_code() {
        assert!(validate_coupon_redemption_replay(
            AccountValueOrderSubject::CouponRecharge,
            Some(" SUB-MONTH "),
            "SUB-MONTH",
        )
        .is_ok());
        let error = validate_coupon_redemption_replay(
            AccountValueOrderSubject::CouponRecharge,
            Some("SUB-MONTH"),
            "TOKEN-500",
        )
        .expect_err("different coupon code must not replay the original order");
        assert_eq!("conflict", error.code());
    }

    #[test]
    fn coupon_redemption_restores_an_existing_order_before_promotion_preview() {
        let source = include_str!("recharge_router.rs");
        let handler = source
            .split("async fn create_coupon_redemption")
            .nth(1)
            .expect("coupon redemption handler");
        let replay_lookup = handler
            .find("load_account_value_order_by_idempotency")
            .expect("order replay lookup");
        let promotion_preview = handler.find(".preview_coupon").expect("Promotion preview");
        assert!(replay_lookup < promotion_preview);
    }

    #[test]
    fn points_recharge_payment_scope_matches_persisted_platform_scope() {
        assert_eq!(
            "0",
            points_recharge_organization_scope(None),
            "an unscoped points recharge order is persisted in the platform organization scope"
        );
        assert_eq!("org-1", points_recharge_organization_scope(Some(" org-1 ")));
    }

    #[test]
    fn points_recharge_defaults_to_the_mobile_h5_cashier() {
        assert_eq!(
            DEFAULT_PAYMENT_PRODUCT,
            validate_payment_product(None).expect("default payment product")
        );
        assert!(validate_payment_method_for_product("wechat_pay", DEFAULT_PAYMENT_PRODUCT).is_ok());
        assert!(!payment_product_requires_provider_route(
            DEFAULT_PAYMENT_PRODUCT
        ));
        assert!(payment_product_requires_provider_route("wechat_native"));
        assert_eq!(
            "wechat_native",
            payment_scene("wechat_native"),
            "native products retain provider channel routing"
        );
    }

    fn test_subject() -> AppRuntimeSubject {
        AppRuntimeSubject {
            tenant_id: "tenant-1".to_string(),
            organization_id: Some("org-1".to_string()),
            user_id: "user-1".to_string(),
        }
    }
}
