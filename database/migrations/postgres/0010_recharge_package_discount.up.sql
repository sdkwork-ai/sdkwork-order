-- sdkwork:migration
-- id: 0010_recharge_package_discount
-- engine: postgres
-- module: sdkwork-order
-- purpose: Add the discount rate percentage column (1-100, 100 = no
--   discount) to commerce_recharge_package. Existing rows default to 100 (no
--   discount); the baseline gains the same column with the same default and
--   CHECK constraint for fresh installs.
-- reversible: false
-- rollback: forward-fix (the column default and CHECK constraint are the
--   canonical state; there is no down migration)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

ALTER TABLE commerce_recharge_package ADD COLUMN IF NOT EXISTS discount BIGINT NOT NULL DEFAULT 100;

ALTER TABLE commerce_recharge_package
    DROP CONSTRAINT IF EXISTS ck_recharge_package_discount_range;

ALTER TABLE commerce_recharge_package
    ADD CONSTRAINT ck_recharge_package_discount_range
    CHECK (discount >= 1 AND discount <= 100);

COMMIT;
