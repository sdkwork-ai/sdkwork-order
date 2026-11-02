-- sdkwork:migration
-- id: 0020_demand_claim_app_forms
-- engine: postgres
-- module: sdkwork-order
-- purpose: Snapshot every application form selected by the demand (not just
--   the first one) on the claim. `app_form` remains a compatibility projection
--   of the first form for older SDK consumers.
-- reversible: false
-- rollback: forward-fix (app_forms is canonical; app_form remains a projection)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

ALTER TABLE commerce_demand_claim
    ADD COLUMN IF NOT EXISTS app_forms TEXT[] NOT NULL DEFAULT '{}';

UPDATE commerce_demand_claim
SET app_forms = ARRAY[app_form]
WHERE app_form IS NOT NULL
  AND btrim(app_form) <> ''
  AND cardinality(app_forms) = 0;

CREATE INDEX IF NOT EXISTS idx_commerce_demand_claim_app_forms
    ON commerce_demand_claim USING GIN (app_forms);

COMMIT;
