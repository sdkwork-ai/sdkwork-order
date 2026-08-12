use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_utils_rust::{
    offset_list_page_info, offset_list_page_params_from_values, validated_offset_list_params,
    OffsetListPageParams, PageInfo, SdkWorkApiResponse, SdkWorkCommandData, SdkWorkPageData,
    SdkWorkProblemDetail, SdkWorkProblemRouting, SdkWorkResourceData, SdkWorkResultCode,
    MAX_LIST_PAGE_SIZE,
};
use sdkwork_web_core::WebRequestContext;

pub fn resolve_trace_id(context: Option<&WebRequestContext>) -> String {
    context
        .map(WebRequestContext::resolved_trace_id)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(sdkwork_utils_rust::uuid)
}

fn problem_routing(context: Option<&WebRequestContext>) -> SdkWorkProblemRouting {
    context
        .map(WebRequestContext::problem_routing)
        .unwrap_or_default()
}

fn problem_for_context(
    context: Option<&WebRequestContext>,
    status: StatusCode,
    result_code: SdkWorkResultCode,
    detail: impl Into<String>,
) -> Response {
    let trace_id = resolve_trace_id(context);
    let problem = SdkWorkProblemDetail::platform_enriched(
        result_code,
        detail,
        trace_id.clone(),
        problem_routing(context),
    );
    let response = (
        status,
        [(axum::http::header::CONTENT_TYPE, "application/problem+json")],
        Json(problem),
    )
        .into_response();
    attach_trace_header(response, &trace_id)
}

/// Returns a single resource as `{ code: 0, data: { item }, traceId }`.
pub fn success_item<T: serde::Serialize>(context: Option<&WebRequestContext>, item: T) -> Response {
    let trace_id = resolve_trace_id(context);
    let envelope = SdkWorkApiResponse::success(SdkWorkResourceData { item }, trace_id.clone());
    attach_trace_header((StatusCode::OK, Json(envelope)).into_response(), &trace_id)
}

pub fn success_created_item<T: serde::Serialize>(
    context: Option<&WebRequestContext>,
    item: T,
) -> Response {
    let trace_id = resolve_trace_id(context);
    let envelope = SdkWorkApiResponse::success(SdkWorkResourceData { item }, trace_id.clone());
    attach_trace_header(
        (StatusCode::CREATED, Json(envelope)).into_response(),
        &trace_id,
    )
}

/// Returns a command result with `accepted: true` inside the standard envelope.
pub fn success_command(
    context: Option<&WebRequestContext>,
    resource_id: Option<String>,
    status: Option<String>,
) -> Response {
    let trace_id = resolve_trace_id(context);
    let payload = SdkWorkCommandData {
        accepted: true,
        resource_id,
        status,
    };
    let envelope = SdkWorkApiResponse::success(payload, trace_id.clone());
    attach_trace_header((StatusCode::OK, Json(envelope)).into_response(), &trace_id)
}

/// Returns an offset-paginated list inside the standard response envelope.
/// `total_items` is the filtered row count; paging comes from `params`.
pub fn success_items<T: serde::Serialize>(
    context: Option<&WebRequestContext>,
    items: Vec<T>,
    total_items: i64,
    params: OffsetListPageParams,
) -> Response {
    let trace_id = resolve_trace_id(context);
    let page_data: SdkWorkPageData<T> = SdkWorkPageData {
        items,
        page_info: offset_list_page_info(total_items, params),
    };
    let envelope = SdkWorkApiResponse::success(page_data, trace_id.clone());
    attach_trace_header((StatusCode::OK, Json(envelope)).into_response(), &trace_id)
}

/// Returns a list with caller-provided page metadata, including cursor pages.
pub fn success_items_with_page_info<T: serde::Serialize>(
    context: Option<&WebRequestContext>,
    items: Vec<T>,
    page_info: PageInfo,
) -> Response {
    let trace_id = resolve_trace_id(context);
    let page_data = SdkWorkPageData { items, page_info };
    let envelope = SdkWorkApiResponse::success(page_data, trace_id.clone());
    attach_trace_header((StatusCode::OK, Json(envelope)).into_response(), &trace_id)
}

pub fn offset_list_page_params_from_query(page: i64, page_size: i64) -> OffsetListPageParams {
    offset_list_page_params_from_values(page, page_size)
}

