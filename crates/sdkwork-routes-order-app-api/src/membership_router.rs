use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_repository_sqlx::PostgresCommerceMembershipOrderStore;
use sdkwork_order_service::{
    CreateMembershipOrderCommand, CreateMembershipOrderOutcome, MAX_MEMBERSHIP_ORDER_QUANTITY,
};
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_payment_repository_sqlx::cashier_token::{cashier_url_for_token, CashierTokenSubject};
use sdkwork_payment_service::{
    PayOwnerOrderCommand, PayOwnerOrderCommandInput, PayOwnerOrderOutcome,
};
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api_response::{map_service_error, success_created_item, unauthorized, validation};
use crate::command_headers::required_app_write_command_headers;
use crate::order_router::OwnerOrderPaymentStore;
use crate::owner_order_payment_enrich::enriched_postgres_owner_order_payments;
use crate::subject::{app_runtime_subject_from_contexts, AppRuntimeSubject};

/// Payment countdown in seconds, backed by the shared expiry window
/// (`SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS`, default 30 minutes).
pub(crate) fn payment_expire_seconds() -> i64 {
    sdkwork_order_service::payment_expire_seconds()
}
const ALLOWED_PAYMENT_METHODS: &[&str] = &["wechat_pay", "alipay", "balance"];
/// Cashier product that resolves its provider after the order exists. Shared
/// with the app-template family, which pays through the same seam.
pub(crate) const DEFAULT_PAYMENT_PRODUCT: &str = "mobile_cashier_h5";
const PLATFORM_ORGANIZATION_SCOPE_SENTINEL: &str = "0";

pub type CommerceMembershipOrderFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommerceMembershipOrderStore: Send + Sync {
    fn create_membership_order<'a>(
        &'a self,
        command: CreateMembershipOrderCommand,
    ) -> CommerceMembershipOrderFuture<'a, CreateMembershipOrderOutcome>;
}

#[derive(Clone)]
struct AppMembershipOrderState {
    store: Arc<dyn CommerceMembershipOrderStore>,
    payments: Option<Arc<dyn OwnerOrderPaymentStore>>,
    /// Payment database pool; used to resolve the tenant's cashier key epoch.
    /// Absent in tests/embedded hosts that do not mint H5 cashier links.
    pool: Option<PgPool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateMembershipOrderRequest {
    action: Option<String>,
    package_id: Option<String>,
    payment_method: Option<String>,
    payment_product: Option<String>,
    client_request_no: Option<String>,
    source: Option<String>,
    /// 订阅期额度充值数量（仅 action=recharge）。
    grant_quantity: Option<i64>,
    /// 订阅期额度充值金额（仅 action=recharge，货币金额字符串）。
    amount: Option<String>,
    /// 席位数量：团队/组织套餐按席位计价（`price_unit='seat'`），默认 1。
    /// wire 为 int64 十进制字符串，同时兼容进程内客户端直接传数字。
    #[serde(default, with = "sdkwork_utils_rust::serde_int64::option_query")]
    quantity: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateMembershipOrderResponse {
    action: String,
    order_id: String,
    order_no: String,
    out_trade_no: String,
    amount: String,
    currency_code: String,
    package_id: String,
    package_name: String,
    duration_days: i64,
    expires_at: String,
    payment_method: String,
    payment_product: String,
    qr_code: String,
    qr_code_type: String,
    payment_id: Option<String>,
    payment_params: BTreeMap<String, String>,
    status: String,
    reused: bool,
    cashier_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    grant_quantity: Option<i64>,
}

struct CreateMembershipCommandInput<'a> {
    subject: &'a AppRuntimeSubject,
    package_id: &'a str,
    action: &'a str,
    method: &'a str,
    payment_product: &'a str,
    request_no: &'a str,
    idempotency_key: &'a str,
    client_request_no: Option<&'a str>,
    source: Option<&'a str>,
    grant_quantity: Option<i64>,
    amount: Option<&'a str>,
    quantity: i64,
}

impl CreateMembershipOrderRequest {
    fn action(&self) -> Option<&str> {
        self.action.as_deref()
    }

    fn package_id(&self) -> Option<&str> {
        self.package_id.as_deref()
    }

    fn payment_method(&self) -> Option<&str> {
        self.payment_method.as_deref()
    }

    fn payment_product(&self) -> Option<&str> {
        self.payment_product.as_deref()
    }

