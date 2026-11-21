-- sdkwork:migration
-- id: 0006_organization_id_not_null_legacy_tables
-- engine: postgres
-- module: sdkwork-order
-- purpose: Enforce organization_id NOT NULL DEFAULT on tables created by
--   earlier migrations that predate the standard column contract.
-- reversible: false
-- rollback: forward-fix
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

ALTER TABLE commerce_order_event ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_order_event SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_order_event ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_order_event ALTER COLUMN organization_id SET NOT NULL;

COMMIT;
