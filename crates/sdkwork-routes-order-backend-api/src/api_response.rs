use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_utils_rust::{
    offset_list_page_info, offset_list_page_params_from_values, validated_offset_list_params,
    OffsetListPageParams, SdkWorkApiResponse, SdkWorkCommandData, SdkWorkPageData,
    SdkWorkProblemDetail, SdkWorkResourceData, SdkWorkResultCode, MAX_LIST_PAGE_SIZE,
};
use sdkwork_web_core::WebRequestContext;

pub fn resolve_trace_id(context: Option<&WebRequestContext>) -> String {
    context
        .and_then(|ctx| ctx.trace_id.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(sdkwork_utils_rust::uuid)
}

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

pub fn success_items<T: serde::Serialize>(
    context: Option<&WebRequestContext>,
    items: Vec<T>,
    total_items: i64,
    params: OffsetListPageParams,
) -> Response {
    let trace_id = resolve_trace_id(context);
    let page_data = SdkWorkPageData {
        items,
        page_info: offset_list_page_info(total_items, params),
    };
    let envelope = SdkWorkApiResponse::success(page_data, trace_id.clone());
    attach_trace_header((StatusCode::OK, Json(envelope)).into_response(), &trace_id)
}

pub fn offset_list_page_params_from_query(page: i64, page_size: i64) -> OffsetListPageParams {
    offset_list_page_params_from_values(page, page_size)
}

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

pub fn parse_offset_list_params(page: Option<i64>, page_size: Option<i64>) -> OffsetListPageParams {
    OffsetListPageParams::parse(page, page_size)
}

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
    let trace_id = resolve_trace_id(context);
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
        "unauthorized" => (
            StatusCode::UNAUTHORIZED,
            SdkWorkResultCode::AuthenticationRequired,
            error.message().to_string(),
        ),
        "forbidden" => (
            StatusCode::FORBIDDEN,
            SdkWorkResultCode::PermissionRequired,
            error.message().to_string(),
        ),
        "invalid-state" => (
            StatusCode::UNPROCESSABLE_ENTITY,
            SdkWorkResultCode::UnprocessableEntity,
            error.message().to_string(),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            SdkWorkResultCode::InternalError,
            error.message().to_string(),
        ),
    };
    let problem = SdkWorkProblemDetail::platform(result_code, detail, trace_id.clone());
    if status == StatusCode::INTERNAL_SERVER_ERROR {
        // The wire detail is sanitized for internal failures; without this log
        // line the stored cause (SQL error, corrupt money value, ...) is lost
        // and the trace id resolves to nothing on the gateway side.
        tracing::error!(
            target: "order.api",
            error_kind = error.code(),
            cause = %error.message(),
            trace_id = %trace_id,
            "order API request failed with an internal server error"
        );
    }
    problem_response(status, problem, &trace_id)
}

pub fn unauthorized(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    let trace_id = resolve_trace_id(context);
    let problem = SdkWorkProblemDetail::platform(
        SdkWorkResultCode::AuthenticationRequired,
        detail,
        trace_id.clone(),
    );
    problem_response(StatusCode::UNAUTHORIZED, problem, &trace_id)
}

pub fn validation(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    let trace_id = resolve_trace_id(context);
    let problem = SdkWorkProblemDetail::platform(
        SdkWorkResultCode::ValidationError,
        detail,
        trace_id.clone(),
    );
    problem_response(StatusCode::BAD_REQUEST, problem, &trace_id)
}

pub fn forbidden(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    let trace_id = resolve_trace_id(context);
    let problem = SdkWorkProblemDetail::platform(
        SdkWorkResultCode::PermissionRequired,
        detail,
        trace_id.clone(),
    );
    problem_response(StatusCode::FORBIDDEN, problem, &trace_id)
}

pub fn not_found(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    let trace_id = resolve_trace_id(context);
    let problem =
        SdkWorkProblemDetail::platform(SdkWorkResultCode::NotFound, detail, trace_id.clone());
    problem_response(StatusCode::NOT_FOUND, problem, &trace_id)
}

pub fn conflict(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    let trace_id = resolve_trace_id(context);
    let problem =
        SdkWorkProblemDetail::platform(SdkWorkResultCode::Conflict, detail, trace_id.clone());
    problem_response(StatusCode::CONFLICT, problem, &trace_id)
}

fn problem_response(status: StatusCode, problem: SdkWorkProblemDetail, trace_id: &str) -> Response {
    attach_trace_header(
        (
            status,
            [(axum::http::header::CONTENT_TYPE, "application/problem+json")],
            Json(problem),
        )
            .into_response(),
        trace_id,
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

    #[test]
    fn success_items_uses_offset_page_info_with_total_items() {
        let params = parse_offset_list_params(Some(2), Some(10));
        let response = success_items(None, vec!["a".to_string()], 45, params);
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn validate_page_size_rejects_zero_and_over_max() {
        assert!(validate_page_size(None, Some(0)).is_err());
        assert!(validate_page_size(None, Some(201)).is_err());
        assert_eq!(validate_page_size(None, Some(20)).expect("valid"), 20);
    }
}
