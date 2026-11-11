//! Periodic course-entitlement relay.
//!
//! Paid digital orders are projected into the course capability through the
//! course module's in-process entitlement grant port (`MODULE_SPEC.md` §4.2).
//! Order storage stays authoritative for payment, and course storage stays
//! authoritative for enrollment. Both modules run in one process against one
//! PostgreSQL pool, so there is no endpoint URL and no shared secret; delivery
//! is idempotent because the purchase reference is the course enrollment
//! idempotency key.
//!
//! The scan is keyset-paginated: each pass reads the next [`DELIVERY_BATCH`]
//! paid orders strictly after the cursor, asks the course module in one batch
//! which of them it already delivered, grants the missing ones, then advances
//! the cursor to the last id of the page. A short page is the end of the paid
//! order set, so the cursor wraps to the beginning for the next pass. Without
//! that cursor the relay re-selected the same first page forever and every
//! order past it starved.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_integration_course::CourseEntitlementGrantAdapter;
use sqlx::postgres::PgRow;
use sqlx::Row;

use crate::OrderServiceHost;

const DEFAULT_INTERVAL_SECONDS: u64 = 30;
const DELIVERY_BATCH: i64 = 50;

/// The relay awaits the shared PostgreSQL pool; every external await is
/// bounded so a stalled query cannot park the worker task forever
/// (`RUST_CODE_SPEC.md` §8). A timed-out pass is retried by the next tick.
const CAPABILITY_PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const DELIVERY_PASS_TIMEOUT: Duration = Duration::from_secs(120);

/// Presence of this table means the shared database carries the course
/// capability; without it there is nothing to relay into.
const COURSE_CAPABILITY_PROBE_TABLE: &str = "course_offering";

/// Keyset cursor shared by every pass of one relay task.
///
/// `after_order_id` is the last order id of the previously completed page; the
/// next page selects orders strictly greater than it. The cursor is in-memory
/// module state only: it resets on restart, so a restart re-scans from the first
/// paid order. Re-scan is idempotent because the purchase reference is the
/// course enrollment idempotency key, so a restart only replays no-ops.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct DeliveryCursor {
    after_order_id: String,
}

impl DeliveryCursor {
    /// The keyset bound for the next page; empty means "from the beginning".
    fn page_after(&self) -> &str {
        &self.after_order_id
    }

    /// Advances to the last id of the delivered page.
    ///
    /// A page shorter than [`DELIVERY_BATCH`] is the end of the paid order set,
    /// so the cursor wraps to the empty string and the next pass restarts from
    /// the first order. Advancing even when individual grants failed keeps a
    /// permanently failing order from pinning the scan at the same page.
    fn advance(&mut self, page: &[PaidCourseOrder]) {
        match page.last() {
            Some(last) if page.len() >= DELIVERY_BATCH as usize => {
                self.after_order_id.clone_from(&last.purchase_ref);
            }
            _ => self.after_order_id.clear(),
        }
    }
}

/// The two course-module capabilities one delivery page needs.
///
/// The relay is generic over this port so the cursor and batch-selection logic
/// is exercised without a database; [`CourseEntitlementGrantAdapter`] provides
/// the in-process implementation.
#[async_trait::async_trait]
trait CourseEntitlementDelivery: Send + Sync {
    /// Which of `purchase_refs` the course module already delivered for
    /// `tenant_id`, in one batch call.
    async fn list_granted_purchase_refs(
        &self,
        tenant_id: &str,
        purchase_refs: &[String],
    ) -> Result<HashSet<String>, String>;

    /// Delivers one purchase; a replay returns the existing enrollment.
    async fn grant_entitlement(
        &self,
        offering_id: &str,
        learner_user_id: &str,
        purchase_ref: &str,
    ) -> Result<String, String>;
}

#[async_trait::async_trait]
impl CourseEntitlementDelivery for CourseEntitlementGrantAdapter {
    async fn list_granted_purchase_refs(
        &self,
        tenant_id: &str,
        purchase_refs: &[String],
    ) -> Result<HashSet<String>, String> {
        CourseEntitlementGrantAdapter::already_granted_purchase_refs(self, tenant_id, purchase_refs)
            .await
    }

