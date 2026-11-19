//! Anonymous (no-login) cashier endpoints.
//!
//! A scanned cashier QR must work without a platform session. The capability
//! token in the URL proves "this order may be paid"; it is **not** a login and
//! must never be treated as one.
//!
//! Security invariants enforced here:
//!
//! - The token is verified with the tenant key derived from the tenant carried in
//!   the claims; a token claiming another tenant cannot verify.
//! - Every failure (unknown, tampered, expired, order mismatch) collapses to the
//!   same `404` so an anonymous caller learns nothing about which part failed.
//! - `owner_user_id` / `organization_id` are resolved **from the order row**, not
//!   from the request, and are never serialized. The anonymous path therefore
//!   acts on the order's behalf instead of widening what the caller may see.
//! - The charged amount and currency must match the order; a token whose amount
//!   drifts from the stored order is rejected rather than silently honoured.
//! - `balance` is not offered anonymously; it is an identity-bound instrument.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use sdkwork_order_repository_sqlx::cashier_order::{
    load_cashier_link_order_postgres, load_cashier_order_postgres, CashierOrderRecord,
};
use sdkwork_payment_repository_sqlx::cashier_token::{
    mint_cashier_token_with_secret, CashierTokenSubject,
};
use sdkwork_payment_repository_sqlx::cashier_token_setting::load_cashier_token_signing_secrets_postgres;
use sdkwork_payment_service::{
    PayOwnerOrderCommand, PayOwnerOrderCommandInput, PayOwnerOrderOutcome,
};
use sdkwork_utils_rust::{
    cashier_openid_binding_message, commerce_cashier_scene, derive_cashier_token_key_with_epoch,
    extract_client_ip_from_headers, hmac_sha256_base64url, secure_compare, CashierTokenClaims,
};
use sdkwork_web_core::WebRequestContext;

use crate::api_response::{
    conflict, map_service_error, not_found, success_created_item, success_item, validation,
};
use crate::order_router::OwnerOrderPaymentStore;

/// Payment methods the anonymous cashier may request.
///
/// `balance` is deliberately absent (identity-bound). `wechat_jsapi` is allowed
/// only with an openid issued by the IAM anonymous payment OAuth and bound to this
/// token (see [`verify_openid_binding`]); the caller can never supply an openid of
/// its own choosing.
const ANONYMOUS_PAYMENT_METHODS: &[&str] = &[
    "wechat_pay",
    "wechat_h5",
    "wechat_jsapi",
    "alipay",
    "alipay_wap",
];

#[derive(Clone)]
pub struct AppCashierState {
    pool: PgPool,
    payments: Arc<dyn OwnerOrderPaymentStore>,
}

/// Public wire view of a payable order.
///
/// Everything here is safe for an anonymous caller. Ownership facts, item-level
/// addresses, and the payer identity are intentionally not part of this type.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashierOrderResponse {
    pub order_id: String,
    pub order_sn: String,
    pub subject: String,
    pub amount: String,
    pub currency_code: String,
    pub status: String,
    pub payable: bool,
    pub scene: String,
    pub allowed_methods: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<String>,
}

/// Public wire view of a created payment.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashierPaymentResponse {
    pub payment_id: String,
    pub order_id: String,
    pub out_trade_no: String,
    pub amount: String,
    pub payment_method: String,
    pub payment_params: BTreeMap<String, String>,
}

/// Public wire view of the payment status.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashierStatusResponse {
    pub order_id: String,
    pub status: String,
    pub paid: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCashierPaymentRequest {
    payment_method: Option<String>,
    /// WeChat payer openid issued by the anonymous IAM payment OAuth.
    openid: Option<String>,
    /// Signature binding `openid` to this capability token.
    openid_binding: Option<String>,
}

/// Builds the anonymous cashier router.
pub fn build_app_cashier_router(pool: PgPool, payments: Arc<dyn OwnerOrderPaymentStore>) -> Router {
    Router::new()
        // A legacy QR carries `{orderId}?outTradeNo=..` instead of a signed
        // token. The page exchanges that pair for a token, so an already-issued
        // link opens the cashier without a session. Static segment: axum ranks it
        // above `/cashier/{token}` for this method.
        .route("/app/v3/api/cashier/links", post(create_cashier_link))
        .route("/app/v3/api/cashier/{token}", get(retrieve_cashier))
        .route(
            "/app/v3/api/cashier/{token}/payments",
            post(create_cashier_payment),
        )
        .route(
            "/app/v3/api/cashier/{token}/status",
            get(retrieve_cashier_status),
        )
        .with_state(AppCashierState { pool, payments })
}

