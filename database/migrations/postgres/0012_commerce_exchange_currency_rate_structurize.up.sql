-- sdkwork:migration
-- id: 0012_commerce_exchange_currency_rate_structurize
-- engine: postgres
-- module: sdkwork-order
-- purpose: Promote the recharge settings payload out of the legacy `remark`
--   JSON blob into structured storage: a `base_currency_code` column on
--   `commerce_exchange_rule` plus the `commerce_exchange_currency_rate`
--   child table, so exchange rates are SQL-enforceable (format + positive
--   CHECK), queryable, and auditable. Existing JSON payloads are migrated
--   into the structured shape and the blob is cleared from `remark`, which
--   becomes a plain free-text note. The platform catalog seed and the DDL
--   baseline are updated to the same canonical shape.
-- reversible: false
-- rollback: forward-fix (structured columns/table are the canonical state;
--   the migrated data is not re-embedded into remark by a down migration)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

ALTER TABLE commerce_exchange_rule
    ADD COLUMN IF NOT EXISTS base_currency_code TEXT;

ALTER TABLE commerce_exchange_rule
    DROP CONSTRAINT IF EXISTS ck_commerce_exchange_rule_base_currency_code;

ALTER TABLE commerce_exchange_rule
    ADD CONSTRAINT ck_commerce_exchange_rule_base_currency_code
    CHECK (base_currency_code IS NULL OR base_currency_code ~ '^[A-Z]{3}$');

CREATE TABLE IF NOT EXISTS commerce_exchange_currency_rate (
    id TEXT NOT NULL PRIMARY KEY,
    rule_id TEXT NOT NULL REFERENCES commerce_exchange_rule(id) ON DELETE CASCADE,
    currency_code TEXT NOT NULL,
    rate TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT ck_commerce_exchange_currency_rate_code
        CHECK (currency_code ~ '^[A-Z]{3}$'),
    CONSTRAINT ck_commerce_exchange_currency_rate_decimal
        CHECK (rate ~ '^[0-9]+(\.[0-9]{1,6})?$' AND rate::numeric > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_exchange_currency_rate_pair
    ON commerce_exchange_currency_rate(rule_id, currency_code);

CREATE INDEX IF NOT EXISTS idx_exchange_currency_rate_rule
    ON commerce_exchange_currency_rate(rule_id);

-- Migrate the base currency out of the legacy remark JSON payload.
-- Only cash-to-points rules ever carried the JSON settings payload.
UPDATE commerce_exchange_rule rule
   SET base_currency_code = COALESCE(
           NULLIF(TRIM(BOTH '"' FROM (
               SELECT value::text
               FROM jsonb_each(remark::jsonb)
               WHERE key = 'baseCurrencyCode'
               LIMIT 1
           )), ''),
           'CNY')
 WHERE LOWER(source_asset_type) = 'cash'
   AND LOWER(target_asset_type) = 'points'
   AND remark IS NOT NULL
   AND remark <> ''
   AND base_currency_code IS NULL;

-- Migrate each currency rate out of the remark payload into the child table.
INSERT INTO commerce_exchange_currency_rate (id, rule_id, currency_code, rate, created_at, updated_at)
SELECT
    'rate-' || rule.id || '-' || rate_entry.key,
    rule.id,
    rate_entry.key,
    TRIM(BOTH '"' FROM rate_entry.value::text),
    rule.updated_at,
    rule.updated_at
FROM commerce_exchange_rule rule
CROSS JOIN LATERAL jsonb_each(
    COALESCE(remark::jsonb, '{}'::jsonb) -> 'currencyToCnyRates'
) AS rate_entry(key, value)
WHERE LOWER(rule.source_asset_type) = 'cash'
  AND LOWER(rule.target_asset_type) = 'points'
  AND rule.remark IS NOT NULL
  AND rule.remark <> ''
ON CONFLICT (rule_id, currency_code) DO UPDATE SET
    rate = EXCLUDED.rate,
    updated_at = EXCLUDED.updated_at;

-- The payload is now structured; clear the legacy JSON blob so no historical
-- document remains in `remark` (which is now a plain free-text note).
UPDATE commerce_exchange_rule
   SET remark = NULL
 WHERE LOWER(source_asset_type) = 'cash'
   AND LOWER(target_asset_type) = 'points'
   AND remark IS NOT NULL
   AND remark <> '';

COMMIT;
