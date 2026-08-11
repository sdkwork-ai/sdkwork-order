//! SQLx-backed `OrderPartnerRelationPort` implementation.

use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_service::{
    OrderPartnerRelationFuture, OrderPartnerRelationPort, OrderPartnerSnapshot,
};

use sdkwork_commerce_partner_repository_sqlx::PostgresPartnerRelationResolver;
use sdkwork_commerce_partner_service::ports::PartnerRelationResolvePort;

/// Resolves the customer's active partner binding through the partner domain
/// resolver on the shared federated commerce pool.
#[derive(Debug)]
pub struct StoreOrderPartnerRelationAdapter {
    resolver: PostgresPartnerRelationResolver,
}

impl StoreOrderPartnerRelationAdapter {
    pub fn new(pool: DatabasePool) -> Self {
        // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
        let DatabasePool::Postgres(pool, _) = pool else {
            panic!("order partner relation adapter requires a postgres pool")
        };
        Self {
            resolver: PostgresPartnerRelationResolver::new(pool),
        }
    }
}

impl OrderPartnerRelationPort for StoreOrderPartnerRelationAdapter {
    fn resolve_order_partner<'a>(
        &'a self,
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
    ) -> OrderPartnerRelationFuture<'a, Option<OrderPartnerSnapshot>> {
        // Clone the text ids so the async block does not borrow the method
        // arguments (which may outlive the returned future).
        let tenant_id = tenant_id.to_owned();
        let organization_id = organization_id.map(str::to_owned);
        let owner_user_id = owner_user_id.to_owned();
        Box::pin(async move {
            // Order text ids map onto partner numeric ids; a tenant/user that
            // cannot be parsed has no partner relation by definition.
            let Ok(tenant_id) = tenant_id.parse::<i64>() else {
                return Ok(None);
            };
            let organization_id = organization_id
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(0);
            let Ok(customer_user_id) = owner_user_id.parse::<i64>() else {
                return Ok(None);
            };

            let Some(snapshot) = self
                .resolver
                .resolve_customer_partner(tenant_id, organization_id, customer_user_id)
                .await?
            else {
                return Ok(None);
            };

            Ok(Some(OrderPartnerSnapshot {
                partner_id: snapshot.partner_id.to_string(),
                name: snapshot.name,
                level_no: snapshot.level_no.to_string(),
                status: snapshot.status,
            }))
        })
    }
}