    fn client_request_no(&self) -> Option<&str> {
        self.client_request_no.as_deref()
    }

    fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    fn grant_quantity(&self) -> Option<i64> {
        self.grant_quantity
    }

    fn amount(&self) -> Option<&str> {
        self.amount.as_deref()
    }

    fn quantity(&self) -> Option<i64> {
        self.quantity
    }
}

impl CommerceMembershipOrderStore for PostgresCommerceMembershipOrderStore {
    fn create_membership_order<'a>(
        &'a self,
        command: CreateMembershipOrderCommand,
    ) -> CommerceMembershipOrderFuture<'a, CreateMembershipOrderOutcome> {
        Box::pin(async move { self.create_membership_order(command).await })
    }
}

pub fn app_membership_order_router_with_postgres_pool(pool: PgPool) -> Router {
    let credentials = ProviderCredentialBundle::from_env();
    let registry = Arc::new(PaymentProviderRegistry::from_credentials(
        credentials.clone(),
    ));
    app_membership_order_router_with_postgres_pool_and_payments(pool, registry, credentials)
}

pub fn app_membership_order_router_with_postgres_pool_and_payments(
    pool: PgPool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
) -> Router {
    let payments = enriched_postgres_owner_order_payments(pool.clone(), registry, credentials);
    build_app_membership_order_router_state(
        Arc::new(PostgresCommerceMembershipOrderStore::new(pool.clone())),
        Some(payments),
        Some(pool),
    )
}

pub fn build_app_membership_order_router(store: Arc<dyn CommerceMembershipOrderStore>) -> Router {
    build_app_membership_order_router_state(store, None, None)
}

pub fn build_app_membership_order_router_with_payments(
    store: Arc<dyn CommerceMembershipOrderStore>,
    payments: Arc<dyn OwnerOrderPaymentStore>,
) -> Router {
    build_app_membership_order_router_state(store, Some(payments), None)
}

fn build_app_membership_order_router_state(
    store: Arc<dyn CommerceMembershipOrderStore>,
    payments: Option<Arc<dyn OwnerOrderPaymentStore>>,
    pool: Option<PgPool>,
) -> Router {
    Router::new()
        .route(
            "/app/v3/api/memberships/orders",
            post(create_membership_order),
        )
        .with_state(AppMembershipOrderState {
            store,
            payments,
            pool,
        })
}

