-- sdkwork:migration
-- id: 0007_commerce_exchange_rule
-- engine: postgres
-- module: sdkwork-order
-- purpose: Create the cash-to-points exchange rule projection table
--   (`commerce_exchange_rule`) shared by the order read store and the Cloud
--   Router admin store. The table was previously missing from every DDL
--   authority, which surfaced as HTTP 500 on the admin recharge settings and
--   package list paths (SQL 42P01).
-- reversible: false
-- rollback: forward-fix
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

CREATE TABLE IF NOT EXISTS commerce_exchange_rule (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    rule_no TEXT NOT NULL,
    source_asset_type TEXT NOT NULL,
    target_asset_type TEXT NOT NULL,
    rate TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    remark TEXT,
    request_no TEXT,
    idempotency_key TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_exchange_rule_pair
    ON commerce_exchange_rule(tenant_id, organization_id, source_asset_type, target_asset_type);

CREATE INDEX IF NOT EXISTS idx_exchange_rule_list
    ON commerce_exchange_rule(tenant_id, organization_id, status, updated_at, id);

COMMIT;
