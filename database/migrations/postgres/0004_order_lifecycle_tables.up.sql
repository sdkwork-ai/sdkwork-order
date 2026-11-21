-- sdkwork:migration
-- id: 0004_order_lifecycle_tables
-- engine: postgres
-- module: sdkwork-order
-- purpose: Backfill the order lifecycle audit tables (order_event and
--   order_cancellation) on deployments initialized before the consolidated
--   baseline included them.
-- reversible: false
-- rollback: forward-fix (the lifecycle tables are the canonical state; there
--   is no down migration)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

CREATE TABLE IF NOT EXISTS commerce_order_event (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT,
    event_no TEXT NOT NULL,
    order_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    from_status TEXT,
    to_status TEXT NOT NULL,
    actor_type TEXT,
    actor_id TEXT,
    reason_code TEXT,
    message TEXT,
    payload_json TEXT NOT NULL DEFAULT '{}',
    request_id TEXT,
    idempotency_key TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_order_event_order
    ON commerce_order_event(tenant_id, order_id, event_type, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_order_event_tenant_type
    ON commerce_order_event(tenant_id, event_type, created_at DESC, id DESC);

CREATE TABLE IF NOT EXISTS commerce_order_cancellation (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    order_id TEXT NOT NULL,
    status TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    reason_message TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_order_cancellation_order
    ON commerce_order_cancellation(tenant_id, order_id, status, created_at DESC, id DESC);

COMMIT;
