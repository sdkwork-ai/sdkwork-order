use std::fmt;
use std::future::Future;
use std::pin::Pin;

use sdkwork_contract_service::CommerceServiceError;
use serde::{Deserialize, Serialize};

/// Immutable partner facts snapshotted onto an order at creation time.
///
/// The snapshot keeps the order readable after the partner later changes
/// name, level, or status. `partner_id` is the numeric `partner_partner.id`
/// rendered as text to match order text-id conventions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrderPartnerSnapshot {
    pub partner_id: String,
    pub name: String,
    pub level_no: String,
    pub status: String,
}

pub type OrderPartnerRelationFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

/// Resolves the partner related to an order's customer.
///
/// The port is intentionally narrow: order creation only needs the partner
/// facts snapshot. Implementations resolve the active customer->partner
/// binding on the partner side of the dependency boundary; a missing or
/// inactive binding resolves to `Ok(None)` and never blocks order creation.
pub trait OrderPartnerRelationPort: Send + Sync + fmt::Debug {
    fn resolve_order_partner<'a>(
        &'a self,
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
    ) -> OrderPartnerRelationFuture<'a, Option<OrderPartnerSnapshot>>;
}

/// Default implementation used when no partner integration is configured:
/// orders simply carry no partner relation.
#[derive(Debug)]
pub struct NoopOrderPartnerRelationPort;

impl OrderPartnerRelationPort for NoopOrderPartnerRelationPort {
    fn resolve_order_partner<'a>(
        &'a self,
        _tenant_id: &str,
        _organization_id: Option<&str>,
        _owner_user_id: &str,
    ) -> OrderPartnerRelationFuture<'a, Option<OrderPartnerSnapshot>> {
        Box::pin(async move { Ok(None) })
    }
}

/// Port-name constant for the order partner relation resolution port.
pub const ORDER_PARTNER_RELATION_PORT: &str = "order.partner.relation";
