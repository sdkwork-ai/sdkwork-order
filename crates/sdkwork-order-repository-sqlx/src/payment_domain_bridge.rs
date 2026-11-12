//! Bridge to the payment capability's domain settings.
//!
//! The H5 payment domain (`支付设置 → 域名设置`, owned by `sdkwork-payment`) is
//! the operator-chosen origin for cashier deep-links / QR code payloads. Order
//! surfaces consume the resolved origin through this bridge so the setting has
//! exactly one owner (the payment repository) and no order query reads the
//! payment settings table directly.

use sdkwork_contract_service::CommerceServiceError;
use sqlx::PgPool;

/// Resolves the configured H5 cashier origin for a tenant scope.
///
/// `None` means "not configured": callers fall back to
/// `SDKWORK_COMMERCE_CASHIER_BASE_URL` / the platform default inside
/// `sdkwork_utils_rust::build_commerce_cashier_url_with_base`.
pub(crate) async fn resolve_h5_cashier_base_url(
    pool: &PgPool,
    tenant_id: &str,
    organization_id: Option<&str>,
) -> Result<Option<String>, CommerceServiceError> {
    sdkwork_payment_repository_sqlx::resolve_h5_cashier_base_url_postgres(
        pool,
        tenant_id,
        organization_id,
    )
    .await
}
