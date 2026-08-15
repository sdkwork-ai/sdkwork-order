//! Management order lifecycle orchestration (payments before order state).

use std::sync::Arc;

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_repository_sqlx::PostgresCommerceOrderStore;
use sdkwork_order_service::{
    physical_inventory_release_idempotency_key, physical_order_fulfillment_requires_release,
    CancelManagementOrderCommand, CloseManagementOrderCommand, OrderOwnerDetailQuery,
    PhysicalInventoryReservationPort, ReleasePhysicalOrderInventoryRequest,
};
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_payment_repository_sqlx::cancel_owner_order_payments_with_provider_postgres;
use sdkwork_payment_service::CancelOrderPaymentsCommand;
use sqlx::PgPool;

#[derive(Clone)]
pub enum BackendManagementPaymentStore {
    Postgres {
        pool: PgPool,
        registry: Arc<PaymentProviderRegistry>,
        credentials: ProviderCredentialBundle,
    },
}

#[derive(Clone)]
pub enum BackendManagementOrderStore {
    Postgres(Arc<PostgresCommerceOrderStore>),
}

pub async fn cancel_management_order_with_payments(
    orders: &BackendManagementOrderStore,
    payments: &BackendManagementPaymentStore,
    inventory: &dyn PhysicalInventoryReservationPort,
    command: CancelManagementOrderCommand,
) -> Result<(), CommerceServiceError> {
    let owner_user_id = resolve_management_order_owner_user_id(
        orders,
        &command.tenant_id,
        command.organization_id.as_deref(),
        &command.order_id,
    )
    .await?;
    let requires_inventory_release = management_order_requires_inventory_release(
        orders,
        &command.tenant_id,
        command.organization_id.as_deref(),
        &owner_user_id,
        &command.order_id,
    )
    .await?;
    // Build the release request before `command` moves into the order store.
    let release = ReleasePhysicalOrderInventoryRequest {
        tenant_id: command.tenant_id.clone(),
        order_id: command.order_id.clone(),
        reason_code: command
            .cancel_type
            .clone()
            .unwrap_or_else(|| "admin_cancel".to_owned()),
        request_no: format!("admin-cancel-{}", command.order_id),
        idempotency_key: physical_inventory_release_idempotency_key(&command.order_id),
    };
    close_management_order_payments(
        payments,
        &command.tenant_id,
        command.organization_id.as_deref(),
        &command.order_id,
        &owner_user_id,
    )
    .await?;
    match orders {
        BackendManagementOrderStore::Postgres(store) => {
            store.cancel_management_order(command).await?
        }
    }
    if requires_inventory_release {
        inventory.release_physical_order_inventory(release).await?;
    }
    Ok(())
}

pub async fn close_management_order_with_payments(
    orders: &BackendManagementOrderStore,
    payments: &BackendManagementPaymentStore,
    inventory: &dyn PhysicalInventoryReservationPort,
    command: CloseManagementOrderCommand,
) -> Result<(), CommerceServiceError> {
    let owner_user_id = resolve_management_order_owner_user_id(
        orders,
        &command.tenant_id,
        command.organization_id.as_deref(),
        &command.order_id,
    )
    .await?;
    let requires_inventory_release = management_order_requires_inventory_release(
        orders,
        &command.tenant_id,
        command.organization_id.as_deref(),
        &owner_user_id,
        &command.order_id,
    )
    .await?;
    let release = ReleasePhysicalOrderInventoryRequest {
        tenant_id: command.tenant_id.clone(),
        order_id: command.order_id.clone(),
        reason_code: command
            .close_type
            .clone()
            .unwrap_or_else(|| "admin_close".to_owned()),
        request_no: format!("admin-close-{}", command.order_id),
        idempotency_key: physical_inventory_release_idempotency_key(&command.order_id),
    };
    close_management_order_payments(
        payments,
        &command.tenant_id,
        command.organization_id.as_deref(),
        &command.order_id,
        &owner_user_id,
    )
    .await?;
    match orders {
        BackendManagementOrderStore::Postgres(store) => {
            store.close_management_order(command).await?
        }
    }
    if requires_inventory_release {
        inventory.release_physical_order_inventory(release).await?;
    }
    Ok(())
}

/// Whether the order holds a physical inventory reservation that must be
/// released when a management cancel/close succeeds. Shared business rule
/// with the owner cancel flow; non-physical orders never trigger inventory.
async fn management_order_requires_inventory_release(
    orders: &BackendManagementOrderStore,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    order_id: &str,
) -> Result<bool, CommerceServiceError> {
    let fulfillment_status = match orders {
        BackendManagementOrderStore::Postgres(store) => {
            store
                .retrieve_owner_order_fulfillment_status(OrderOwnerDetailQuery {
                    tenant_id: tenant_id.to_owned(),
                    organization_id: organization_id.map(str::to_owned),
                    owner_user_id: owner_user_id.to_owned(),
                    order_id: order_id.to_owned(),
                })
                .await?
        }
    };
    Ok(fulfillment_status
        .map(|status| physical_order_fulfillment_requires_release(Some(status.as_str())))
        .unwrap_or(false))
}

async fn close_management_order_payments(
    payments: &BackendManagementPaymentStore,
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
    owner_user_id: &str,
) -> Result<(), CommerceServiceError> {
    let payment_command =
        CancelOrderPaymentsCommand::new(tenant_id, organization_id, owner_user_id, order_id)?;
    match payments {
        BackendManagementPaymentStore::Postgres {
            pool,
            registry,
            credentials,
        } => {
            cancel_owner_order_payments_with_provider_postgres(
                pool,
                registry,
                credentials,
                payment_command,
            )
            .await
        }
    }
}

pub(crate) async fn resolve_management_order_owner_user_id(
    orders: &BackendManagementOrderStore,
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
) -> Result<String, CommerceServiceError> {
    let owner_user_id = match orders {
        BackendManagementOrderStore::Postgres(store) => {
            store
                .resolve_management_order_owner_user_id(tenant_id, organization_id, order_id)
                .await?
        }
    };
    owner_user_id.ok_or_else(|| CommerceServiceError::not_found("order was not found"))
}
