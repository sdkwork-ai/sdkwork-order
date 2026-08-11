//! Order → Partner relation integration adapter.
//!
//! Resolves the active customer->partner binding through the partner domain
//! repository and maps it to the order-side snapshot. The dependency edge is
//! one-directional: `sdkwork-order` consumes the partner capability through
//! this adapter only; the partner domain never reads order tables.

mod resolver;

pub use resolver::StoreOrderPartnerRelationAdapter;

use std::sync::Arc;

use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service::OrderPartnerRelationPort;

/// Builds the order partner relation port on a shared commerce pool.
///
/// The adapter reads the partner domain on the same federated commerce
/// database; no extra environment configuration is required.
pub fn order_partner_relation_port_from_database_pool(
    pool: &DatabasePool,
) -> Arc<dyn OrderPartnerRelationPort> {
    Arc::new(StoreOrderPartnerRelationAdapter::new(pool.clone()))
}
