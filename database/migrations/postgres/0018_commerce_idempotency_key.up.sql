-- sdkwork:migration
-- id: 0018_commerce_idempotency_key
-- engine: postgres
-- module: sdkwork-order
-- purpose: Add the idempotency ledger used by checkout/session and owner-order
--   commands. The repository layer reads and writes this table on every
--   idempotent command; without it the checkout API returns an invalid-state
--   error before a session can be created.
-- reversible: false
-- rollback: forward-fix (idempotency locks are disposable; completed responses
--   may be retained)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 60s

CREATE TABLE IF NOT EXISTS commerce_idempotency_key (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    scope TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    response_json TEXT,
    locked_until TEXT,
    expires_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_commerce_idempotency_key_scope
    ON commerce_idempotency_key(tenant_id, scope, idempotency_key);
