//! Gateway assembly for sdkwork-order.
//! Application bootstrap lives in `bootstrap.rs`; route inventory is in `assembly-manifest.json`.
// SDKWORK-ASSEMBLY-LIB-CUSTOM

mod bootstrap;
mod generated;

pub use bootstrap::{
    assemble_api_router, assemble_api_router_with_pool, assemble_app_api_contribution,
    assemble_app_api_contribution_with_pool, assemble_backend_business_router, ApiAssembly,
    ApiAssemblyContribution, BusinessRouterAssembly, OrderAssemblyContract,
};

pub async fn assemble_api_router_from_env() -> Result<ApiAssembly, String> {
    let host = std::sync::Arc::new(sdkwork_order_service_host::OrderServiceHost::from_env().await?);
    assemble_api_router(host).await
}

pub async fn assemble_backend_business_router_from_env() -> Result<BusinessRouterAssembly, String> {
    let host = std::sync::Arc::new(sdkwork_order_service_host::OrderServiceHost::from_env().await?);
    Ok(assemble_backend_business_router(host).await)
}

pub fn order_contract_fallback_config() -> sdkwork_web_bootstrap::ContractFallbackConfig {
    OrderAssemblyContract::contract_fallback_config()
}

/// Order App API route manifest for host gateway composition (API_ASSEMBLY_SPEC §3).
pub fn app_api_route_manifest() -> sdkwork_web_core::HttpRouteManifest {
    OrderAssemblyContract::app_route_manifest()
}

pub fn assembly_route_count() -> usize {
    generated::ROUTE_CRATE_COUNT
}
