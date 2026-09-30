-- sdkwork:migration
-- id: 0015_order_time_columns_native_timestamptz
-- engine: postgres
-- module: sdkwork-order
-- purpose: Promote `commerce_order` instant columns to native TIMESTAMPTZ
--   (DATABASE_SPEC 8.1: new L2+ PostgreSQL tables MUST use TIMESTAMPTZ for
--   instants). The e2e/CI schema already models these columns as TIMESTAMPTZ;
--   production converges to it. Legacy rows carry either unix-seconds text or
--   canonical RFC 3339 text; the USING clause parses both. Canonical RFC 3339
--   UTC (millisecond precision) remains the application wire format: writes
--   bind text through explicit CASTs, reads decode and re-format through the
--   shared store clock, so no consumer-visible value changes.
--   The 0013 expiration expression index is replaced by a plain partial
--   b-tree over the now-native column.
-- reversible: false
-- rollback: forward-fix (values survive as instants; a text rebuild would
--   only re-encode the same instants)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 60s

BEGIN;

ALTER TABLE commerce_order
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
        USING CASE
            WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
            ELSE NULLIF(created_at, '')::timestamptz
        END;
ALTER TABLE commerce_order
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ
        USING CASE
            WHEN updated_at ~ '^[0-9]+$' THEN to_timestamp(updated_at::bigint)
            ELSE NULLIF(updated_at, '')::timestamptz
        END;
ALTER TABLE commerce_order
    ALTER COLUMN paid_at TYPE TIMESTAMPTZ
        USING CASE
            WHEN paid_at ~ '^[0-9]+$' THEN to_timestamp(paid_at::bigint)
            ELSE NULLIF(paid_at, '')::timestamptz
        END;
ALTER TABLE commerce_order
    ALTER COLUMN cancelled_at TYPE TIMESTAMPTZ
        USING CASE
            WHEN cancelled_at ~ '^[0-9]+$' THEN to_timestamp(cancelled_at::bigint)
            ELSE NULLIF(cancelled_at, '')::timestamptz
        END;
ALTER TABLE commerce_order
    ALTER COLUMN expired_at TYPE TIMESTAMPTZ
        USING CASE
            WHEN expired_at ~ '^[0-9]+$' THEN to_timestamp(expired_at::bigint)
            ELSE NULLIF(expired_at, '')::timestamptz
        END;

-- The 0013 expression index wrapped the TEXT column; the native column takes
-- a plain partial b-tree (rebuilds of dependent indexes happen automatically
-- during the ALTER TYPE statements above).
DROP INDEX IF EXISTS idx_order_expiration_due;
CREATE INDEX IF NOT EXISTS idx_order_expiration_due
    ON commerce_order (expired_at, id)
    WHERE LOWER(COALESCE(status, ''))
          IN ('draft', 'pending', 'pending_payment', 'unpaid', 'wait_pay');

COMMIT;