    async fn grant_entitlement(
        &self,
        offering_id: &str,
        learner_user_id: &str,
        purchase_ref: &str,
    ) -> Result<String, String> {
        CourseEntitlementGrantAdapter::grant_entitlement(
            self,
            offering_id,
            learner_user_id,
            purchase_ref,
        )
        .await
    }
}

/// Starts the relay on the host's PostgreSQL pool.
///
/// Returns `None` for deployments that do not mount a course capability: a
/// non-PostgreSQL (client-local) pool, or a database that carries no course
/// schema. Those deployments keep the pre-in-process behaviour of not running a
/// relay at all instead of failing a course query on every pass.
pub async fn spawn_course_entitlement_relay(
    host: Arc<OrderServiceHost>,
) -> Option<tokio::task::JoinHandle<()>> {
    #[allow(unreachable_patterns)]
    let pool = match host.database_pool() {
        DatabasePool::Postgres(pool, _) => pool.clone(),
        _ => return None,
    };
    if !course_capability_present(&pool).await {
        tracing::debug!("course capability is not mounted; course entitlement relay is disabled");
        return None;
    }
    let grant = CourseEntitlementGrantAdapter::new(pool.clone());
    let configured_interval = std::env::var("SDKWORK_COURSE_ENTITLEMENT_RELAY_SECONDS").ok();
    let interval_seconds = relay_interval_seconds(configured_interval.as_deref());
    Some(tokio::spawn(async move {
        // One cursor for the lifetime of the task: passes walk the paid order
        // set forward instead of re-selecting the same lowest page.
        let mut cursor = DeliveryCursor::default();
        loop {
            match tokio::time::timeout(
                DELIVERY_PASS_TIMEOUT,
                deliver_paid_course_orders(&pool, &grant, &mut cursor),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => tracing::warn!(%error, "course entitlement relay pass failed"),
                Err(_) => tracing::warn!("course entitlement relay pass timed out"),
            }
            tokio::time::sleep(Duration::from_secs(interval_seconds)).await;
        }
    }))
}

/// Parses the configured interval with the clamp the HTTP relay already had:
/// anything below five seconds falls back to the default cadence.
fn relay_interval_seconds(configured: Option<&str>) -> u64 {
    configured
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value >= 5)
        .unwrap_or(DEFAULT_INTERVAL_SECONDS)
}

