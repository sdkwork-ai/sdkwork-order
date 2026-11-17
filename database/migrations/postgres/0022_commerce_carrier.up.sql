-- sdkwork:migration
-- id: 0022_commerce_carrier
-- engine: postgres
-- module: sdkwork-order
-- purpose: Materialize the operator carrier registry. Shipments already carry a
--   free-text `carrier_code` and tracking numbers, but no durable record of the
--   carrier itself (display name, tracking URL template, enablement), so the
--   logistics surface could not validate a carrier or render a tracking link.
--   One carrier code is unique per tenant + organization.
-- reversible: false
-- rollback: forward-fix (carrier records are referenced by shipments)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

CREATE TABLE IF NOT EXISTS commerce_carrier (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    carrier_code TEXT NOT NULL,
    carrier_name TEXT NOT NULL,
    tracking_url_template TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    sort_order BIGINT NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_commerce_carrier_code
    ON commerce_carrier (tenant_id, organization_id, carrier_code);

CREATE INDEX IF NOT EXISTS idx_commerce_carrier_tenant_status
    ON commerce_carrier (tenant_id, status, sort_order);

COMMIT;
