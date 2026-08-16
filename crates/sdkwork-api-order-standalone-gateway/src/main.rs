//! Order API server entrypoint.
//!
//! Production-grade bootstrap:
//! - Returns `Result` from host bootstrap so DB errors don't panic the process.
//! - CORS is restricted to an explicit allow-list read from `SDKWORK_CORS_ALLOWED_ORIGINS`.
//! - Readiness probe reflects the real database health via `SELECT 1`.
//! - Graceful shutdown drains in-flight requests on SIGINT / SIGTERM.

use sdkwork_api_order_assembly::assemble_api_router_from_env;
use sdkwork_iam_web_adapter::{
    build_web_framework_builder, iam_web_request_context_resolver_from_env,
};
use sdkwork_web_bootstrap::{infra_public_path_prefixes, ComposedApiAssembly};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let assembly = match assemble_api_router_from_env().await {
        Ok(assembly) => assembly,
        Err(error) => {
            tracing::error!(target = "order.bootstrap", error = %error, "order API assembly bootstrap failed");
            return Err(error.into());
        }
    };

    if std::env::var("ORDER_READ_MODEL_LENIENT").as_deref() == Ok("1") {
        tracing::warn!(
            target = "order.security",
            "ORDER_READ_MODEL_LENIENT=1 is active; missing commerce tables return empty reads — forbidden in production"
        );
    }

    let framework = build_web_framework_builder(
        iam_web_request_context_resolver_from_env().await,
        assembly.route_manifest.clone(),
        infra_public_path_prefixes(),
    );
    let app = ComposedApiAssembly::try_compose("SDKWork Order API", vec![assembly])
        .map_err(std::io::Error::other)?
        .into_hosted(framework)
        .router
        .layer(TraceLayer::new_for_http());

    let addr = std::env::var("ORDER_API_BIND").unwrap_or_else(|_| "0.0.0.0:18093".to_owned());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(target = "order.bootstrap", %addr, "order api server listening");

    // `with_graceful_shutdown` makes axum::serve stop accepting new
    // connections once the signal future resolves, then drain in-flight
    // requests. We don't duplicate the signal with tokio::select! here.
    let serve = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());

    if let Err(error) = serve.await {
        tracing::error!(target = "order.runtime", error = %error, "axum serve failed");
        return Err(error.into());
    }

    tracing::info!(target = "order.runtime", "order api server stopped");
    Ok(())
}

/// Waits for SIGINT (Ctrl+C) or SIGTERM to trigger graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!(target = "order.runtime", error = %error, "ctrl_c signal handler failed");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(error) => {
                tracing::warn!(target = "order.runtime", error = %error, "SIGTERM signal handler failed");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