/// Request body of the legacy-link exchange.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCashierLinkRequest {
    order_id: Option<String>,
    out_trade_no: Option<String>,
}

/// Wire view of a minted capability token.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashierLinkResponse {
    pub token: String,
}

/// Exchanges a legacy `{orderId}?outTradeNo=..` link for a capability token.
///
/// The order id alone is never sufficient: the caller must also present the
/// trade number the order/checkout flow bound to it. Every failure collapses to
/// the same `404` so the endpoint cannot be used to enumerate orders or to learn
/// which part of the pair was wrong.
async fn create_cashier_link(
    State(state): State<AppCashierState>,
    request_context: Option<Extension<WebRequestContext>>,
    body: Option<Json<CreateCashierLinkRequest>>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    // `Option<Json<_>>` keeps a missing or malformed body inside the SDKWork
    // ProblemDetail envelope instead of axum's plain-text rejection.
    let Some(Json(request)) = body else {
        return validation(ctx, "a JSON body with orderId and outTradeNo is required");
    };
    let order_id = request
        .order_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let out_trade_no = request
        .out_trade_no
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let (Some(order_id), Some(out_trade_no)) = (order_id, out_trade_no) else {
        return validation(ctx, "orderId and outTradeNo are required");
    };
    let Some(link) = load_cashier_link_order_postgres(&state.pool, order_id)
        .await
        .ok()
        .flatten()
    else {
        return not_found(ctx, "cashier link is not payable");
    };
    let Some(matched_out_trade_no) = link.matched_out_trade_no(out_trade_no) else {
        return not_found(ctx, "cashier link is not payable");
    };
    let matched_out_trade_no = matched_out_trade_no.to_owned();
    let Some(order) = load_cashier_order_postgres(&state.pool, &link.tenant_id, order_id)
        .await
        .ok()
        .flatten()
    else {
        return not_found(ctx, "cashier link is not payable");
    };
    let scene = commerce_cashier_scene(Some(order.subject.as_str()));
    let material =
        sdkwork_payment_repository_sqlx::cashier_token::resolve_cashier_token_mint_material(
            &state.pool,
            &link.tenant_id,
        )
        .await;
    let material = match material {
        Ok(material) => material,
        // A configured-but-unreadable stored secret is an infrastructure error,
        // not an unknown link: surface it instead of collapsing to 404.
        Err(error) => return map_service_error(ctx, error),
    };
    let Some((secret, key_epoch)) = material else {
        // No stored secret and no deployment secret: the anonymous cashier is
        // not enabled, so the page keeps the login-required entry.
        return not_found(ctx, "cashier link is not payable");
    };
    let subject = CashierTokenSubject {
        tenant_id: &link.tenant_id,
        order_id: &order.order_id,
        out_trade_no: &matched_out_trade_no,
        amount: order.amount.as_str(),
        currency_code: order.currency_code.as_str(),
        scene,
        key_epoch,
    };
    let token_id = uuid::Uuid::new_v4().to_string();
    match mint_cashier_token_with_secret(
        &secret,
        &subject,
        sdkwork_utils_rust::now().timestamp(),
        &token_id,
    ) {
        Ok(token) => {
            // Audit fields only; the raw token must never reach logs.
            tracing::info!(
                tenant_id = %link.tenant_id,
                order_id = %order.order_id,
                out_trade_no = %matched_out_trade_no,
                "minted a cashier capability token for a legacy link"
            );
            success_created_item(ctx, CashierLinkResponse { token })
        }
        Err(error) => map_service_error(ctx, error),
    }
}

async fn retrieve_cashier(
    State(state): State<AppCashierState>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(token): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let claims = match verify_request_token(&state.pool, &token).await {
        Some(claims) => claims,
        None => return not_found(ctx, "cashier link is not payable"),
    };
    let Some(order) = load_order(&state, &claims).await else {
        return not_found(ctx, "cashier link is not payable");
    };
    if !amount_and_currency_match(&claims, &order) {
        return not_found(ctx, "cashier link is not payable");
    }
    success_item(ctx, to_cashier_view(&claims, &order))
}

