-- sdkwork:migration
-- id: 0017_order_instant_columns_native_timestamptz
-- engine: postgres
-- module: sdkwork-order
-- purpose: Promote the remaining order-owned instant columns to native
--   TIMESTAMPTZ (DATABASE_SPEC 8.1), completing what 0015 started for
--   `commerce_order`. Legacy rows carry either unix-seconds text or canonical
--   RFC 3339 text; the USING clause parses both. Writes bind canonical RFC
--   3339 UTC text through explicit CASTs and reads project back to the same
--   canonical text, so consumer-visible values are unchanged.
--   `commerce_inventory_reservation` is intentionally excluded: the table is
--   owned by the sdkwork-inventory module baseline (its own migration
--   authority); the order queries that touch it use explicit casts.
-- reversible: false
-- rollback: forward-fix (instants re-encode to canonical text losslessly)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 60s

BEGIN;
ALTER TABLE commerce_order_item
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_checkout_session
    ALTER COLUMN expires_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN expires_at ~ '^[0-9]+$' THEN to_timestamp(expires_at::bigint)
        ELSE NULLIF(expires_at, '')::timestamptz
    END;
ALTER TABLE commerce_checkout_session
    ALTER COLUMN submitted_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN submitted_at ~ '^[0-9]+$' THEN to_timestamp(submitted_at::bigint)
        ELSE NULLIF(submitted_at, '')::timestamptz
    END;
ALTER TABLE commerce_checkout_session
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_checkout_session
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN updated_at ~ '^[0-9]+$' THEN to_timestamp(updated_at::bigint)
        ELSE NULLIF(updated_at, '')::timestamptz
    END;
ALTER TABLE commerce_checkout_line
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_checkout_line
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN updated_at ~ '^[0-9]+$' THEN to_timestamp(updated_at::bigint)
        ELSE NULLIF(updated_at, '')::timestamptz
    END;
ALTER TABLE commerce_checkout_quote
    ALTER COLUMN expires_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN expires_at ~ '^[0-9]+$' THEN to_timestamp(expires_at::bigint)
        ELSE NULLIF(expires_at, '')::timestamptz
    END;
ALTER TABLE commerce_checkout_quote
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_fulfillment_order
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_fulfillment_order
    ALTER COLUMN completed_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN completed_at ~ '^[0-9]+$' THEN to_timestamp(completed_at::bigint)
        ELSE NULLIF(completed_at, '')::timestamptz
    END;
ALTER TABLE commerce_fulfillment_order
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN updated_at ~ '^[0-9]+$' THEN to_timestamp(updated_at::bigint)
        ELSE NULLIF(updated_at, '')::timestamptz
    END;
ALTER TABLE commerce_recharge_package
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_recharge_package
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN updated_at ~ '^[0-9]+$' THEN to_timestamp(updated_at::bigint)
        ELSE NULLIF(updated_at, '')::timestamptz
    END;
ALTER TABLE commerce_account_value_package
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_account_value_package
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN updated_at ~ '^[0-9]+$' THEN to_timestamp(updated_at::bigint)
        ELSE NULLIF(updated_at, '')::timestamptz
    END;
ALTER TABLE commerce_token_bank_plan
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_token_bank_plan
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN updated_at ~ '^[0-9]+$' THEN to_timestamp(updated_at::bigint)
        ELSE NULLIF(updated_at, '')::timestamptz
    END;
ALTER TABLE commerce_order_refund_request
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_order_refund_request
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN updated_at ~ '^[0-9]+$' THEN to_timestamp(updated_at::bigint)
        ELSE NULLIF(updated_at, '')::timestamptz
    END;
ALTER TABLE commerce_order_withdrawal_request
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_order_withdrawal_request
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN updated_at ~ '^[0-9]+$' THEN to_timestamp(updated_at::bigint)
        ELSE NULLIF(updated_at, '')::timestamptz
    END;
ALTER TABLE commerce_after_sales_request
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_after_sales_request
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN updated_at ~ '^[0-9]+$' THEN to_timestamp(updated_at::bigint)
        ELSE NULLIF(updated_at, '')::timestamptz
    END;
ALTER TABLE commerce_after_sales_request_item
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_after_sales_event
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_after_sales_return_shipment
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_after_sales_return_shipment
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN updated_at ~ '^[0-9]+$' THEN to_timestamp(updated_at::bigint)
        ELSE NULLIF(updated_at, '')::timestamptz
    END;
ALTER TABLE commerce_shipment
    ALTER COLUMN shipped_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN shipped_at ~ '^[0-9]+$' THEN to_timestamp(shipped_at::bigint)
        ELSE NULLIF(shipped_at, '')::timestamptz
    END;
ALTER TABLE commerce_shipment
    ALTER COLUMN delivered_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN delivered_at ~ '^[0-9]+$' THEN to_timestamp(delivered_at::bigint)
        ELSE NULLIF(delivered_at, '')::timestamptz
    END;
ALTER TABLE commerce_shipment
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_shipment
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN updated_at ~ '^[0-9]+$' THEN to_timestamp(updated_at::bigint)
        ELSE NULLIF(updated_at, '')::timestamptz
    END;
ALTER TABLE commerce_shipment_package
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_shipment_tracking_event
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_order_event
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;
ALTER TABLE commerce_order_cancellation
    ALTER COLUMN created_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN created_at ~ '^[0-9]+$' THEN to_timestamp(created_at::bigint)
        ELSE NULLIF(created_at, '')::timestamptz
    END;

COMMIT;

-- caught in review: catalog retirement instant
ALTER TABLE commerce_account_value_package
    ALTER COLUMN retired_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN retired_at ~ '^[0-9]+$' THEN to_timestamp(retired_at::bigint)
        ELSE NULLIF(retired_at, '')::timestamptz
    END;

-- caught in review: catalog retirement instant
ALTER TABLE commerce_token_bank_plan
    ALTER COLUMN retired_at TYPE TIMESTAMPTZ
    USING CASE
        WHEN retired_at ~ '^[0-9]+$' THEN to_timestamp(retired_at::bigint)
        ELSE NULLIF(retired_at, '')::timestamptz
    END;
