use std::future::Future;
use std::pin::Pin;

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

pub type PhysicalPurchaseFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShippingAddressSnapshot {
    pub receiver_name: String,
    pub receiver_phone: String,
    pub country_code: String,
    pub province: String,
    pub city: String,
    pub district: Option<String>,
    pub detail_address: String,
    pub postal_code: Option<String>,
}

impl ShippingAddressSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        receiver_name: &str,
        receiver_phone: &str,
        country_code: &str,
        province: &str,
        city: &str,
        district: Option<&str>,
        detail_address: &str,
        postal_code: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        Ok(Self {
            receiver_name: required_text("receiver_name", receiver_name)?,
            receiver_phone: required_text("receiver_phone", receiver_phone)?,
            country_code: required_text("country_code", country_code)?.to_ascii_uppercase(),
            province: required_text("province", province)?,
            city: required_text("city", city)?,
            district: optional_text(district),
            detail_address: required_text("detail_address", detail_address)?,
            postal_code: optional_text(postal_code),
        })
    }

    pub fn snapshot_json(&self) -> Result<String, CommerceServiceError> {
        serde_json::to_string(self).map_err(|error| {
            CommerceServiceError::validation(format!(
                "shipping address snapshot is invalid: {error}"
            ))
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvePhysicalCheckoutLine {
    pub sku_id: String,
    pub quantity: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvePhysicalCheckoutRequest {
    pub tenant_id: String,
    pub owner_user_id: String,
    pub currency_code: String,
    pub lines: Vec<ResolvePhysicalCheckoutLine>,
    pub shipping_address: ShippingAddressSnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPhysicalCheckoutLine {
    pub sku_id: String,
    pub product_id: String,
    pub merchant_organization_id: String,
    pub shop_id: String,
    pub title: String,
    pub unit_price: CommerceMoney,
    pub currency_code: String,
    pub fulfillment_type: String,
    pub quantity: i64,
    pub inventory_tracking: String,
    pub sku_snapshot_json: String,
}

impl ResolvedPhysicalCheckoutLine {
    pub fn fulfillment_type_is_physical(&self) -> bool {
        matches!(
            self.fulfillment_type.trim().to_ascii_lowercase().as_str(),
            "physical" | "physical_shipment"
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPhysicalCheckout {
    pub merchant_organization_id: String,
    pub shop_id: String,
    pub shop_snapshot_json: String,
    pub shipping_address: ShippingAddressSnapshot,
    pub lines: Vec<ResolvedPhysicalCheckoutLine>,
}

pub trait PhysicalCheckoutResolverPort: Send + Sync {
    fn resolve_physical_checkout<'a>(
        &'a self,
        request: ResolvePhysicalCheckoutRequest,
    ) -> PhysicalPurchaseFuture<'a, ResolvedPhysicalCheckout>;
}

#[derive(Default)]
pub struct UnavailablePhysicalCheckoutResolverPort;

impl PhysicalCheckoutResolverPort for UnavailablePhysicalCheckoutResolverPort {
    fn resolve_physical_checkout<'a>(
        &'a self,
        _request: ResolvePhysicalCheckoutRequest,
    ) -> PhysicalPurchaseFuture<'a, ResolvedPhysicalCheckout> {
        Box::pin(async move {
            Err(CommerceServiceError::provider_unavailable(
                "physical checkout resolver is not configured",
            ))
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalInventoryLine {
    pub sku_id: String,
    pub shop_id: String,
    pub quantity: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReservePhysicalOrderInventoryRequest {
    pub tenant_id: String,
    pub merchant_organization_id: String,
    pub order_id: String,
    pub request_no: String,
    pub idempotency_key: String,
    pub lines: Vec<PhysicalInventoryLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleasePhysicalOrderInventoryRequest {
    pub tenant_id: String,
    pub order_id: String,
    pub reason_code: String,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalInventoryMutationOutcome {
    pub accepted: bool,
    pub replayed: bool,
}

pub trait PhysicalInventoryReservationPort: Send + Sync {
    fn reserve_physical_order_inventory<'a>(
        &'a self,
        request: ReservePhysicalOrderInventoryRequest,
    ) -> PhysicalPurchaseFuture<'a, PhysicalInventoryMutationOutcome>;

    fn release_physical_order_inventory<'a>(
        &'a self,
        request: ReleasePhysicalOrderInventoryRequest,
    ) -> PhysicalPurchaseFuture<'a, PhysicalInventoryMutationOutcome>;

    /// Returns consumed stock back to available stock after a physical
    /// return is completed (`consumed` reservations become `restocked`).
    /// Idempotent: released/restocked reservations are skipped.
    fn restock_consumed_order_inventory<'a>(
        &'a self,
        request: ReleasePhysicalOrderInventoryRequest,
    ) -> PhysicalPurchaseFuture<'a, PhysicalInventoryMutationOutcome>;

    /// Sweeps reservations whose `expires_at` has elapsed while still
    /// `reserved`, releasing them back to stock regardless of the order's
    /// current status (covers failed releases, legacy orders without
    /// `expired_at`, and abandoned payment windows). Returns the number of
    /// orders whose reservations were released. Idempotent per reservation.
    fn sweep_expired_inventory_reservations<'a>(
        &'a self,
        limit: i64,
    ) -> PhysicalPurchaseFuture<'a, i64>;
}

/// Whether a stored `commerce_order.fulfillment_status` means the order
/// holds a physical inventory reservation that must be released on
/// cancellation or closure. Non-physical orders (points recharge, membership,
/// account value, virtual goods) never carry this status, so their lifecycle
/// stays independent of the inventory schema.
pub fn physical_order_fulfillment_requires_release(fulfillment_status: Option<&str>) -> bool {
    fulfillment_status.is_some_and(|status| status.eq_ignore_ascii_case("inventory_reserved"))
}

#[derive(Default)]
pub struct UnavailablePhysicalInventoryReservationPort;

impl PhysicalInventoryReservationPort for UnavailablePhysicalInventoryReservationPort {
    fn reserve_physical_order_inventory<'a>(
        &'a self,
        _request: ReservePhysicalOrderInventoryRequest,
    ) -> PhysicalPurchaseFuture<'a, PhysicalInventoryMutationOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::provider_unavailable(
                "physical inventory reservation is not configured",
            ))
        })
    }

    fn release_physical_order_inventory<'a>(
        &'a self,
        _request: ReleasePhysicalOrderInventoryRequest,
    ) -> PhysicalPurchaseFuture<'a, PhysicalInventoryMutationOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::provider_unavailable(
                "physical inventory release is not configured",
            ))
        })
    }

    fn restock_consumed_order_inventory<'a>(
        &'a self,
        _request: ReleasePhysicalOrderInventoryRequest,
    ) -> PhysicalPurchaseFuture<'a, PhysicalInventoryMutationOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::provider_unavailable(
                "physical inventory restock is not configured",
            ))
        })
    }

    fn sweep_expired_inventory_reservations<'a>(
        &'a self,
        _limit: i64,
    ) -> PhysicalPurchaseFuture<'a, i64> {
        Box::pin(async move {
            Err(CommerceServiceError::provider_unavailable(
                "physical inventory reservation sweep is not configured",
            ))
        })
    }
}