async fn create_cashier_payment(
    State(state): State<AppCashierState>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(token): Path<String>,
    body: Option<Json<CreateCashierPaymentRequest>>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let claims = match verify_request_token(&state.pool, &token).await {
        Some(claims) => claims,
        None => return not_found(ctx, "cashier link is not payable"),
    };
    let Some(order) = load_order(&state, &claims).await else {
        return not_found(ctx, "cashier link is not payable");
    };
    if !amount_and_currency_match(&claims, &order) {
        return conflict(
            ctx,
            "the order amount changed after this cashier link was issued",
        );
    }
    if !order.is_payable() {
        return conflict(ctx, "the order is not payable in its current state");
    }
    let requested_body = body.map(|Json(value)| value);
    let requested = requested_body
        .as_ref()
        .and_then(|value| value.payment_method.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    let payment_method = match requested {
        Some(method) if ANONYMOUS_PAYMENT_METHODS.contains(&method.as_str()) => method,
        Some(method) => {
            return validation(
                ctx,
                format!("payment method {method} is not available without signing in"),
            )
        }
        None => {
            return validation(ctx, "a payment method is required");
        }
    };
    // `wechat_jsapi` needs the payer openid. It is accepted only when the IAM
    // anonymous payment OAuth signed it against this very token, so a caller can
    // never bind an arbitrary openid to the charge.
    let payer_openid = if payment_method == "wechat_jsapi" {
        let openid = requested_body
            .as_ref()
            .and_then(|value| value.openid.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let binding = requested_body
            .as_ref()
            .and_then(|value| value.openid_binding.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        match (openid, binding) {
            (Some(openid), Some(binding)) => {
                match verify_openid_binding(
                    &state.pool,
                    &claims.tenant_id,
                    &claims.token_id,
                    claims.epoch,
                    openid,
                    binding,
                )
                .await
                {
                    Some(openid) => Some(openid),
                    None => return validation(ctx, "the WeChat payer authorization is not valid"),
                }
            }
            _ => {
                return validation(
                    ctx,
                    "wechat_jsapi requires an authorized WeChat payer for this cashier link",
                )
            }
        }
    } else {
        None
    };
    let payment_metadata =
        anonymous_payment_metadata(&payment_method, &headers, payer_openid.as_deref());
    let command = match PayOwnerOrderCommand::new(PayOwnerOrderCommandInput {
        tenant_id: claims.tenant_id.clone(),
        organization_id: order.organization_id.clone(),
        // The capability speaks for the order, never for the caller: the owner is
        // resolved server-side from the order row.
        owner_user_id: order.owner_user_id.clone(),
        order_id: order.order_id.clone(),
        payment_method: payment_method.clone(),
        // The cashier scene is display metadata, not a provider selector: the
        // provider method key comes from `payment_method`.
        payment_scene: None,
        payment_attempt_callback_payload: None,
        payment_metadata,
        request_no: format!("cashier-{}", claims.token_id),
        idempotency_key: format!("cashier-{}-{payment_method}", claims.token_id),
    }) {
        Ok(command) => command,
        Err(error) => return map_service_error(ctx, error),
    };
    match state.payments.pay_owner_order(command).await {
        Ok(outcome) => success_item(ctx, to_payment_view(outcome)),
        Err(error) => map_service_error(ctx, error),
    }
}

async fn retrieve_cashier_status(
    State(state): State<AppCashierState>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(token): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|value| &value.0);
    let claims = match verify_request_token(&state.pool, &token).await {
        Some(claims) => claims,
        None => return not_found(ctx, "cashier link is not payable"),
    };
    let Some(order) = load_order(&state, &claims).await else {
        return not_found(ctx, "cashier link is not payable");
    };
    success_item(
        ctx,
        CashierStatusResponse {
            order_id: order.order_id.clone(),
            paid: is_paid(&order.status),
            status: order.status.clone(),
        },
    )
}

/// Verifies the capability token, returning `None` for every failure mode.
///
/// The failure modes are deliberately indistinguishable to the caller; the caller
/// must not learn whether a link was unknown, tampered, or expired. Verification
/// uses the payment-center managed signing secret (active and the rotation grace
/// secret) and falls back to the deployment secret for legacy installs.
async fn verify_request_token(pool: &PgPool, token: &str) -> Option<CashierTokenClaims> {
    // The QR carries `s1.<22>`; resolve it to the signed token stored encrypted
    // in the payment-side mapping, then run the existing signature verification.
    let signed_token = if sdkwork_utils_rust::is_cashier_short_link_segment(token) {
        let short_id = token
            .trim()
            .strip_prefix(sdkwork_utils_rust::CASHIER_SHORT_LINK_PREFIX)?;
        sdkwork_payment_repository_sqlx::cashier_link::resolve_cashier_short_link_token_postgres(
            pool, short_id,
        )
        .await
        .ok()
        .flatten()?
    } else {
        token.to_owned()
    };
    sdkwork_payment_repository_sqlx::cashier_token::verify_cashier_token_from_store(
        pool,
        &signed_token,
        sdkwork_utils_rust::now().timestamp(),
    )
    .await
}

async fn load_order(
    state: &AppCashierState,
    claims: &CashierTokenClaims,
) -> Option<CashierOrderRecord> {
    load_cashier_order_postgres(&state.pool, &claims.tenant_id, &claims.order_id)
        .await
        .ok()
        .flatten()
}

/// Rejects a link whose bound amount or currency drifted from the stored order.
fn amount_and_currency_match(claims: &CashierTokenClaims, order: &CashierOrderRecord) -> bool {
    claims.amount.trim() == order.amount.trim()
        && claims
            .currency_code
            .trim()
            .eq_ignore_ascii_case(order.currency_code.trim())
}

fn to_cashier_view(
    claims: &CashierTokenClaims,
    order: &CashierOrderRecord,
) -> CashierOrderResponse {
    CashierOrderResponse {
        order_id: order.order_id.clone(),
        order_sn: order.order_sn.clone(),
        subject: order.subject.clone(),
        amount: order.amount.clone(),
        currency_code: order.currency_code.clone(),
        status: order.status.clone(),
        payable: order.is_payable(),
        scene: claims.scene.clone(),
        allowed_methods: ANONYMOUS_PAYMENT_METHODS
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        expire_time: order.expire_time.clone(),
    }
}

fn to_payment_view(outcome: PayOwnerOrderOutcome) -> CashierPaymentResponse {
    CashierPaymentResponse {
        payment_id: outcome.payment_id,
        order_id: outcome.order_id,
        out_trade_no: outcome.out_trade_no,
        amount: outcome.amount.as_str().to_owned(),
        payment_method: outcome.payment_method,
        payment_params: outcome.payment_params,
    }
}

/// Builds provider metadata for the anonymous methods.
///
/// `wechat_h5` needs the payer's real client IP (the provider trade is bound to
/// it); `wechat_jsapi` needs the openid that the anonymous payment OAuth bound to
/// this capability token.
fn anonymous_payment_metadata(
    payment_method: &str,
    headers: &HeaderMap,
    payer_openid: Option<&str>,
) -> serde_json::Value {
    if let Some(openid) = payer_openid {
        return serde_json::json!({ "openid": openid });
    }
    if payment_method == "wechat_h5" {
        let client_ip = extract_client_ip_from_headers(|name| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        });
        return serde_json::json!({ "client_ip": client_ip.to_string() });
    }
    serde_json::json!({})
}

/// Verifies that `openid` was issued for this capability token.
///
/// The IAM anonymous payment OAuth signs `tokenId|openid` with the tenant's
/// cashier-token key after the WeChat `snsapi_base` exchange. Recomputing that
/// signature here means the payer identifier is authenticated without any shared
/// server-side session: a caller cannot invent an openid, and one issued for a
/// different token does not verify.
async fn verify_openid_binding(
    pool: &PgPool,
    tenant_id: &str,
    token_id: &str,
    key_epoch: i64,
    openid: &str,
    binding: &str,
) -> Option<String> {
    let Some(resolved) = load_cashier_token_signing_secrets_postgres(pool, tenant_id)
        .await
        .ok()
        .flatten()
    else {
        return None;
    };
    // A disabled tenant (or an unreadable stored secret) fails closed.
    if !resolved.enabled {
        return None;
    }
    let mut secrets = vec![resolved.current_secret];
    if let Some(previous) = resolved.previous_secret {
        secrets.push(previous);
    }
    for secret in secrets {
        let Ok(key) = derive_cashier_token_key_with_epoch(&secret, tenant_id, key_epoch) else {
            continue;
        };
        let expected = hmac_sha256_base64url(
            cashier_openid_binding_message(token_id, openid).as_bytes(),
            &key,
        );
        if secure_compare(&expected, binding) {
            return Some(openid.to_owned());
        }
    }
    None
}

fn is_paid(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "paid" | "fulfilled" | "completed" | "refunding" | "refunded"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(amount: &str, currency: &str) -> CashierTokenClaims {
        CashierTokenClaims {
            tenant_id: "100001".to_owned(),
            order_id: "ORD-1".to_owned(),
            out_trade_no: "OT-1".to_owned(),
            amount: amount.to_owned(),
            currency_code: currency.to_owned(),
            scene: "checkout".to_owned(),
            audience: sdkwork_utils_rust::CASHIER_TOKEN_AUDIENCE.to_owned(),
            epoch: 1,
            issued_at: 0,
            expires_at: i64::MAX,
            token_id: "jti-1".to_owned(),
        }
    }

    fn order(amount: &str, currency: &str) -> CashierOrderRecord {
        CashierOrderRecord {
            order_id: "ORD-1".to_owned(),
            order_sn: "SW1".to_owned(),
            subject: "Membership".to_owned(),
            amount: amount.to_owned(),
            currency_code: currency.to_owned(),
            quantity: 1,
            status: "pending_payment".to_owned(),
            expire_time: None,
            owner_user_id: "user-1".to_owned(),
            organization_id: Some("0".to_owned()),
        }
    }

    #[test]
    fn amount_and_currency_must_match_the_stored_order() {
        assert!(amount_and_currency_match(
            &claims("6990", "CNY"),
            &order("6990", "CNY")
        ));
        assert!(amount_and_currency_match(
            &claims("6990", "cny"),
            &order(" 6990 ", "CNY")
        ));
        assert!(!amount_and_currency_match(
            &claims("1", "CNY"),
            &order("6990", "CNY")
        ));
        assert!(!amount_and_currency_match(
            &claims("6990", "USD"),
            &order("6990", "CNY")
        ));
    }

    #[test]
    fn the_public_view_never_carries_ownership_facts() {
        let serialized = serde_json::to_string(&to_cashier_view(
            &claims("6990", "CNY"),
            &order("6990", "CNY"),
        ))
        .expect("serialize");
        assert!(!serialized.contains("user-1"));
        assert!(!serialized.contains("ownerUserId"));
        assert!(!serialized.contains("organizationId"));
        assert!(serialized.contains("\"payable\":true"));
    }

    #[test]
    fn balance_is_never_offered_anonymously() {
        assert!(!ANONYMOUS_PAYMENT_METHODS.contains(&"balance"));
        assert!(ANONYMOUS_PAYMENT_METHODS.contains(&"wechat_jsapi"));
        assert!(ANONYMOUS_PAYMENT_METHODS.contains(&"wechat_h5"));
        assert!(ANONYMOUS_PAYMENT_METHODS.contains(&"alipay_wap"));
    }

    #[test]
    fn openid_binding_message_is_token_scoped() {
        assert_eq!(
            cashier_openid_binding_message("jti-1", "o_payer"),
            "cashier-openid|jti-1|o_payer"
        );
        // A binding minted for another token must not collide.
        assert_ne!(
            cashier_openid_binding_message("jti-1", "o_payer"),
            cashier_openid_binding_message("jti-2", "o_payer")
        );
    }

    #[test]
    fn a_tampered_payload_cannot_select_a_tenant_key() {
        // Base64url of `{"tenantId":"100001"}` is decodable but carries no valid
        // signature, so verification must fail even though peeking succeeds.
        let tampered = "v1.eyJ0ZW5hbnRJZCI6IjEwMDAwMSJ9.not-a-signature";
        let secret = b"sdkwork-cashier-token-test-secret-0001";
        let key = derive_cashier_token_key_with_epoch(secret, "100001", 1).expect("key");
        assert!(sdkwork_utils_rust::verify_cashier_token(
            tampered,
            &key,
            0,
            sdkwork_utils_rust::CASHIER_TOKEN_AUDIENCE
        )
        .is_err());
    }
}