async fn course_capability_present(pool: &sqlx::PgPool) -> bool {
    let probe = sqlx::query_scalar::<_, bool>("SELECT to_regclass($1) IS NOT NULL")
        .bind(COURSE_CAPABILITY_PROBE_TABLE)
        .fetch_one(pool);
    match tokio::time::timeout(CAPABILITY_PROBE_TIMEOUT, probe).await {
        Ok(Ok(present)) => present,
        Ok(Err(error)) => {
            tracing::warn!(%error, "course entitlement relay capability probe failed");
            false
        }
        Err(_) => {
            tracing::warn!("course entitlement relay capability probe timed out");
            false
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PaidCourseOrder {
    purchase_ref: String,
    tenant_id: String,
    learner_user_id: String,
    offering_id: String,
}

impl PaidCourseOrder {
    fn from_row(row: &PgRow) -> Result<Self, String> {
        Ok(Self {
            purchase_ref: row
                .try_get("purchase_ref")
                .map_err(|error| error.to_string())?,
            tenant_id: row
                .try_get("tenant_id")
                .map_err(|error| error.to_string())?,
            learner_user_id: row
                .try_get("owner_user_id")
                .map_err(|error| error.to_string())?,
            offering_id: row
                .try_get("offering_id")
                .map_err(|error| error.to_string())?,
        })
    }
}

async fn deliver_paid_course_orders(
    pool: &sqlx::PgPool,
    grant: &CourseEntitlementGrantAdapter,
    cursor: &mut DeliveryCursor,
) -> Result<(), String> {
    let page = load_paid_course_order_page(pool, cursor.page_after()).await?;
    deliver_page(grant, &page, cursor).await;
    Ok(())
}

/// Loads the next keyset page of paid orders.
///
/// `o.id > CAST($2 AS TEXT)` is the cursor predicate; `ORDER BY o.id` makes the
/// result deterministic so the last id of the page is a safe next bound.
async fn load_paid_course_order_page(
    pool: &sqlx::PgPool,
    after_order_id: &str,
) -> Result<Vec<PaidCourseOrder>, String> {
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT o.id AS purchase_ref, o.owner_user_id,
               o.tenant_id, co.id AS offering_id
        FROM commerce_order o
        JOIN commerce_order_item i
          ON i.order_id = o.id AND i.tenant_id = o.tenant_id
        JOIN course_offering co
          ON co.tenant_id = o.tenant_id
         AND co.access_policy_json::jsonb ->> 'commerceSkuId' = i.sku_id
         AND co.deleted_at IS NULL
        WHERE (LOWER(o.status) IN ('paid', 'fulfilled', 'completed')
               OR LOWER(o.payment_status) IN ('success', 'paid', 'succeeded'))
          AND o.id > CAST($2 AS TEXT)
        ORDER BY o.id
        LIMIT $1
        "#,
    )
    .bind(DELIVERY_BATCH)
    .bind(after_order_id)
    .fetch_all(pool)
    .await
    .map_err(|error| format!("load paid course orders: {error}"))?;

    rows.iter().map(PaidCourseOrder::from_row).collect()
}

/// Delivers one page: one batch lookup, grants for the missing purchases, then
/// the cursor advance.
async fn deliver_page<P: CourseEntitlementDelivery + ?Sized>(
    grant: &P,
    page: &[PaidCourseOrder],
    cursor: &mut DeliveryCursor,
) {
    // One lookup per tenant on the page (a page is normally one tenant), never
    // one lookup per order.
    let mut by_tenant: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for order in page {
        by_tenant
            .entry(order.tenant_id.as_str())
            .or_default()
            .push(order.purchase_ref.clone());
    }

    let mut granted = HashSet::new();
    for (tenant_id, purchase_refs) in by_tenant {
        match grant
            .list_granted_purchase_refs(tenant_id, &purchase_refs)
            .await
        {
            Ok(found) => granted.extend(found),
            Err(error) => {
                // The batch lookup is an optimisation, not a gate: a failed
                // lookup must not stall the scan. Treat the page as
                // undelivered and let the idempotent grant answer for it.
                tracing::warn!(%error, %tenant_id, "course entitlement relay batch lookup failed");
            }
        }
    }

    for order in page {
        if granted.contains(&order.purchase_ref) {
            continue;
        }
        // One failing order must not abort the pass: the remaining orders are
        // still delivered, and the next full scan retries the failed one.
        if let Err(error) = grant
            .grant_entitlement(
                &order.offering_id,
                &order.learner_user_id,
                &order.purchase_ref,
            )
            .await
        {
            tracing::warn!(
                %error,
                purchase_ref = %order.purchase_ref,
                "course entitlement relay failed to grant a paid order"
            );
        }
    }

    cursor.advance(page);
}

#[cfg(test)]
mod tests {
    use super::{
        deliver_page, relay_interval_seconds, CourseEntitlementDelivery, DeliveryCursor,
        PaidCourseOrder, DEFAULT_INTERVAL_SECONDS, DELIVERY_BATCH, DELIVERY_PASS_TIMEOUT,
    };
    use std::collections::HashSet;
    use std::sync::Mutex;

    const TENANT: &str = "100001";

    fn order(index: usize) -> PaidCourseOrder {
        PaidCourseOrder {
            purchase_ref: format!("order-{index:04}"),
            tenant_id: TENANT.to_owned(),
            learner_user_id: format!("learner-{index:04}"),
            offering_id: format!("offering-{index:04}"),
        }
    }

    fn paid_orders(count: usize) -> Vec<PaidCourseOrder> {
        (1..=count).map(order).collect()
    }

    /// The keyset page the relay's query returns for `cursor`.
    fn page_after(rows: &[PaidCourseOrder], cursor: &DeliveryCursor) -> Vec<PaidCourseOrder> {
        rows.iter()
            .filter(|order| order.purchase_ref.as_str() > cursor.page_after())
            .take(DELIVERY_BATCH as usize)
            .cloned()
            .collect()
    }

    /// In-memory stand-in for the course capability: it remembers what was
    /// granted, counts grant calls and serves the batch lookup.
    #[derive(Default)]
    struct FakeCourse {
        granted: Mutex<HashSet<String>>,
        grants: Mutex<Vec<String>>,
        lookups: Mutex<usize>,
        failing: HashSet<String>,
    }

    impl FakeCourse {
        fn with_granted(purchase_refs: &[&str]) -> Self {
            let fake = Self::default();
            fake.granted.lock().expect("granted set").extend(
                purchase_refs
                    .iter()
                    .map(|reference| (*reference).to_owned()),
            );
            fake
        }

        fn granted(&self) -> HashSet<String> {
            self.granted.lock().expect("granted set").clone()
        }

        fn grants(&self) -> Vec<String> {
            self.grants.lock().expect("grant log").clone()
        }

        fn lookups(&self) -> usize {
            *self.lookups.lock().expect("lookup counter")
        }
    }

    #[async_trait::async_trait]
    impl CourseEntitlementDelivery for FakeCourse {
        async fn list_granted_purchase_refs(
            &self,
            _tenant_id: &str,
            purchase_refs: &[String],
        ) -> Result<HashSet<String>, String> {
            *self.lookups.lock().expect("lookup counter") += 1;
            let granted = self.granted.lock().expect("granted set");
            Ok(purchase_refs
                .iter()
                .filter(|purchase_ref| granted.contains(*purchase_ref))
                .cloned()
                .collect())
        }

        async fn grant_entitlement(
            &self,
            _offering_id: &str,
            _learner_user_id: &str,
            purchase_ref: &str,
        ) -> Result<String, String> {
            if self.failing.contains(purchase_ref) {
                return Err("course storage unavailable".to_owned());
            }
            self.grants
                .lock()
                .expect("grant log")
                .push(purchase_ref.to_owned());
            self.granted
                .lock()
                .expect("granted set")
                .insert(purchase_ref.to_owned());
            Ok(format!("enrollment-{purchase_ref}"))
        }
    }

    #[test]
    fn the_relay_reaches_the_course_module_in_process() {
        let source = include_str!("course_entitlement_relay.rs");
        // Regression guard for MODULE_SPEC.md §4.2: the order module must not
        // address the course module over HTTP or depend on a shared secret.
        assert!(!source.contains(concat!("SDKWORK_COURSE_ENTITLEMENT_", "URL")));
        assert!(!source.contains(concat!("SDKWORK_COURSE_ENTITLEMENT_", "SECRET")));
        assert!(!source.contains(concat!("req", "west")));
        assert!(source.contains("CourseEntitlementGrantAdapter"));
        assert!(source.contains("grant_entitlement"));
        // The purchase reference stays the delivery key and the projection
        // query keeps its shape, now with the keyset bound.
        assert!(source.contains("purchase_ref"));
        assert!(source.contains("JOIN course_offering"));
        assert!(source.contains("LIMIT $1"));
        assert!(source.contains("o.id > CAST($2 AS TEXT)"));
        assert!(source.contains("ORDER BY o.id"));
    }

    #[test]
    fn the_delivery_batch_and_interval_are_preserved() {
        assert_eq!(DELIVERY_BATCH, 50);
        assert_eq!(DEFAULT_INTERVAL_SECONDS, 30);
    }

    #[test]
    fn every_relay_await_is_bounded() {
        let source = include_str!("course_entitlement_relay.rs");
        // The HTTP timeout is gone with the HTTP client; the shared-pool awaits
        // that replaced it must be bounded instead of relying on a healthy
        // connection forever.
        assert!(source.contains("tokio::time::timeout"));
        assert!(source.contains("DELIVERY_PASS_TIMEOUT"));
        assert!(source.contains("CAPABILITY_PROBE_TIMEOUT"));
        assert_eq!(DELIVERY_PASS_TIMEOUT.as_secs(), 120);
    }

    #[test]
    fn the_interval_keeps_its_lower_clamp() {
        assert_eq!(relay_interval_seconds(None), DEFAULT_INTERVAL_SECONDS);
        assert_eq!(
            relay_interval_seconds(Some("not-a-number")),
            DEFAULT_INTERVAL_SECONDS
        );
        assert_eq!(relay_interval_seconds(Some("0")), DEFAULT_INTERVAL_SECONDS);
        assert_eq!(relay_interval_seconds(Some("4")), DEFAULT_INTERVAL_SECONDS);
        assert_eq!(relay_interval_seconds(Some("5")), 5);
        assert_eq!(relay_interval_seconds(Some("300")), 300);
    }

    /// The starvation defect: without a cursor, the lowest 50 orders were
    /// re-selected on every pass and order #51 onward was never granted. 120
    /// orders must all be granted across three keyset passes, each exactly once.
    #[tokio::test]
    async fn all_paid_orders_are_granted_across_three_cursor_passes() {
        let rows = paid_orders(120);
        let fake = FakeCourse::default();
        let mut cursor = DeliveryCursor::default();
        let mut per_pass = Vec::new();

        for _ in 0..3 {
            let page = page_after(&rows, &cursor);
            assert!(page.len() <= DELIVERY_BATCH as usize);
            let before = fake.grants().len();
            deliver_page(&fake, &page, &mut cursor).await;
            per_pass.push(fake.grants().len() - before);
        }

        assert_eq!(
            per_pass,
            vec![50, 50, 20],
            "one page per pass, no order repeated within a pass"
        );
        assert_eq!(cursor.page_after(), "", "the short third page wraps");

        let grants = fake.grants();
        assert_eq!(grants.len(), 120, "every paid order is granted");
        let unique: HashSet<&String> = grants.iter().collect();
        assert_eq!(unique.len(), 120, "no order is granted twice");
        let granted = fake.granted();
        for order in &rows {
            assert!(
                granted.contains(&order.purchase_ref),
                "order {} starved",
                order.purchase_ref
            );
        }
    }

    #[tokio::test]
    async fn a_short_page_wraps_the_cursor_and_a_restart_rescans_from_the_start() {
        let rows = paid_orders(120);
        let fake = FakeCourse::default();
        let mut cursor = DeliveryCursor::default();

        for _ in 0..2 {
            let page = page_after(&rows, &cursor);
            deliver_page(&fake, &page, &mut cursor).await;
        }
        assert_eq!(cursor.page_after(), "order-0100");

        let tail = page_after(&rows, &cursor);
        assert!(tail.len() < DELIVERY_BATCH as usize);
        deliver_page(&fake, &tail, &mut cursor).await;
        assert_eq!(cursor.page_after(), "", "a short page ends the scan");

        // A restart loses the in-memory cursor; the fresh state starts from the
        // beginning again and the re-scan completes without error.
        let mut restarted = DeliveryCursor::default();
        assert_eq!(restarted.page_after(), "");
        let first_page = page_after(&rows, &restarted);
        assert_eq!(first_page[0].purchase_ref, "order-0001");

        // Re-scan is idempotent: everything the restart touches is already
        // delivered, so the batch lookup reports it and no grant is issued.
        let before = fake.grants().len();
        deliver_page(&fake, &first_page, &mut restarted).await;
        assert_eq!(fake.grants().len(), before);
        assert_eq!(restarted.page_after(), "order-0050");
    }

    #[tokio::test]
    async fn an_already_granted_page_costs_no_grant_and_one_batch_lookup() {
        let rows = paid_orders(120);
        let delivered: Vec<&str> = rows
            .iter()
            .take(DELIVERY_BATCH as usize)
            .map(|order| order.purchase_ref.as_str())
            .collect();
        let fake = FakeCourse::with_granted(&delivered);
        let mut cursor = DeliveryCursor::default();

        let page = page_after(&rows, &cursor);
        deliver_page(&fake, &page, &mut cursor).await;

        assert!(
            fake.grants().is_empty(),
            "a delivered page must not re-grant"
        );
        assert_eq!(
            fake.lookups(),
            1,
            "the page is resolved by one batch lookup, not one per order"
        );
        assert_eq!(cursor.page_after(), "order-0050");
    }

    #[tokio::test]
    async fn one_failing_order_does_not_abort_the_pass() {
        let rows = paid_orders(5);
        let mut fake = FakeCourse::default();
        fake.failing.insert("order-0002".to_owned());
        let mut cursor = DeliveryCursor::default();

        deliver_page(&fake, &rows, &mut cursor).await;

        let granted = fake.granted();
        assert_eq!(granted.len(), 4, "the other four orders are still granted");
        assert!(!granted.contains("order-0002"));
        assert_eq!(cursor.page_after(), "", "a short page still wraps");
    }
}
