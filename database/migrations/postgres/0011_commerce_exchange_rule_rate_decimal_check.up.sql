-- sdkwork:migration
-- id: 0011_commerce_exchange_rule_rate_decimal_check
-- engine: postgres
-- module: sdkwork-order
-- purpose: Enforce the industry-standard exact-decimal exchange rate shape
--   at the database layer: `rate` must be a positive decimal with at most 6
--   fractional digits, matching the admin API pattern and the i128
--   fixed-point compute path shared by the order read store and the Cloud
--   Router admin store. Existing rows that are non-decimal or non-positive
--   are defensively corrected to the platform default rate '1' before the
--   CHECK is added; the baseline gains the same constraint for fresh
--   installs.
-- reversible: false
-- rollback: forward-fix (the CHECK constraint is the canonical state; the
--   defensive data correction cannot be reversed by a down migration)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

-- First pass: repair rows that are not a plain positive decimal string.
-- The regex guards the numeric cast below so malformed text never reaches
-- `rate::numeric` (e.g. 'abc' would raise an invalid input syntax error).
UPDATE commerce_exchange_rule
   SET rate = '1'
 WHERE rate !~ '^[0-9]+(\.[0-9]{1,6})?$';

-- Second pass: repair well-formed rows that are not positive ('0', '0.0').
UPDATE commerce_exchange_rule
   SET rate = '1'
 WHERE rate ~ '^[0-9]+(\.[0-9]{1,6})?$'
   AND rate::numeric <= 0;

ALTER TABLE commerce_exchange_rule
    DROP CONSTRAINT IF EXISTS ck_commerce_exchange_rule_rate_decimal;

ALTER TABLE commerce_exchange_rule
    ADD CONSTRAINT ck_commerce_exchange_rule_rate_decimal
    CHECK (rate ~ '^[0-9]+(\.[0-9]{1,6})?$' AND rate::numeric > 0);

COMMIT;
