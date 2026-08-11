//! Owner-initiated order cancel orchestration (payments before order state).

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    physical_inventory_release_idempotency_key, physical_order_fulfillment_requires_release,
    CancelOwnerOrderCommand, OrderOwnerDetailQuery, PhysicalInventoryReservationPort,
    ReleasePhysicalOrderInventoryRequest,
};

use crate::order_router::{CommerceOrderStore, OwnerOrderPaymentStore};

/// Close payment intents first, then cancel the order.
///
/// Payment cancellation is attempted before mutating order status so a PSP
/// failure does not leave a cancelled order with still-open payment attempts.
pub async fn cancel_owner_order_with_payments(
    orders: &dyn CommerceOrderStore,
    payments: &dyn OwnerOrderPaymentStore,
    command: CancelOwnerOrderCommand,
) -> Result<(), CommerceServiceError> {
    payments
        .cancel_owner_order_payments(command.clone())
        .await?;
    orders.cancel_owner_order(command).await
}

pub async fn cancel_owner_order_with_payments_and_inventory(
    orders: &dyn CommerceOrderStore,
    payments: &dyn OwnerOrderPaymentStore,
    inventory: &dyn PhysicalInventoryReservationPort,
    command: CancelOwnerOrderCommand,
    request_no: &str,
) -> Result<(), CommerceServiceError> {
    // Only orders that successfully reserved physical inventory need a
    // release. Skipping the inventory port for non-physical orders keeps
    // their cancellation independent of the inventory schema.
    let requires_inventory_release = orders
        .retrieve_owner_order_fulfillment_status(OrderOwnerDetailQuery {
            tenant_id: command.tenant_id.clone(),
            organization_id: command.organization_id.clone(),
            owner_user_id: command.owner_user_id.clone(),
            order_id: command.order_id.clone(),
        })
        .await?
        .map(|status| physical_order_fulfillment_requires_release(Some(status.as_str())))
        .unwrap_or(false);
    let release = ReleasePhysicalOrderInventoryRequest {
        tenant_id: command.tenant_id.clone(),
        order_id: command.order_id.clone(),
        reason_code: command
            .cancel_type
            .clone()
            .unwrap_or_else(|| "buyer_cancelled".to_owned()),
        request_no: request_no.to_owned(),
        idempotency_key: physical_inventory_release_idempotency_key(&command.order_id),
    };
    cancel_owner_order_with_payments(orders, payments, command).await?;
    if requires_inventory_release {
        inventory.release_physical_order_inventory(release).await?;
    }
    Ok(())
}

/// Best-effort rollback when a recharge checkout create succeeded but pay failed.
pub async fn compensate_failed_recharge_pay(
    orders: &dyn CommerceOrderStore,
    payments: &dyn OwnerOrderPaymentStore,
    command: CancelOwnerOrderCommand,
) {
    let _ = cancel_owner_order_with_payments(orders, payments, command).await;
}
