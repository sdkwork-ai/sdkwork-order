//! PSP-enriched owner-order payment store for order app-api routers.

use std::sync::Arc;

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_payment_repository_sqlx::{
    cancel_owner_order_payments_with_provider_postgres, enrich_owner_order_payment_postgres,
    OwnerOrderPaymentEnrichmentContext, PostgresCommerceOwnerOrderPaymentStore,
};
use sdkwork_payment_service::{
    CancelOrderPaymentsCommand, PayOwnerOrderCommand, PayOwnerOrderOutcome,
};
use sqlx::PgPool;

use crate::order_router::{CommerceOrderFuture, OwnerOrderPaymentStore};

const RUNTIME_ENVIRONMENT_KEYS: &[&str] = &[
    "SDKWORK_ORDER_ENVIRONMENT",
    "ORDER_ENVIRONMENT",
    "SDKWORK_ENVIRONMENT",
    "SDKWORK_ENV",
    "SDKWORK_CLOUDROUTER_ENVIRONMENT",
];

pub struct ProviderEnrichedPostgresOwnerOrderPayments {
    inner: Arc<PostgresCommerceOwnerOrderPaymentStore>,
    pool: PgPool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
}

pub fn enriched_postgres_owner_order_payments(
    pool: PgPool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
) -> Arc<dyn OwnerOrderPaymentStore> {
    Arc::new(ProviderEnrichedPostgresOwnerOrderPayments {
        inner: Arc::new(PostgresCommerceOwnerOrderPaymentStore::new(pool.clone())),
        pool,
        registry,
        credentials,
    })
}

impl OwnerOrderPaymentStore for ProviderEnrichedPostgresOwnerOrderPayments {
    fn pay_owner_order<'a>(
        &'a self,
        command: PayOwnerOrderCommand,
    ) -> CommerceOrderFuture<'a, PayOwnerOrderOutcome> {
        let registry = self.registry.clone();
        let credentials = self.credentials.clone();
        let pool = self.pool.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            let tenant_id = command.tenant_id.clone();
            let organization_id = command.organization_id.clone();
            let order_id = command.order_id.clone();
            let payment_scene = command.payment_scene.clone();
            let outcome = inner.pay_owner_order(command).await?;
            let fallback = outcome.clone();
            let enriched = enrich_owner_order_payment_postgres(
                &pool,
                OwnerOrderPaymentEnrichmentContext {
                    deployment_registry: &registry,
                    credentials: &credentials,
                    tenant_id: &tenant_id,
                    organization_id: organization_id.as_deref(),
                    order_id: &order_id,
                    payment_scene: payment_scene.as_deref(),
                },
                outcome,
            )
            .await;
            checkout_enrichment_or_development_fallback(enriched, fallback)
        })
    }

    fn cancel_owner_order_payments<'a>(
        &'a self,
        command: sdkwork_order_service::CancelOwnerOrderCommand,
    ) -> CommerceOrderFuture<'a, ()> {
        let pool = self.pool.clone();
        let registry = self.registry.clone();
        let credentials = self.credentials.clone();
        Box::pin(async move {
            let payment_command = CancelOrderPaymentsCommand::new(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.owner_user_id,
                &command.order_id,
            )?;
            cancel_owner_order_payments_with_provider_postgres(
                &pool,
                &registry,
                &credentials,
                payment_command,
            )
            .await
        })
    }
}

fn checkout_enrichment_or_development_fallback(
    result: Result<PayOwnerOrderOutcome, CommerceServiceError>,
    fallback: PayOwnerOrderOutcome,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    match result {
        Ok(outcome) => Ok(outcome),
        Err(error) if should_use_development_cashier_fallback(&error, runtime_environment()) => {
            tracing::warn!(
                provider_code = fallback
                    .payment_params
                    .get("providerCode")
                    .map(String::as_str),
                order_id = fallback.order_id,
                "payment provider is not configured; returning the pending development cashier"
            );
            Ok(fallback)
        }
        Err(error) => Err(error),
    }
}

fn runtime_environment() -> Option<String> {
    runtime_environment_from(|key| std::env::var(key).ok())
}

fn runtime_environment_from(mut read: impl FnMut(&str) -> Option<String>) -> Option<String> {
    RUNTIME_ENVIRONMENT_KEYS.iter().copied().find_map(|key| {
        read(key)
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
    })
}

fn should_use_development_cashier_fallback(
    error: &CommerceServiceError,
    environment: Option<String>,
) -> bool {
    error.code() == "provider-unavailable"
        && error.message().contains("is not configured")
        && matches!(
            environment.as_deref(),
            Some("development" | "dev" | "local" | "test")
        )
}

#[cfg(test)]
mod tests {
    use sdkwork_contract_service::CommerceServiceError;

    use super::{runtime_environment_from, should_use_development_cashier_fallback};

    #[test]
    fn runtime_environment_accepts_cloud_router_host_environment() {
        let environment = runtime_environment_from(|key| match key {
            "SDKWORK_CLOUDROUTER_ENVIRONMENT" => Some(" Development ".to_owned()),
            _ => None,
        });

        assert_eq!(environment.as_deref(), Some("development"));
    }

    #[test]
    fn runtime_environment_prefers_order_environment_over_host_environment() {
        let environment = runtime_environment_from(|key| match key {
            "SDKWORK_ORDER_ENVIRONMENT" => Some("production".to_owned()),
            "SDKWORK_CLOUDROUTER_ENVIRONMENT" => Some("development".to_owned()),
            _ => None,
        });

        assert_eq!(environment.as_deref(), Some("production"));
    }

    #[test]
    fn runtime_environment_skips_empty_values() {
        let environment = runtime_environment_from(|key| match key {
            "SDKWORK_ORDER_ENVIRONMENT" => Some("  ".to_owned()),
            "SDKWORK_ENVIRONMENT" => Some("test".to_owned()),
            _ => None,
        });

        assert_eq!(environment.as_deref(), Some("test"));
    }

    #[test]
    fn development_cashier_fallback_only_accepts_unconfigured_provider_errors() {
        let unconfigured = CommerceServiceError::provider_unavailable(
            "payment provider wechat_pay is not configured",
        );
        let transport = CommerceServiceError::provider_unavailable("wechat transport failed");

        assert!(should_use_development_cashier_fallback(
            &unconfigured,
            Some("development".to_owned())
        ));
        assert!(should_use_development_cashier_fallback(
            &unconfigured,
            Some("dev".to_owned())
        ));
        assert!(!should_use_development_cashier_fallback(
            &unconfigured,
            Some("production".to_owned())
        ));
        assert!(!should_use_development_cashier_fallback(
            &transport,
            Some("development".to_owned())
        ));
        assert!(!should_use_development_cashier_fallback(
            &unconfigured,
            None
        ));
    }
}
