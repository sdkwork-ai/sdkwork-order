//! App API for the demand-claim (抢单) lifecycle.
//!
//! Thin HTTP layer over [`PostgresDemandClaimRepository`]: authenticated
//! subject resolution, request validation, role checks, and the standard v3
//! response envelope. The status machine itself lives in
//! `sdkwork_order_service::domain::demand_claim`.

use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_repository_sqlx::PostgresDemandClaimRepository;
use sdkwork_order_service::{
    parse_status_or_default, validate_demand_claim_command, CreateDemandClaimCommand,
    DemandClaimAction, DemandClaimActionCommand, DemandClaimActorType, DemandClaimListQuery,
    DemandClaimStatus, DemandClaimStatusGroup, DemandClaimTransitionPatch, DemandClaimViewerRole,
};
use sdkwork_web_core::WebRequestContext;
use serde::Deserialize;
use sqlx::PgPool;

use crate::api_response::{
    map_service_error, parse_offset_list_params, parse_offset_list_params_validated,
    success_created_item, success_item, success_items,
};
use crate::subject::{app_runtime_subject_from_contexts, AppRuntimeSubject};

#[derive(Clone)]
struct DemandClaimState {
    repository: Arc<PostgresDemandClaimRepository>,
}

#[derive(Debug, Deserialize)]
struct DemandClaimsQuery {
    status: Option<String>,
    status_group: Option<String>,
    demand_id: Option<String>,
    role: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

/// Build the demand-claim app router.
pub fn app_demand_claim_router_with_postgres_pool(pool: PgPool) -> Router {
    let state = DemandClaimState {
        repository: Arc::new(PostgresDemandClaimRepository::new(pool)),
    };
    Router::new()
        .route(
            "/app/v3/api/orders/demand_claims",
            get(list_demand_claims).post(create_demand_claim),
        )
        .route(
            "/app/v3/api/orders/demand_claims/statistics",
            get(demand_claim_statistics),
        )
        .route(
            "/app/v3/api/orders/demand_claims/{claimId}",
            get(retrieve_demand_claim),
        )
        .route(
            "/app/v3/api/orders/demand_claims/{claimId}/events",
            get(list_demand_claim_events),
        )
        .route(
            "/app/v3/api/orders/demand_claims/{claimId}/withdraw",
            post(withdraw_demand_claim),
        )
        .route(
            "/app/v3/api/orders/demand_claims/{claimId}/start",
            post(start_demand_claim),
        )
        .route(
            "/app/v3/api/orders/demand_claims/{claimId}/deliver",
            post(deliver_demand_claim),
        )
        .route(
            "/app/v3/api/orders/demand_claims/{claimId}/shortlist",
            post(shortlist_demand_claim),
        )
        .route(
            "/app/v3/api/orders/demand_claims/{claimId}/accept",
            post(accept_demand_claim),
        )
        .route(
            "/app/v3/api/orders/demand_claims/{claimId}/reject",
            post(reject_demand_claim),
        )
        .route(
            "/app/v3/api/orders/demand_claims/{claimId}/complete",
            post(complete_demand_claim),
        )
        .route(
            "/app/v3/api/orders/demand_claims/{claimId}/rework",
            post(rework_demand_claim),
        )
        .route(
            "/app/v3/api/orders/demand_claims/{claimId}/cancel",
            post(cancel_demand_claim),
        )
        .with_state(state)
}

async fn create_demand_claim(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Json(command): Json<CreateDemandClaimCommand>,
) -> Response {
    let request_context = context.as_ref().map(|Extension(value)| value);
    let subject = match subject_from(iam.as_ref(), request_context) {
        Ok(subject) => subject,
        Err(error) => return map_service_error(request_context, error),
    };
    let request_no = resolved_request_no(request_context, &headers);
    let insert = match validate_demand_claim_command(
        command,
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &request_no,
    ) {
        Ok(insert) => insert,
        Err(error) => return map_service_error(request_context, error),
    };
    match state.repository.create_claim(&insert).await {
        Ok(claim) => success_created_item(request_context, claim),
        Err(error) => map_service_error(request_context, error),
    }
}

async fn list_demand_claims(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    Query(params): Query<DemandClaimsQuery>,
) -> Response {
    let request_context = context.as_ref().map(|Extension(value)| value);
    let subject = match subject_from(iam.as_ref(), request_context) {
        Ok(subject) => subject,
        Err(error) => return map_service_error(request_context, error),
    };
    let page_params =
        match parse_offset_list_params_validated(request_context, params.page, params.page_size) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let role = match DemandClaimViewerRole::parse(params.role.as_deref()) {
        Ok(value) => value,
        Err(error) => return map_service_error(request_context, error),
    };
    // The operator inbox is a backend surface; the app API only exposes the
    // caller's own provider claims (and, when asked, the claims they publish).
    if role == DemandClaimViewerRole::Operator {
        return map_service_error(
            request_context,
            CommerceServiceError::unauthorized(
                "operator claim scope is not available on the app API",
            ),
        );
    }
    let status = match parse_status_filter(params.status.as_deref()) {
        Ok(value) => value,
        Err(error) => return map_service_error(request_context, error),
    };
    let status_group = match parse_group_filter(params.status_group.as_deref()) {
        Ok(value) => value,
        Err(error) => return map_service_error(request_context, error),
    };
    let query = DemandClaimListQuery {
        tenant_id: subject.tenant_id,
        viewer_user_id: subject.user_id,
        role,
        status,
        status_group,
        demand_id: params
            .demand_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        page: page_params.page,
        page_size: page_params.page_size,
    };
    match state.repository.list_claims(&query).await {
        Ok((items, total_items)) => success_items(request_context, items, total_items, page_params),
        Err(error) => map_service_error(request_context, error),
    }
}

async fn demand_claim_statistics(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
) -> Response {
    let request_context = context.as_ref().map(|Extension(value)| value);
    let subject = match subject_from(iam.as_ref(), request_context) {
        Ok(subject) => subject,
        Err(error) => return map_service_error(request_context, error),
    };
    match state
        .repository
        .statistics(
            &subject.tenant_id,
            &subject.user_id,
            DemandClaimViewerRole::Provider,
        )
        .await
    {
        Ok(statistics) => success_item(request_context, statistics),
        Err(error) => map_service_error(request_context, error),
    }
}

async fn retrieve_demand_claim(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    Path(claim_id): Path<String>,
) -> Response {
    let request_context = context.as_ref().map(|Extension(value)| value);
    let subject = match subject_from(iam.as_ref(), request_context) {
        Ok(subject) => subject,
        Err(error) => return map_service_error(request_context, error),
    };
    match state
        .repository
        .find_claim(&subject.tenant_id, &claim_id, &subject.user_id)
        .await
    {
        Ok(Some(claim)) if claim.role != "operator" => success_item(request_context, claim),
        Ok(Some(_)) => map_service_error(
            request_context,
            CommerceServiceError::not_found("demand claim not found"),
        ),
        Ok(None) => map_service_error(
            request_context,
            CommerceServiceError::not_found("demand claim not found"),
        ),
        Err(error) => map_service_error(request_context, error),
    }
}

async fn list_demand_claim_events(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    Path(claim_id): Path<String>,
) -> Response {
    let request_context = context.as_ref().map(|Extension(value)| value);
    let subject = match subject_from(iam.as_ref(), request_context) {
        Ok(subject) => subject,
        Err(error) => return map_service_error(request_context, error),
    };
    match state
        .repository
        .find_claim(&subject.tenant_id, &claim_id, &subject.user_id)
        .await
    {
        Ok(Some(claim)) if claim.role != "operator" => {}
        Ok(Some(_)) => {
            return map_service_error(
                request_context,
                CommerceServiceError::not_found("demand claim not found"),
            )
        }
        Ok(None) => {
            return map_service_error(
                request_context,
                CommerceServiceError::not_found("demand claim not found"),
            )
        }
        Err(error) => return map_service_error(request_context, error),
    }
    match state
        .repository
        .list_events(&subject.tenant_id, &claim_id, 200)
        .await
    {
        Ok(events) => {
            let total = events.len() as i64;
            success_items(
                request_context,
                events,
                total,
                parse_offset_list_params(Some(1), Some(200)),
            )
        }
        Err(error) => map_service_error(request_context, error),
    }
}

async fn withdraw_demand_claim(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    Path(claim_id): Path<String>,
    Json(command): Json<DemandClaimActionCommand>,
) -> Response {
    apply_action(
        state,
        context,
        iam,
        claim_id,
        DemandClaimAction::Withdraw,
        command,
    )
    .await
}

async fn start_demand_claim(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    Path(claim_id): Path<String>,
    Json(command): Json<DemandClaimActionCommand>,
) -> Response {
    apply_action(
        state,
        context,
        iam,
        claim_id,
        DemandClaimAction::Start,
        command,
    )
    .await
}

async fn deliver_demand_claim(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    Path(claim_id): Path<String>,
    Json(command): Json<DemandClaimActionCommand>,
) -> Response {
    apply_action(
        state,
        context,
        iam,
        claim_id,
        DemandClaimAction::Deliver,
        command,
    )
    .await
}

async fn shortlist_demand_claim(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    Path(claim_id): Path<String>,
    Json(command): Json<DemandClaimActionCommand>,
) -> Response {
    apply_action(
        state,
        context,
        iam,
        claim_id,
        DemandClaimAction::Shortlist,
        command,
    )
    .await
}

async fn accept_demand_claim(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    Path(claim_id): Path<String>,
    Json(command): Json<DemandClaimActionCommand>,
) -> Response {
    apply_action(
        state,
        context,
        iam,
        claim_id,
        DemandClaimAction::Accept,
        command,
    )
    .await
}

async fn reject_demand_claim(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    Path(claim_id): Path<String>,
    Json(command): Json<DemandClaimActionCommand>,
) -> Response {
    apply_action(
        state,
        context,
        iam,
        claim_id,
        DemandClaimAction::Reject,
        command,
    )
    .await
}

async fn complete_demand_claim(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    Path(claim_id): Path<String>,
    Json(command): Json<DemandClaimActionCommand>,
) -> Response {
    apply_action(
        state,
        context,
        iam,
        claim_id,
        DemandClaimAction::Complete,
        command,
    )
    .await
}

async fn rework_demand_claim(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    Path(claim_id): Path<String>,
    Json(command): Json<DemandClaimActionCommand>,
) -> Response {
    apply_action(
        state,
        context,
        iam,
        claim_id,
        DemandClaimAction::Rework,
        command,
    )
    .await
}

async fn cancel_demand_claim(
    State(state): State<DemandClaimState>,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    Path(claim_id): Path<String>,
    Json(command): Json<DemandClaimActionCommand>,
) -> Response {
    apply_action(
        state,
        context,
        iam,
        claim_id,
        DemandClaimAction::Cancel,
        command,
    )
    .await
}

async fn apply_action(
    state: DemandClaimState,
    context: Option<Extension<WebRequestContext>>,
    iam: Option<Extension<IamAppContext>>,
    claim_id: String,
    action: DemandClaimAction,
    command: DemandClaimActionCommand,
) -> Response {
    let request_context = context.as_ref().map(|Extension(value)| value);
    let subject = match subject_from(iam.as_ref(), request_context) {
        Ok(subject) => subject,
        Err(error) => return map_service_error(request_context, error),
    };
    let claim = match state
        .repository
        .find_claim(&subject.tenant_id, &claim_id, &subject.user_id)
        .await
    {
        Ok(Some(claim)) => claim,
        Ok(None) => {
            return map_service_error(
                request_context,
                CommerceServiceError::not_found("demand claim not found"),
            )
        }
        Err(error) => return map_service_error(request_context, error),
    };
    if let Err(error) = authorize_actor(&subject, &claim, action) {
        return map_service_error(request_context, error);
    }
    if action == DemandClaimAction::Deliver {
        let note = command
            .delivery_note
            .as_deref()
            .or(command.note.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if note.is_none() {
            return map_service_error(
                request_context,
                CommerceServiceError::validation("deliveryNote is required"),
            );
        }
    }
    let current = parse_status_or_default(&claim.status);
    let next = match action.validate(current) {
        Ok(next) => next,
        Err(error) => return map_service_error(request_context, error),
    };
    let note = command
        .note
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if action == DemandClaimAction::Deliver {
        if command.attachment_urls.len() > 20 {
            return map_service_error(
                request_context,
                CommerceServiceError::validation("at most 20 delivery links are allowed"),
            );
        }
        let invalid = command.attachment_urls.iter().any(|url| {
            let value = url.trim();
            value.len() > 2048 || !(value.starts_with("http://") || value.starts_with("https://"))
        });
        if invalid {
            return map_service_error(
                request_context,
                CommerceServiceError::validation(
                    "delivery links must be http(s) URLs of at most 2048 characters",
                ),
            );
        }
    }
    let now = sdkwork_order_service::canonical_now_timestamp();
    let patch = DemandClaimTransitionPatch {
        tenant_id: subject.tenant_id.clone(),
        claim_id: claim_id.clone(),
        demand_id: claim.demand_id.clone(),
        expected_status: current.as_str().to_owned(),
        next_status: next.as_str().to_owned(),
        milestone: next.milestone(),
        reject_reason: (action == DemandClaimAction::Reject)
            .then(|| note.unwrap_or("rejected").to_owned()),
        withdraw_reason: (action == DemandClaimAction::Withdraw)
            .then(|| note.unwrap_or("withdrawn").to_owned()),
        cancel_reason: (action == DemandClaimAction::Cancel)
            .then(|| note.unwrap_or("cancelled").to_owned()),
        delivery_note: (action == DemandClaimAction::Deliver).then(|| {
            command
                .delivery_note
                .as_deref()
                .or(command.note.as_deref())
                .unwrap_or_default()
                .trim()
                .to_owned()
        }),
        delivery_attachment_urls: (action == DemandClaimAction::Deliver)
            .then(|| command.attachment_urls.clone()),
        now: now.clone(),
    };
    let actor_type = action.actor();
    match state
        .repository
        .apply_transition(
            &patch,
            actor_type,
            &subject.user_id,
            &format!("claim.{}", action.as_str()),
            note,
            serde_json::json!({
                "claimNo": claim.claim_no,
                "note": note,
                "deliveryNote": command.delivery_note,
                "attachmentUrls": command.attachment_urls,
            }),
        )
        .await
    {
        Ok(true) => match state
            .repository
            .find_claim(&subject.tenant_id, &claim_id, &subject.user_id)
            .await
        {
            Ok(Some(updated)) => success_item(request_context, updated),
            Ok(None) => map_service_error(
                request_context,
                CommerceServiceError::storage("claim disappeared after transition"),
            ),
            Err(error) => map_service_error(request_context, error),
        },
        Ok(false) => map_service_error(
            request_context,
            CommerceServiceError::invalid_state("the claim status changed; reload and retry"),
        ),
        Err(error) => map_service_error(request_context, error),
    }
}

fn subject_from(
    iam: Option<&Extension<IamAppContext>>,
    context: Option<&WebRequestContext>,
) -> Result<AppRuntimeSubject, CommerceServiceError> {
    app_runtime_subject_from_contexts(iam.cloned(), context)
        .map_err(CommerceServiceError::unauthorized)
}

fn authorize_actor(
    subject: &AppRuntimeSubject,
    claim: &sdkwork_order_service::DemandClaimView,
    action: DemandClaimAction,
) -> Result<(), CommerceServiceError> {
    let allowed = match action.actor() {
        DemandClaimActorType::Provider => claim.bidder_user_id == subject.user_id,
        DemandClaimActorType::Publisher => claim.publisher_user_id == subject.user_id,
        DemandClaimActorType::Operator => true,
        DemandClaimActorType::System => false,
    };
    if allowed {
        Ok(())
    } else {
        Err(CommerceServiceError::unauthorized(format!(
            "claim action {} is not allowed for this principal",
            action.as_str()
        )))
    }
}

fn parse_status_filter(
    value: Option<&str>,
) -> Result<Option<DemandClaimStatus>, CommerceServiceError> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some(value) => DemandClaimStatus::parse(value).map(Some).ok_or_else(|| {
            CommerceServiceError::validation(format!("unknown claim status: {value}"))
        }),
    }
}

fn parse_group_filter(
    value: Option<&str>,
) -> Result<Option<DemandClaimStatusGroup>, CommerceServiceError> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some(value) => DemandClaimStatusGroup::parse(value)
            .map(Some)
            .ok_or_else(|| {
                CommerceServiceError::validation(format!("unknown claim status group: {value}"))
            }),
    }
}

fn resolved_request_no(context: Option<&WebRequestContext>, headers: &HeaderMap) -> String {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            context
                .map(WebRequestContext::resolved_trace_id)
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(sdkwork_utils_rust::uuid)
}
