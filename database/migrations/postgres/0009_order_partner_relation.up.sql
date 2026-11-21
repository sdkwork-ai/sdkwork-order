-- sdkwork:migration
-- id: 0009_order_partner_relation
-- engine: postgres
-- module: sdkwork-order
-- purpose: Snapshot the partner bound to the order's customer at order
--   creation time (the order domain records the relation; the partner
--   domain owns binding resolution).
-- reversible: false
-- rollback: forward-fix (partner snapshot columns are additive; there is no
--   down migration)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

ALTER TABLE commerce_order ADD COLUMN IF NOT EXISTS partner_id TEXT;
ALTER TABLE commerce_order ADD COLUMN IF NOT EXISTS partner_snapshot_json TEXT;

CREATE INDEX IF NOT EXISTS idx_order_partner_list
    ON commerce_order(tenant_id, organization_id, partner_id, created_at DESC, id DESC);

COMMIT;
