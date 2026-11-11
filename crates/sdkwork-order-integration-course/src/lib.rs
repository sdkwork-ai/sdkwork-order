//! Order -> Course entitlement grant integration.
//!
//! `MODULE_SPEC.md` §4.2: a packaged deployment runs one backend process against
//! one PostgreSQL, so the order module reaches the course capability
//! **in-process** through this adapter instead of POSTing paid orders to the
//! course App API grant endpoint with a shared secret. The adapter borrows the
//! shared pool, calls the course repository's grant port, and therefore needs no
//! endpoint URL and no service credential.

use std::collections::HashSet;

use sdkwork_content_course_repository_sqlx::PostgresCourseRepository;
use sdkwork_content_course_service::CourseEntitlementGrantRepository;
use sdkwork_database_sqlx::DatabasePool;
use sqlx::PgPool;

/// In-process adapter for the course purchase projection.
pub struct CourseEntitlementGrantAdapter {
    repository: PostgresCourseRepository,
}

impl CourseEntitlementGrantAdapter {
    /// Builds the adapter on the process pool shared with the course capability.
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: PostgresCourseRepository::new(pool),
        }
    }

    /// Builds the adapter from a host database pool.
    ///
    /// Authoritative server persistence is PostgreSQL only (`DATABASE_SPEC`), so
    /// a non-PostgreSQL pool has no course capability to reach.
    pub fn from_database_pool(pool: &DatabasePool) -> Result<Self, String> {
        let pool = pool.as_postgres().ok_or_else(|| {
            "the course entitlement grant adapter requires a PostgreSQL pool \
             (DATABASE_SPEC: authoritative-server)"
                .to_owned()
        })?;
        Ok(Self::new(pool.clone()))
    }

    /// Grants the purchased offering to the learner.
    ///
    /// `purchase_ref` is the course enrollment idempotency key, so replaying the
    /// same order returns the enrollment created by the first delivery.
    pub async fn grant_entitlement(
        &self,
        offering_id: &str,
        learner_user_id: &str,
        purchase_ref: &str,
    ) -> Result<String, String> {
        self.repository
            .grant_entitlement_by_offering(
                offering_id.to_owned(),
                learner_user_id.to_owned(),
                purchase_ref.to_owned(),
            )
            .await
            .map_err(|error| format!("{}: {}", error.code(), error.message()))
    }

    /// Which of `purchase_refs` the course capability already delivered for the
    /// tenant, resolved in one tenant-scoped batch query.
    pub async fn already_granted_purchase_refs(
        &self,
        tenant_id: &str,
        purchase_refs: &[String],
    ) -> Result<HashSet<String>, String> {
        self.repository
            .list_granted_purchase_refs(tenant_id, purchase_refs)
            .await
            .map_err(|error| format!("{}: {}", error.code(), error.message()))
    }
}
