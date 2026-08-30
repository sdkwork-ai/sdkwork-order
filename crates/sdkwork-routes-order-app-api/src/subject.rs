use axum::Extension;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_web_core::WebRequestContext;

#[derive(Debug, Clone)]
pub(crate) struct AppRuntimeSubject {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub user_id: String,
}

pub(crate) fn app_runtime_subject_from_extension(
    context: Option<Extension<IamAppContext>>,
) -> Result<AppRuntimeSubject, String> {
    let Some(Extension(context)) = context else {
        return Err("authenticated runtime context is required".to_owned());
    };
    app_runtime_subject_from_iam(&context)
}

pub(crate) fn app_runtime_subject_from_contexts(
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<&WebRequestContext>,
) -> Result<AppRuntimeSubject, String> {
    if let Some(context) = runtime_context {
        return app_runtime_subject_from_extension(Some(context));
    }
    app_runtime_subject_from_web_context(request_context)
}

/// Resolves an app runtime subject for `RouteAuth::Public` catalog/config
/// routes that must stay accessible without an authenticated principal. When
/// no authenticated context is present, falls back to the platform default
/// tenant and an anonymous user so the handler can still scope its query.
pub(crate) fn app_runtime_subject_from_contexts_or_default(
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<&WebRequestContext>,
) -> AppRuntimeSubject {
    app_runtime_subject_from_contexts(runtime_context, request_context).unwrap_or_else(|_| {
        AppRuntimeSubject {
            tenant_id: "100001".to_owned(),
            organization_id: None,
            user_id: "anonymous".to_owned(),
        }
    })
}

pub(crate) fn app_runtime_subject_from_iam(
    context: &IamAppContext,
) -> Result<AppRuntimeSubject, String> {
    let tenant_id = required_context_text(&context.tenant_id, "tenant_id")?;
    let user_id = required_context_text(&context.user_id, "user_id")?;
    let organization_id = context
        .organization_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    Ok(AppRuntimeSubject {
        tenant_id,
        organization_id,
        user_id,
    })
}

pub(crate) fn app_runtime_subject_from_web_context(
    context: Option<&WebRequestContext>,
) -> Result<AppRuntimeSubject, String> {
    let Some(context) = context else {
        return Err("authenticated request context is required".to_owned());
    };
    let tenant_id = required_context_text(context.tenant_id().unwrap_or_default(), "tenant_id")?;
    let user_id = required_context_text(context.user_id().unwrap_or_default(), "user_id")?;
    let organization_id = context
        .organization_id()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    Ok(AppRuntimeSubject {
        tenant_id,
        organization_id,
        user_id,
    })
}

fn required_context_text(value: &str, field_name: &'static str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!(
            "authenticated runtime context {field_name} is required"
        ));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_web_core::{
        ServerRequestId, WebApiSurface, WebAuthMode, WebRequestContext, WebRequestPrincipal,
        WebTransportFacts,
    };

    #[test]
    fn builds_app_runtime_subject_from_web_request_context() {
        let context = web_request_context();

        let subject = app_runtime_subject_from_web_context(Some(&context)).expect("subject");

        assert_eq!("100001", subject.tenant_id);
        assert_eq!(Some("0"), subject.organization_id.as_deref());
        assert_eq!("user-1", subject.user_id);
    }

    #[test]
    fn builds_app_runtime_subject_from_available_web_request_context() {
        let context = web_request_context();

        let subject = app_runtime_subject_from_contexts(None, Some(&context)).expect("subject");

        assert_eq!("100001", subject.tenant_id);
        assert_eq!(Some("0"), subject.organization_id.as_deref());
        assert_eq!("user-1", subject.user_id);
    }

    fn web_request_context() -> WebRequestContext {
        WebRequestContext {
            request_id: ServerRequestId("test-request".to_owned()),
            api_surface: WebApiSurface::AppApi,
            auth_mode: WebAuthMode::DualToken,
            transport: WebTransportFacts {
                path: "/app/v3/api/memberships/orders".to_owned(),
                method: "POST".to_owned(),
                auth_token_present: true,
                access_token_present: true,
                api_key_present: false,
                ingress_token_present: false,
                oauth_bearer_present: false,
                agent_token_present: false,
            },
            principal: Some(
                WebRequestPrincipal::builder()
                    .tenant_id("100001")
                    .organization_id(Some("0".to_owned()))
                    .user_id("user-1")
                    .app_id("sdkwork-order-pc")
                    .build(),
            ),
            locale: None,
            client_kind: None,
            operation: None,
            idempotency_key: None,
            trace_id: None,
        }
    }
}