async fn create_membership_order(
    State(state): State<AppMembershipOrderState>,
    runtime_context: Option<axum::extract::Extension<IamAppContext>>,
    request_context: Option<axum::extract::Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(request): Json<CreateMembershipOrderRequest>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let subject = match app_runtime_subject_from_contexts(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let action = match validate_membership_action(request.action()) {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let is_recharge = action == "recharge";
    // 席位数量：团队/组织套餐按席位售卖，只有购买类动作允许指定。
    let quantity = match validate_membership_quantity(request.quantity(), is_recharge) {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    // 订阅期额度充值：数量与金额必填；非充值动作不得携带充值字段
    let package_id = if is_recharge {
        if request.grant_quantity().unwrap_or(0) <= 0 {
            return validation(
                ctx,
                "grantQuantity must be a positive integer for membership quota recharge",
            );
        }
        if !is_positive_money_text(request.amount()) {
            return validation(
                ctx,
                "amount must be a positive money value for membership quota recharge",
            );
        }
        MEMBERSHIP_QUOTA_RECHARGE_PACKAGE_ID.to_owned()
    } else {
        if request.grant_quantity().is_some() || request.amount().is_some() {
            return validation(
                ctx,
                "grantQuantity and amount are only valid for membership quota recharge",
            );
        }
        match validate_package_id(request.package_id()) {
            Ok(value) => value,
            Err(message) => return validation(ctx, message),
        }
    };
    let payment_product = match validate_payment_product(request.payment_product()) {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let method = match validate_payment_method(request.payment_method(), &payment_product) {
        Ok(value) => value,
        Err(message) => return validation(ctx, message),
    };
    let write_headers = match required_app_write_command_headers(ctx, &headers, |idempotency_key| {
        fallback_request_no(&subject, &package_id, &method, idempotency_key)
    }) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let command = match build_create_membership_command(CreateMembershipCommandInput {
        subject: &subject,
        package_id: &package_id,
        action: &action,
        method: &method,
        payment_product: &payment_product,
        request_no: &write_headers.request_no,
        idempotency_key: &write_headers.idempotency_key,
        client_request_no: request.client_request_no(),
        source: request.source(),
        grant_quantity: request.grant_quantity(),
        amount: request.amount(),
        quantity,
    }) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    let outcome = match state.store.create_membership_order(command).await {
        Ok(outcome) => outcome,
        Err(error) => return map_service_error(ctx, error),
    };

    if payment_product == DEFAULT_PAYMENT_PRODUCT {
        // Mint the short `/cashier/s1.<22>` link; the real signed token stays in
        // the payment-side mapping and never enters the QR.
        let outcome = match state.pool.as_ref() {
            Some(pool) => match apply_cashier_short_link(pool, outcome, &subject.tenant_id).await {
                Ok(outcome) => outcome,
                Err(error) => return map_service_error(ctx, error),
            },
            None => outcome,
        };
        return success_created_item(
            ctx,
            map_membership_order_outcome(outcome, &payment_product, None, request.grant_quantity()),
        );
    }

    let Some(payments) = state.payments.as_ref() else {
        return map_service_error(
            ctx,
            CommerceServiceError::provider_unavailable(
                "membership order payment orchestration is not configured",
            ),
        );
    };
    let persisted_organization_id =
        membership_order_organization_scope(subject.organization_id.as_deref());
    let payment_execution_token = stable_hex_token(&format!(
        "membership-payment|{}|{}|{}|{}",
        subject.tenant_id, outcome.order_id, method, payment_product
    ));
    let pay_command = match PayOwnerOrderCommand::new(PayOwnerOrderCommandInput {
        tenant_id: subject.tenant_id.clone(),
        organization_id: Some(persisted_organization_id),
        owner_user_id: subject.user_id.clone(),
        order_id: outcome.order_id.clone(),
        payment_method: method,
        payment_scene: Some(payment_scene(&payment_product).to_string()),
        payment_attempt_callback_payload: None,
        payment_metadata: serde_json::json!({}),
        request_no: format!("membership-payment-{payment_execution_token}"),
        idempotency_key: format!("membership-payment:{payment_execution_token}"),
    }) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };
    match payments.pay_owner_order(pay_command).await {
        Ok(payment) if provider_qr_code(&payment.payment_params).is_some() => success_created_item(
            ctx,
            map_membership_order_outcome(
                outcome,
                &payment_product,
                Some(payment),
                request.grant_quantity(),
            ),
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

fn validate_package_id(value: Option<&str>) -> Result<String, String> {
    let package_id = value.unwrap_or_default().trim();
    if package_id.is_empty() {
        return Err("package id must be provided".to_string());
    }
    Ok(package_id.to_string())
}

fn validate_membership_action(value: Option<&str>) -> Result<String, String> {
    let action = value.unwrap_or("purchase").trim().to_ascii_lowercase();
    if matches!(
        action.as_str(),
        "purchase" | "renew" | "upgrade" | "recharge" | "add_seats"
    ) {
        return Ok(action);
    }
    Err(
        "membership action must be one of: purchase, renew, upgrade, recharge, add_seats"
            .to_string(),
    )
}

/// 订阅期额度充值的合成包标识（不依赖目录套餐）。
const MEMBERSHIP_QUOTA_RECHARGE_PACKAGE_ID: &str = "membership-quota-recharge";

/// 席位数量校验：购买类动作允许 1..=10000（缺省为单席位）；充值是额度包，
/// 不按席位售卖，显式传 quantity 一律视为校验错误。
///
/// `add_seats`（席位扩容）的 quantity 是**要追加**的席位数，而不是续费或
/// 购买后的席位总数：它同样落在 1..=10000，并且不能缺省为 0。
fn validate_membership_quantity(value: Option<i64>, is_recharge: bool) -> Result<i64, String> {
    if is_recharge {
        return match value {
            None => Ok(1),
            Some(_) => Err(
                "quantity is only valid for membership purchase, renew, upgrade, and add_seats"
                    .to_string(),
            ),
        };
    }
    let quantity = value.unwrap_or(1);
    if !(1..=MAX_MEMBERSHIP_ORDER_QUANTITY).contains(&quantity) {
        return Err(format!(
            "quantity must be between 1 and {MAX_MEMBERSHIP_ORDER_QUANTITY}"
        ));
    }
    Ok(quantity)
}

/// 轻量金额正数校验（命令层为权威校验）。
fn is_positive_money_text(value: Option<&str>) -> bool {
    let raw = value.unwrap_or_default().trim();
    !raw.is_empty()
        && raw
            .parse::<f64>()
            .map(|parsed| parsed > 0.0)
            .unwrap_or(false)
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

pub(crate) fn payment_scene(payment_product: &str) -> &str {
    match payment_product {
        "wechat_native" => "wechat_native",
        "alipay_native" => "alipay_qr",
        _ => DEFAULT_PAYMENT_PRODUCT,
    }
}

pub(crate) fn provider_qr_code(payment_params: &BTreeMap<String, String>) -> Option<&String> {
    payment_params
        .get("qrCodeUrl")
        .or_else(|| payment_params.get("qrCode"))
        .or_else(|| payment_params.get("codeUrl"))
        .filter(|value| !value.trim().is_empty())
}

fn membership_order_organization_scope(organization_id: Option<&str>) -> String {
    organization_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(PLATFORM_ORGANIZATION_SCOPE_SENTINEL)
        .to_owned()
}

fn build_create_membership_command(
    input: CreateMembershipCommandInput<'_>,
) -> Result<CreateMembershipOrderCommand, CommerceServiceError> {
    // Canonical RFC 3339 store format (sdkwork_order_service::clock): the
    // command constructor validates RFC 3339 inputs, so unix-seconds text
    // here made every membership creation fail validation.
    let requested_at = sdkwork_order_service::canonical_now_timestamp();
    let expire_at =
        sdkwork_order_service::canonical_timestamp_after_seconds(payment_expire_seconds());
    let order_id = Uuid::new_v4().to_string();
    let order_item_id = Uuid::new_v4().to_string();
    let token = stable_hex_token(&format!(
        "{}|{}|{}|{}|{}|{}",
        input.subject.tenant_id,
        input.subject.organization_id.as_deref().unwrap_or(""),
        input.subject.user_id,
        input.package_id,
        input.request_no,
        input.idempotency_key,
    ));
    let order_no = format!("MB{token}");
    let out_trade_no = format!("MEMBERSHIP{token}");

    CreateMembershipOrderCommand::new(
        &input.subject.tenant_id,
        input.subject.organization_id.as_deref(),
        &input.subject.user_id,
        input.package_id,
        input.action,
        input.method,
        input.payment_product,
        &order_id,
        &order_item_id,
        &order_no,
        &out_trade_no,
        &requested_at,
        &expire_at,
        input.idempotency_key,
        input.client_request_no,
        input.source,
        input.grant_quantity,
        input.amount,
        input.quantity,
    )
}

/// Rewrites the legacy membership cashier link to the short `/cashier/s1.<22>`
/// form and stores the real signed token in the payment-side mapping.
async fn apply_cashier_short_link(
    pool: &sqlx::PgPool,
    mut outcome: CreateMembershipOrderOutcome,
    tenant_id: &str,
) -> Result<CreateMembershipOrderOutcome, CommerceServiceError> {
    let Some(base_url) = cashier_base_url_from_legacy_url(&outcome.cashier_url) else {
        return Ok(outcome);
    };
    let subject = CashierTokenSubject {
        tenant_id,
        order_id: &outcome.order_id,
        out_trade_no: &outcome.out_trade_no,
        amount: outcome.amount.as_str(),
        currency_code: &outcome.currency_code,
        scene: sdkwork_utils_rust::commerce_cashier_scene(Some("membership")),
        key_epoch: 1,
    };
    let segment = sdkwork_payment_repository_sqlx::cashier_link::mint_cashier_short_link_postgres(
        pool, &subject,
    )
    .await?;
    let Some(segment) = segment else {
        // No capability could be minted. If the cashier is *enabled* but the
        // deployment cannot mint (no KEK, or a stored secret the KEK cannot
        // open), the order-scoped link below can never be exchanged for a
        // capability — the buyer would scan a QR that only 404s. Answer
        // "temporarily unavailable" instead of handing out a dead code.
        if sdkwork_payment_repository_sqlx::cashier_token_setting::cashier_token_mint_is_blocked_postgres(
            pool, tenant_id,
        )
        .await?
        {
            return Err(CommerceServiceError::provider_unavailable(
                "the cashier is temporarily unavailable; retry shortly",
            ));
        }
        return Ok(outcome);
    };
    let Some(cashier_url) = cashier_url_for_token(&segment, Some(&base_url)) else {
        return Ok(outcome);
    };
    outcome.cashier_url = cashier_url;
    Ok(outcome)
}

/// Minting is now done by [`apply_cashier_short_link`]; this legacy helper is
/// kept only for the unit test of the pre-short-link behavior.
#[cfg(test)]
fn apply_cashier_capability_with_secret(
    mut outcome: CreateMembershipOrderOutcome,
    tenant_id: &str,
    key_epoch: i64,
    secret: Option<&[u8]>,
    issued_at: i64,
    token_id: &str,
) -> CreateMembershipOrderOutcome {
    let Some(secret) = secret else {
        return outcome;
    };
    let Some(base_url) = cashier_base_url_from_legacy_url(&outcome.cashier_url) else {
        return outcome;
    };
    let subject = CashierTokenSubject {
        tenant_id,
        order_id: &outcome.order_id,
        out_trade_no: &outcome.out_trade_no,
        amount: outcome.amount.as_str(),
        currency_code: &outcome.currency_code,
        scene: sdkwork_utils_rust::commerce_cashier_scene(Some("membership")),
        key_epoch,
    };
    let Ok(token) = sdkwork_payment_repository_sqlx::cashier_token::mint_cashier_token_with_secret(
        secret, &subject, issued_at, token_id,
    ) else {
        return outcome;
    };
    let Some(cashier_url) = cashier_url_for_token(&token, Some(&base_url)) else {
        return outcome;
    };
    outcome.cashier_url = cashier_url;
    outcome
}

/// Extracts the operator-configured origin from a legacy cashier URL.
///
/// `https://h5.example.com/cashier/ORD-1?scene=virtual&outTradeNo=OT-1`
/// yields `https://h5.example.com`. Returns `None` when the URL has no cashier
/// path or no origin, so a malformed link is left untouched instead of being
/// rewritten against the deployment default.
fn cashier_base_url_from_legacy_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let index = trimmed.find("/cashier/")?;
    let base = trimmed.get(..index)?.trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    Some(base.to_owned())
}

fn map_membership_order_outcome(
    value: CreateMembershipOrderOutcome,
    payment_product: &str,
    payment: Option<PayOwnerOrderOutcome>,
    grant_quantity: Option<i64>,
) -> CreateMembershipOrderResponse {
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
    CreateMembershipOrderResponse {
        action: value.action,
        order_id: value.order_id,
        order_no: value.order_no,
        out_trade_no: value.out_trade_no,
        amount: value.amount.as_str().to_string(),
        currency_code: value.currency_code,
        package_id: value.package_id,
        package_name: value.package_name,
        duration_days: value.duration_days,
        expires_at: value.expires_at,
        payment_method: value.payment_method,
        payment_product: payment_product.to_string(),
        qr_code,
        qr_code_type: qr_code_type.to_string(),
        payment_id,
        payment_params,
        status: payment_status.unwrap_or(value.status),
        reused: value.reused,
        cashier_url,
        grant_quantity,
    }
}

fn fallback_request_no(
    subject: &AppRuntimeSubject,
    package_id: &str,
    method: &str,
    idempotency_key: &str,
) -> String {
    stable_header_token(&format!(
        "membership-order-{}-{}-{}-{}",
        subject.user_id, package_id, method, idempotency_key
    ))
}

pub(crate) fn stable_header_token(value: &str) -> String {
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

pub(crate) fn stable_hex_token(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use sdkwork_contract_service::CommerceMoney;
    use sdkwork_order_service::CreateMembershipOrderOutcome;
    use sdkwork_utils_rust::{
        derive_cashier_token_key, verify_cashier_token, CASHIER_TOKEN_AUDIENCE,
    };

    use super::{
        apply_cashier_capability_with_secret, cashier_base_url_from_legacy_url,
        map_membership_order_outcome, membership_order_organization_scope, payment_scene,
        provider_qr_code, validate_membership_action, validate_membership_quantity,
        validate_payment_method, validate_payment_product, CreateMembershipOrderRequest,
        DEFAULT_PAYMENT_PRODUCT,
    };

    #[test]
    fn membership_action_accepts_seat_expansion_and_rejects_unknown_actions() {
        assert_eq!(
            validate_membership_action(Some(" ADD_SEATS ")).expect("seat expansion action"),
            "add_seats"
        );
        for action in ["purchase", "renew", "upgrade", "recharge"] {
            assert_eq!(
                validate_membership_action(Some(action)).expect("existing membership action"),
                action
            );
        }
        assert_eq!(
            validate_membership_action(None).expect("default action"),
            "purchase"
        );
        assert!(validate_membership_action(Some("expand")).is_err());
    }

    #[test]
    fn membership_seat_expansion_requires_a_positive_bounded_quantity() {
        // `add_seats` quantity is the number of seats to ADD; the same 1..=10000
        // bound as every other seat-priced action applies and 0 is rejected.
        assert_eq!(validate_membership_quantity(Some(5), false).unwrap(), 5);
        assert!(validate_membership_quantity(Some(0), false).is_err());
        assert!(validate_membership_quantity(Some(-5), false).is_err());
        assert!(validate_membership_quantity(Some(10_001), false).is_err());
    }

    #[test]
    fn defaults_to_order_bound_mobile_cashier_product() {
        assert_eq!(
            validate_payment_product(None).expect("default payment product"),
            DEFAULT_PAYMENT_PRODUCT
        );
    }

    #[test]
    fn native_products_require_their_provider_payment_method() {
        assert_eq!(
            validate_payment_method(Some("wechat_pay"), "wechat_native")
                .expect("wechat native method"),
            "wechat_pay"
        );
        assert!(validate_payment_method(Some("alipay"), "wechat_native").is_err());
        assert_eq!(
            validate_payment_method(Some("alipay"), "alipay_native").expect("alipay native method"),
            "alipay"
        );
        assert!(validate_payment_method(Some("wechat_pay"), "alipay_native").is_err());
    }

    #[test]
    fn native_products_map_to_provider_supported_scenes() {
        assert_eq!(payment_scene("wechat_native"), "wechat_native");
        assert_eq!(payment_scene("alipay_native"), "alipay_qr");
    }

    #[test]
    fn legacy_cashier_url_origin_is_preserved_for_token_links() {
        assert_eq!(
            cashier_base_url_from_legacy_url(
                "https://platform.birdcoder.com/cashier/ORD-1?scene=virtual&outTradeNo=OT-1"
            )
            .as_deref(),
            Some("https://platform.birdcoder.com")
        );
        assert_eq!(
            cashier_base_url_from_legacy_url(
                "https://h5.example.com/cashier/ORD-1?scene=virtual&outTradeNo=OT-1"
            )
            .as_deref(),
            Some("https://h5.example.com")
        );
        assert_eq!(
            cashier_base_url_from_legacy_url("/cashier/ORD-1?scene=virtual&outTradeNo=OT-1"),
            None
        );
        assert_eq!(
            cashier_base_url_from_legacy_url("https://h5.example.com/orders/ORD-1"),
            None
        );
    }

    #[test]
    fn membership_cashier_url_is_tokenized_with_the_order_claims() {
        const SECRET: &[u8] = b"sdkwork-cashier-token-test-secret-0001";
        let outcome = CreateMembershipOrderOutcome::new(
            "order-1",
            "purchase",
            "MB-1",
            "MEMBERSHIP-1",
            CommerceMoney::new("9900").expect("membership amount"),
            "CNY",
            "package-1",
            "Pro Token Plan",
            30,
            "2026-11-16T00:00:00Z",
            "wechat_pay",
            "pending_payment",
            false,
            "https://platform.birdcoder.com/cashier/order-1?scene=virtual&outTradeNo=MEMBERSHIP-1",
        )
        .expect("membership outcome");

        let tokenized = apply_cashier_capability_with_secret(
            outcome,
            "100001",
            1,
            Some(SECRET),
            1_700_000_000,
            "jti-1",
        );

        // The operator-configured origin survives; the URL now carries only a
        // signed token (no order id, no outTradeNo query).
        assert!(tokenized
            .cashier_url
            .starts_with("https://platform.birdcoder.com/cashier/v1."));
        assert!(!tokenized.cashier_url.contains("order-1"));
        assert!(!tokenized.cashier_url.contains("outTradeNo="));

        let token = tokenized
            .cashier_url
            .rsplit('/')
            .next()
            .expect("token path segment");
        let key = derive_cashier_token_key(SECRET, "100001").expect("signing key");
        let claims = verify_cashier_token(token, &key, 1_700_000_100, CASHIER_TOKEN_AUDIENCE)
            .expect("cashier token");
        assert_eq!(claims.order_id, "order-1");
        assert_eq!(claims.out_trade_no, "MEMBERSHIP-1");
        assert_eq!(claims.amount, "9900");
        assert_eq!(claims.currency_code, "CNY");
        assert_eq!(claims.scene, "virtual");
        assert_eq!(claims.token_id, "jti-1");
        assert_eq!(claims.epoch, 1);
    }

    #[test]
    fn membership_payment_uses_the_persisted_organization_scope() {
        assert_eq!(membership_order_organization_scope(None), "0");
        assert_eq!(membership_order_organization_scope(Some("   ")), "0");
        assert_eq!(
            membership_order_organization_scope(Some(" organization-1 ")),
            "organization-1"
        );
    }

    #[test]
    fn native_qr_requires_provider_qr_output() {
        let mut params = BTreeMap::new();
        params.insert(
            "cashierUrl".to_string(),
            "https://cashier.test/order/1".to_string(),
        );
        assert!(provider_qr_code(&params).is_none());

        params.insert(
            "qrCodeUrl".to_string(),
            "weixin://wxpay/bizpayurl?pr=order".to_string(),
        );
        assert_eq!(
            provider_qr_code(&params).map(String::as_str),
            Some("weixin://wxpay/bizpayurl?pr=order")
        );
    }

    #[test]
    fn membership_quantity_defaults_to_one_seat_and_is_bounded() {
        assert_eq!(validate_membership_quantity(None, false).unwrap(), 1);
        assert_eq!(validate_membership_quantity(Some(3), false).unwrap(), 3);
        assert_eq!(
            validate_membership_quantity(Some(10_000), false).unwrap(),
            10_000
        );
        assert!(validate_membership_quantity(Some(0), false).is_err());
        assert!(validate_membership_quantity(Some(-1), false).is_err());
        assert!(validate_membership_quantity(Some(10_001), false).is_err());
    }

    #[test]
    fn membership_quota_recharge_rejects_seat_quantity() {
        assert_eq!(validate_membership_quantity(None, true).unwrap(), 1);
        assert!(validate_membership_quantity(Some(1), true).is_err());
        assert!(validate_membership_quantity(Some(3), true).is_err());
    }

    #[test]
    fn membership_order_request_parses_int64_string_quantity() {
        let request: CreateMembershipOrderRequest = serde_json::from_str(
            r#"{"action":"purchase","packageId":"team-1","paymentMethod":"wechat_pay","quantity":"3"}"#,
        )
        .expect("int64 string quantity");
        assert_eq!(request.quantity(), Some(3));

        let numeric: CreateMembershipOrderRequest = serde_json::from_str(
            r#"{"action":"purchase","packageId":"team-1","paymentMethod":"wechat_pay","quantity":4}"#,
        )
        .expect("numeric quantity");
        assert_eq!(numeric.quantity(), Some(4));

        let omitted: CreateMembershipOrderRequest = serde_json::from_str(
            r#"{"action":"purchase","packageId":"team-1","paymentMethod":"wechat_pay"}"#,
        )
        .expect("omitted quantity");
        assert_eq!(omitted.quantity(), None);
    }

    #[test]
    fn membership_response_preserves_purchase_intent_fields_in_camel_case() {
        let outcome = CreateMembershipOrderOutcome::new(
            "order-1",
            "upgrade",
            "MB-1",
            "MEMBERSHIP-1",
            CommerceMoney::new("9900").expect("membership amount"),
            "CNY",
            "package-1",
            "Pro Token Plan",
            30,
            "2026-07-26T12:30:00Z",
            "alipay",
            "pending_payment",
            true,
            "https://cashier.example.test/orders/order-1",
        )
        .expect("membership order outcome");

        let response = map_membership_order_outcome(outcome, DEFAULT_PAYMENT_PRODUCT, None, None);
        let json = serde_json::to_value(response).expect("membership response JSON");

        assert_eq!(json["action"], "upgrade");
        assert_eq!(json["expiresAt"], "2026-07-26T12:30:00Z");
        assert_eq!(json["reused"], true);
        assert!(json.get("expires_at").is_none());
    }
}
