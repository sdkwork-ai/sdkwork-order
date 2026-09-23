mod checkout_resolver;
mod fulfillment;
mod inventory;

pub use checkout_resolver::PhysicalCheckoutAdapter;
pub use fulfillment::PhysicalFulfillmentAdapter;
pub use inventory::PhysicalInventoryAdapter;

use std::sync::Arc;

use sdkwork_database_config::DatabaseConfig;
use sdkwork_database_id::IdGenerator;
use sdkwork_database_sqlx::{create_pool_from_config, DatabasePool};
use sdkwork_order_service::{
    PhysicalCheckoutResolverPort, PhysicalGoodsFulfillmentPort, PhysicalInventoryReservationPort,
};

pub struct PhysicalCommercePorts {
    pub checkout_resolver: Arc<dyn PhysicalCheckoutResolverPort>,
    pub inventory: Arc<dyn PhysicalInventoryReservationPort>,
    pub fulfillment: Arc<dyn PhysicalGoodsFulfillmentPort>,
}

/// Builds the physical-commerce ports.
///
/// `ids` is the process Snowflake identity: the checkout resolver writes through the merchandise
/// catalog tables, whose primary keys are minted by the caller's node id. It is injected rather than
/// created here so one process never runs two divergent generators.
pub async fn physical_commerce_ports_from_env(
    order_pool: &DatabasePool,
    ids: Arc<dyn IdGenerator>,
) -> Result<PhysicalCommercePorts, String> {
    let merchandise_pool = dependency_pool("MERCHANDISE").await?;
    let shop_pool = dependency_pool("SHOP").await?;
    let inventory_pool = dependency_pool("INVENTORY").await?;

    Ok(PhysicalCommercePorts {
        checkout_resolver: Arc::new(PhysicalCheckoutAdapter::new(merchandise_pool, shop_pool, ids)),
        inventory: Arc::new(PhysicalInventoryAdapter::new(inventory_pool.clone())),
        fulfillment: Arc::new(PhysicalFulfillmentAdapter::new(
            inventory_pool,
            order_pool.clone(),
        )),
    })
}

async fn dependency_pool(prefix: &str) -> Result<DatabasePool, String> {
    let config = DatabaseConfig::from_env(prefix)
        .map_err(|error| format!("read {prefix} database config failed: {error}"))?;
    create_pool_from_config(config)
        .await
        .map_err(|error| format!("create {prefix} database pool failed: {error}"))
}