pub fn physical_inventory_reserve_idempotency_key(order_id: &str) -> String {
    format!("physical-goods:reserve:{order_id}")
}

pub fn physical_inventory_release_idempotency_key(order_id: &str) -> String {
    format!("physical-goods:release:{order_id}")
}

fn required_text(field: &str, value: &str) -> Result<String, CommerceServiceError> {
    crate::validation::require_non_empty(field, value)?;
    Ok(value.trim().to_owned())
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub const PHYSICAL_CHECKOUT_RESOLVER_PORT: &str = "merchandise.physical_checkout.resolver";
pub const PHYSICAL_INVENTORY_RESERVATION_PORT: &str = "inventory.physical_order.reservation";

#[cfg(test)]
mod tests {
    use super::physical_order_fulfillment_requires_release;

    #[test]
    fn inventory_reserved_status_requires_release() {
        assert!(physical_order_fulfillment_requires_release(Some(
            "inventory_reserved"
        )));
        assert!(physical_order_fulfillment_requires_release(Some(
            "INVENTORY_RESERVED"
        )));
    }

    #[test]
    fn non_physical_statuses_never_require_release() {
        for status in [
            None,
            Some(""),
            Some("awaiting_shipment"),
            Some("inventory_failed"),
            Some("fulfilled"),
        ] {
            assert!(
                !physical_order_fulfillment_requires_release(status),
                "status {status:?} must not require inventory release"
            );
        }
    }
}
