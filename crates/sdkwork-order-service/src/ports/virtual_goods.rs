//! Virtual goods fulfillment boundary（虚拟商品支付成功履约）.
//!
//! Virtual goods are the second half of the paid-order fulfillment matrix:
//!
//! | subject kind | fulfillment owner | terminal order state |
//! | --- | --- | --- |
//! | `product` (physical) | [`crate::PhysicalGoodsFulfillmentPort`] → inventory + shipment | `fulfilled` after delivery |
//! | `virtual_goods` | [`VirtualGoodsFulfillmentPort`] → entitlement delivery | `fulfilled` at settlement |
//! | `membership` / `app_template` / recharge | the owning capability's port | domain-specific |
//!
//! A virtual-goods order has no shipment and no stock reservation: payment
//! success IS delivery. The port therefore owns one idempotent transition —
//! "entitlement delivered exactly once for this order" — and the settlement
//! pipeline keeps owning payment/order state.
//!
//! Implementations MUST key every write on
//! [`virtual_goods_fulfillment_idempotency_key`] (or the order id) so a
//! replayed provider webhook, the compensation sweep, and an operator
//! re-confirmation all converge on one delivery.

use std::future::Future;
use std::pin::Pin;

use sdkwork_contract_service::CommerceServiceError;

pub type VirtualGoodsFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

/// One paid virtual-goods order awaiting entitlement delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FulfillPaidVirtualOrderRequest {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
    pub paid_at: String,
    pub request_no: String,
    pub idempotency_key: String,
}

/// Outcome of a virtual-goods delivery attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VirtualGoodsFulfillmentOutcome {
    pub accepted: bool,
    /// True when the delivery had already happened and was not repeated.
    pub replayed: bool,
    /// Canonical order fulfillment status after delivery (normally
    /// `fulfilled`).
    pub fulfillment_status: String,
}

/// Cross-capability boundary for delivering a paid virtual order's
/// entitlement (license key, download, quota, membership of an external
/// product, ...).
pub trait VirtualGoodsFulfillmentPort: Send + Sync {
    fn fulfill_paid_virtual_order<'a>(
        &'a self,
        request: FulfillPaidVirtualOrderRequest,
    ) -> VirtualGoodsFuture<'a, VirtualGoodsFulfillmentOutcome>;
}

/// Deployment without a virtual-goods capability wired.
///
/// The notify pipeline treats `provider_unavailable` from this port as
/// "deferred", not "failed": a virtual order whose entitlement owner is not
/// installed still settles its payment and reports
/// `awaiting_virtual_goods_fulfillment`, so the provider is acked and the
/// delivery can be replayed once the capability is wired.
#[derive(Default)]
pub struct UnavailableVirtualGoodsFulfillmentPort;

impl VirtualGoodsFulfillmentPort for UnavailableVirtualGoodsFulfillmentPort {
    fn fulfill_paid_virtual_order<'a>(
        &'a self,
        _request: FulfillPaidVirtualOrderRequest,
    ) -> VirtualGoodsFuture<'a, VirtualGoodsFulfillmentOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::provider_unavailable(
                "virtual goods fulfillment is not configured",
            ))
        })
    }
}

/// Stable idempotency key for one order's virtual delivery.
pub fn virtual_goods_fulfillment_idempotency_key(order_id: &str) -> String {
    format!("virtual-goods:fulfill:{order_id}")
}

pub const VIRTUAL_GOODS_FULFILLMENT_PORT: &str = "commerce.virtual_goods.fulfillment";
