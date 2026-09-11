use axum::Router;
use sdkwork_iam_web_adapter::IamWebRequestContextResolver;
use sdkwork_web_axum::{with_web_request_context, WebFrameworkLayer};
use sdkwork_web_core::{SecurityPolicy, WebEnvironment, WebRequestContextProfile};

use crate::http_route_manifest::{app_route_manifest, order_app_api_public_path_prefixes};

pub fn wrap_router_with_web_framework(
    resolver: IamWebRequestContextResolver,
    router: Router,
) -> Router {
    let route_manifest = app_route_manifest();
    route_manifest
        .validate_public_path_prefixes(&order_app_api_public_path_prefixes())
        .expect("order app-api public prefixes must not cover protected manifest routes");

    let (environment, security_policy) = order_web_security_policy_from_env();
    let layer = WebFrameworkLayer::new(resolver)
        .with_profile(WebRequestContextProfile {
            public_path_prefixes: order_app_api_public_path_prefixes(),
            environment,
            ..WebRequestContextProfile::default()
        })
        .with_security_policy(security_policy)
        .with_route_manifest(route_manifest);
    with_web_request_context(router, layer)
}

pub async fn wrap_router_with_web_framework_from_env(router: Router) -> Router {
    let resolver = sdkwork_iam_web_adapter::iam_web_request_context_resolver_from_env().await;
    wrap_router_with_web_framework(resolver, router)
}

fn order_web_security_policy_from_env() -> (WebEnvironment, SecurityPolicy) {
    let (environment, mut policy) = sdkwork_web_bootstrap::application_security_policy_from_env(
        &[
            "SDKWORK_ENVIRONMENT",
            "SDKWORK_ORDER_ENVIRONMENT",
            "ORDER_ENVIRONMENT",
            "SDKWORK_ENV",
        ],
        &["SDKWORK_CORS_ALLOWED_ORIGINS"],
    );
    // Public/standard request headers only. Locale negotiation travels through
    // the standard `Accept-Language` header, which the shared framework default
    // allow-list already carries; no SDKWork custom locale header exists
    // (`I18N_SPEC.md` §4).
    for header in ["sdkwork-request-no", "traceparent", "tracestate"] {
        if !policy
            .cors
            .allowed_headers
            .iter()
            .any(|candidate| candidate == header)
        {
            policy.cors.allowed_headers.push(header.to_owned());
        }
    }
    (environment, policy)
}
