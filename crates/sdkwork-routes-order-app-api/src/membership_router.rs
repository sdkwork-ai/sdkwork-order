use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_repository_sqlx::PostgresCommerceMembershipOrderStore;
use sdkwork_order_service::{CreateMembershipOrderCommand, CreateMembershipOrderOutcome};
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
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
fn payment_expire_seconds() -> i64 {
    sdkwork_order_service::payment_expire_seconds()
}
const ALLOWED_PAYMENT_METHODS: &[&str] = &["wechat_pay", "alipay", "balance"];
const DEFAULT_PAYMENT_PRODUCT: &str = "mobile_cashier_h5";
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
    build_app_membership_order_router_with_payments(
        Arc::new(PostgresCommerceMembershipOrderStore::new(pool)),
        payments,
    )
}

pub fn build_app_membership_order_router(store: Arc<dyn CommerceMembershipOrderStore>) -> Router {
    build_app_membership_order_router_state(store, None)
}

pub fn build_app_membership_order_router_with_payments(
    store: Arc<dyn CommerceMembershipOrderStore>,
    payments: Arc<dyn OwnerOrderPaymentStore>,
) -> Router {
    build_app_membership_order_router_state(store, Some(payments))
}

fn build_app_membership_order_router_state(
    store: Arc<dyn CommerceMembershipOrderStore>,
    payments: Option<Arc<dyn OwnerOrderPaymentStore>>,
) -> Router {
    Router::new()
        .route(
            "/app/v3/api/memberships/orders",
            post(create_membership_order),
        )
        .with_state(AppMembershipOrderState { store, payments })
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
    }) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };

    let outcome = match state.store.create_membership_order(command).await {
        Ok(outcome) => outcome,
        Err(error) => return map_service_error(ctx, error),
    };

    if payment_product == DEFAULT_PAYMENT_PRODUCT {
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
        "purchase" | "renew" | "upgrade" | "recharge"
    ) {
        return Ok(action);
    }
    Err("membership action must be one of: purchase, renew, upgrade, recharge".to_string())
}

/// 订阅期额度充值的合成包标识（不依赖目录套餐）。
const MEMBERSHIP_QUOTA_RECHARGE_PACKAGE_ID: &str = "membership-quota-recharge";

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

fn payment_scene(payment_product: &str) -> &str {
    match payment_product {
        "wechat_native" => "wechat_native",
        "alipay_native" => "alipay_qr",
        _ => DEFAULT_PAYMENT_PRODUCT,
    }
}

fn provider_qr_code(payment_params: &BTreeMap<String, String>) -> Option<&String> {
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
    let now = current_unix_timestamp();
    let requested_at = format_unix_timestamp(now);
    let expire_at = format_unix_timestamp(now + payment_expire_seconds());
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
    )
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

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn format_unix_timestamp(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use sdkwork_contract_service::CommerceMoney;
    use sdkwork_order_service::CreateMembershipOrderOutcome;

    use super::{
        map_membership_order_outcome, membership_order_organization_scope, payment_scene,
        provider_qr_code, validate_payment_method, validate_payment_product,
        DEFAULT_PAYMENT_PRODUCT,
    };

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