/// Parses standard `page` and `page_size` values without silently clamping invalid input.
pub fn parse_offset_list_params_validated(
    context: Option<&WebRequestContext>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<OffsetListPageParams, Box<Response>> {
    validated_offset_list_params(page, page_size).map_err(|_| {
        Box::new(validation(
            context,
            format!("page must be >= 1 and page_size must be between 1 and {MAX_LIST_PAGE_SIZE}"),
        ))
    })
}

/// Parses already-validated values and clamps them to the supported range.
/// List handlers must use `parse_offset_list_params_validated` instead.
pub fn parse_offset_list_params(page: Option<i64>, page_size: Option<i64>) -> OffsetListPageParams {
    OffsetListPageParams::parse(page, page_size)
}

/// Validates `page_size` against `MAX_LIST_PAGE_SIZE`.
pub fn validate_page_size(
    context: Option<&WebRequestContext>,
    page_size: Option<i64>,
) -> Result<i64, Box<Response>> {
    parse_offset_list_params_validated(context, Some(1), page_size).map(|params| params.page_size)
}

pub fn map_service_error(
    context: Option<&WebRequestContext>,
    error: CommerceServiceError,
) -> Response {
    let (status, result_code, detail) = match error.code() {
        "validation" => (
            StatusCode::BAD_REQUEST,
            SdkWorkResultCode::ValidationError,
            error.message().to_string(),
        ),
        "not-found" => (
            StatusCode::NOT_FOUND,
            SdkWorkResultCode::NotFound,
            error.message().to_string(),
        ),
        "conflict" | "unsupported-capability" => (
            StatusCode::CONFLICT,
            SdkWorkResultCode::Conflict,
            error.message().to_string(),
        ),
        "invalid-state" => (
            StatusCode::UNPROCESSABLE_ENTITY,
            SdkWorkResultCode::UnprocessableEntity,
            error.message().to_string(),
        ),
        "unauthenticated" | "unauthorized" => (
            StatusCode::UNAUTHORIZED,
            SdkWorkResultCode::AuthenticationRequired,
            error.message().to_string(),
        ),
        "forbidden" => (
            StatusCode::FORBIDDEN,
            SdkWorkResultCode::PermissionRequired,
            error.message().to_string(),
        ),
        "provider-unavailable" => (
            StatusCode::SERVICE_UNAVAILABLE,
            SdkWorkResultCode::ServiceUnavailable,
            error.message().to_string(),
        ),
        "transport" => (
            StatusCode::BAD_GATEWAY,
            SdkWorkResultCode::BadGateway,
            error.message().to_string(),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            SdkWorkResultCode::InternalError,
            error.message().to_string(),
        ),
    };
    let operation_id = context
        .and_then(|ctx| ctx.operation.as_ref())
        .map(|operation| operation.operation_id.as_str())
        .unwrap_or("unknown");
    if status.is_server_error() {
        tracing::error!(
            trace_id = %resolve_trace_id(context),
            operation_id,
            error_code = error.code(),
            error = %error.message(),
            "order app-api request failed"
        );
    }
    problem_for_context(context, status, result_code, detail)
}

/// Maps a webhook pipeline error to a PSP-facing response. Provider
/// webhooks are semi-hostile ingress: 4xx responses carry only generic
/// messages (internal error details stay in logs), while 5xx keep the
/// existing redacted server-error handling.
pub fn map_webhook_service_error(
    context: Option<&WebRequestContext>,
    error: CommerceServiceError,
) -> Response {
    let (status, result_code, generic_detail) = match error.code() {
        "validation" => (
            StatusCode::BAD_REQUEST,
            SdkWorkResultCode::ValidationError,
            "bad request",
        ),
        "not-found" => (
            StatusCode::NOT_FOUND,
            SdkWorkResultCode::NotFound,
            "not found",
        ),
        "conflict" | "locked" | "unsupported-capability" | "invalid-state" => (
            StatusCode::CONFLICT,
            SdkWorkResultCode::Conflict,
            "conflict",
        ),
        "unauthenticated" | "unauthorized" => (
            StatusCode::UNAUTHORIZED,
            SdkWorkResultCode::AuthenticationRequired,
            "unauthorized",
        ),
        "provider-unavailable" => (
            StatusCode::SERVICE_UNAVAILABLE,
            SdkWorkResultCode::ServiceUnavailable,
            "provider unavailable",
        ),
        "transport" => (
            StatusCode::BAD_GATEWAY,
            SdkWorkResultCode::BadGateway,
            "upstream transport error",
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            SdkWorkResultCode::InternalError,
            "internal error",
        ),
    };
    tracing::warn!(
        target = "order.webhook",
        error_code = error.code(),
        error = ?error,
        "provider webhook rejected"
    );
    problem_for_context(context, status, result_code, generic_detail)
}

pub fn unauthorized(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    problem_for_context(
        context,
        StatusCode::UNAUTHORIZED,
        SdkWorkResultCode::AuthenticationRequired,
        detail,
    )
}

pub fn forbidden(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    problem_for_context(
        context,
        StatusCode::FORBIDDEN,
        SdkWorkResultCode::PermissionRequired,
        detail,
    )
}

pub fn validation(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    problem_for_context(
        context,
        StatusCode::BAD_REQUEST,
        SdkWorkResultCode::ValidationError,
        detail,
    )
}

pub fn conflict(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    problem_for_context(
        context,
        StatusCode::CONFLICT,
        SdkWorkResultCode::Conflict,
        detail,
    )
}

pub fn not_found(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    problem_for_context(
        context,
        StatusCode::NOT_FOUND,
        SdkWorkResultCode::NotFound,
        detail,
    )
}

pub fn unprocessable_entity(
    context: Option<&WebRequestContext>,
    detail: impl Into<String>,
) -> Response {
    problem_for_context(
        context,
        StatusCode::UNPROCESSABLE_ENTITY,
        SdkWorkResultCode::UnprocessableEntity,
        detail,
    )
}

pub fn not_implemented(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    problem_for_context(
        context,
        StatusCode::NOT_IMPLEMENTED,
        SdkWorkResultCode::InternalError,
        detail,
    )
}

fn attach_trace_header(response: Response, trace_id: &str) -> Response {
    let mut response = response;
    if let Ok(value) = HeaderValue::from_str(trace_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-sdkwork-trace-id"), value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use sdkwork_web_core::{
        ServerRequestId, WebApiSurface, WebAuthMode, WebOperationBinding, WebRequestContext,
        WebTransportFacts,
    };

    #[test]
    fn success_items_uses_offset_page_info_with_total_items() {
        let params = OffsetListPageParams::parse(Some(2), Some(10));
        let response = success_items(None, vec!["a".to_string()], 45, params);
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn validate_page_size_rejects_zero_and_over_max() {
        assert!(validate_page_size(None, Some(0)).is_err());
        assert!(validate_page_size(None, Some(201)).is_err());
        assert!(validate_page_size(None, Some(200)).is_ok());
        assert!(validate_page_size(None, Some(1)).is_ok());
    }

    #[test]
    fn forbidden_response_returns_403() {
        let response = forbidden(None, "no access");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn unprocessable_entity_response_returns_422() {
        let response = unprocessable_entity(None, "invalid state");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn provider_unavailable_response_returns_503() {
        let response = map_service_error(
            None,
            CommerceServiceError::provider_unavailable(
                "payment provider wechat_pay is not configured",
            ),
        );
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn success_command_returns_accepted_payload() {
        let response = success_command(
            None,
            Some("order-1".to_string()),
            Some("cancelled".to_string()),
        );
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn service_error_includes_standard_operation_routing() {
        let context = WebRequestContext {
            request_id: ServerRequestId("request-1".to_owned()),
            api_surface: WebApiSurface::AppApi,
            auth_mode: WebAuthMode::DualToken,
            transport: WebTransportFacts {
                path: "/app/v3/api/orders/order-1/payment_success".to_owned(),
                method: "GET".to_owned(),
                auth_token_present: true,
                access_token_present: true,
                api_key_present: false,
                ingress_token_present: false,
                oauth_bearer_present: false,
                agent_token_present: false,
            },
            principal: None,
            locale: None,
            client_kind: None,
            operation: Some(WebOperationBinding {
                operation_id: "orders.paymentSuccess.retrieve".to_owned(),
                route_template: "/app/v3/api/orders/{orderId}/payment_success".to_owned(),
                rate_limit_tier: None,
                idempotent: true,
            }),
            trace_id: Some("trace-1".to_owned()),
            idempotency_key: None,
        };

        let response = map_service_error(
            Some(&context),
            CommerceServiceError::storage("database read failed"),
        );
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("application/problem+json")
        );
        let body = to_bytes(response.into_body(), 16 * 1024)
            .await
            .expect("problem body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("problem json");
        assert_eq!(payload["code"], 50001);
        assert_eq!(payload["detail"], "An internal error occurred");
        assert_eq!(payload["traceId"], "trace-1");
        assert_eq!(payload["operationId"], "orders.paymentSuccess.retrieve");
        assert_eq!(
            payload["instance"],
            "GET /app/v3/api/orders/{orderId}/payment_success"
        );
    }

    #[test]
    fn webhook_error_classification_matches_psp_retry_semantics() {
        // PSP-facing classification: 4xx are terminal (no retry), 5xx retry.
        let context = None;
        for (error, expected_status) in [
            (
                CommerceServiceError::validation("bad"),
                axum::http::StatusCode::BAD_REQUEST,
            ),
            (
                CommerceServiceError::not_found("nope"),
                axum::http::StatusCode::NOT_FOUND,
            ),
            (
                CommerceServiceError::conflict("dup"),
                axum::http::StatusCode::CONFLICT,
            ),
            (
                CommerceServiceError::invalid_state("stale"),
                axum::http::StatusCode::CONFLICT,
            ),
            (
                CommerceServiceError::provider_unavailable("up"),
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                CommerceServiceError::storage("boom"),
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ] {
            let response = map_webhook_service_error(context, error);
            assert_eq!(
                expected_status,
                response.status(),
                "classification for {expected_status} mismatch"
            );
        }
    }
}
