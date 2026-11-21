-- sdkwork:migration
-- id: 0014_money_column_format_checks
-- engine: postgres
-- module: sdkwork-order
-- purpose: Enforce the canonical money text format at the database level on
--   every order-domain money column (DATABASE_SPEC 8: the decimal logical
--   type is carried as canonical decimal text -- minor-unit integer or at
--   most two fractional digits -- and never float). `money_amount.rs`
--   normalizes both encodings on read, so legacy major-unit seed rows
--   ('50.00') stay valid while new writes are bounded to the same closed
--   format. Nullable provider passthrough columns keep their NULL option.
-- reversible: false
-- rollback: forward-fix (CHECK constraints are drop-by-name if ever needed)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;
-- commerce_order_item.unit_price_amount
DO $con$
BEGIN
    ALTER TABLE commerce_order_item
        ADD CONSTRAINT ck_commerce_order_item_unit_price_amount_money_format
        CHECK (unit_price_amount IS NULL OR unit_price_amount ~ '^[0-9]+$' OR unit_price_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_order_item.discount_amount
DO $con$
BEGIN
    ALTER TABLE commerce_order_item
        ADD CONSTRAINT ck_commerce_order_item_discount_amount_money_format
        CHECK (discount_amount IS NULL OR discount_amount ~ '^[0-9]+$' OR discount_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_order_item.tax_amount
DO $con$
BEGIN
    ALTER TABLE commerce_order_item
        ADD CONSTRAINT ck_commerce_order_item_tax_amount_money_format
        CHECK (tax_amount IS NULL OR tax_amount ~ '^[0-9]+$' OR tax_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_order_item.total_amount
DO $con$
BEGIN
    ALTER TABLE commerce_order_item
        ADD CONSTRAINT ck_commerce_order_item_total_amount_money_format
        CHECK (total_amount IS NULL OR total_amount ~ '^[0-9]+$' OR total_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_checkout_line.price_amount_snapshot
DO $con$
BEGIN
    ALTER TABLE commerce_checkout_line
        ADD CONSTRAINT ck_commerce_checkout_line_price_amount_snapshot_money_format
        CHECK (price_amount_snapshot ~ '^[0-9]+$' OR price_amount_snapshot ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_checkout_quote.original_amount
DO $con$
BEGIN
    ALTER TABLE commerce_checkout_quote
        ADD CONSTRAINT ck_commerce_checkout_quote_original_amount_money_format
        CHECK (original_amount ~ '^[0-9]+$' OR original_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_checkout_quote.discount_amount
DO $con$
BEGIN
    ALTER TABLE commerce_checkout_quote
        ADD CONSTRAINT ck_commerce_checkout_quote_discount_amount_money_format
        CHECK (discount_amount ~ '^[0-9]+$' OR discount_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_checkout_quote.payable_amount
DO $con$
BEGIN
    ALTER TABLE commerce_checkout_quote
        ADD CONSTRAINT ck_commerce_checkout_quote_payable_amount_money_format
        CHECK (payable_amount ~ '^[0-9]+$' OR payable_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_order_amount_breakdown.original_amount
DO $con$
BEGIN
    ALTER TABLE commerce_order_amount_breakdown
        ADD CONSTRAINT ck_commerce_order_amount_breakdown_original_amount_money_format
        CHECK (original_amount ~ '^[0-9]+$' OR original_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_order_amount_breakdown.discount_amount
DO $con$
BEGIN
    ALTER TABLE commerce_order_amount_breakdown
        ADD CONSTRAINT ck_commerce_order_amount_breakdown_discount_amount_money_format
        CHECK (discount_amount ~ '^[0-9]+$' OR discount_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_order_amount_breakdown.payable_amount
DO $con$
BEGIN
    ALTER TABLE commerce_order_amount_breakdown
        ADD CONSTRAINT ck_commerce_order_amount_breakdown_payable_amount_money_format
        CHECK (payable_amount ~ '^[0-9]+$' OR payable_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_recharge_package.price_amount
DO $con$
BEGIN
    ALTER TABLE commerce_recharge_package
        ADD CONSTRAINT ck_commerce_recharge_package_price_amount_money_format
        CHECK (price_amount ~ '^[0-9]+$' OR price_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_account_value_package.grant_amount
DO $con$
BEGIN
    ALTER TABLE commerce_account_value_package
        ADD CONSTRAINT ck_commerce_account_value_package_grant_amount_money_format
        CHECK (grant_amount ~ '^[0-9]+$' OR grant_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_account_value_package.bonus_amount
DO $con$
BEGIN
    ALTER TABLE commerce_account_value_package
        ADD CONSTRAINT ck_commerce_account_value_package_bonus_amount_money_format
        CHECK (bonus_amount ~ '^[0-9]+$' OR bonus_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_account_value_package.price_amount
DO $con$
BEGIN
    ALTER TABLE commerce_account_value_package
        ADD CONSTRAINT ck_commerce_account_value_package_price_amount_money_format
        CHECK (price_amount ~ '^[0-9]+$' OR price_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_token_bank_plan.grant_amount
DO $con$
BEGIN
    ALTER TABLE commerce_token_bank_plan
        ADD CONSTRAINT ck_commerce_token_bank_plan_grant_amount_money_format
        CHECK (grant_amount ~ '^[0-9]+$' OR grant_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_token_bank_plan.bonus_amount
DO $con$
BEGIN
    ALTER TABLE commerce_token_bank_plan
        ADD CONSTRAINT ck_commerce_token_bank_plan_bonus_amount_money_format
        CHECK (bonus_amount ~ '^[0-9]+$' OR bonus_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_token_bank_plan.price_amount
DO $con$
BEGIN
    ALTER TABLE commerce_token_bank_plan
        ADD CONSTRAINT ck_commerce_token_bank_plan_price_amount_money_format
        CHECK (price_amount ~ '^[0-9]+$' OR price_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_order_refund_request.amount
DO $con$
BEGIN
    ALTER TABLE commerce_order_refund_request
        ADD CONSTRAINT ck_commerce_order_refund_request_amount_money_format
        CHECK (amount IS NULL OR amount ~ '^[0-9]+$' OR amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_order_refund_request.provider_amount
DO $con$
BEGIN
    ALTER TABLE commerce_order_refund_request
        ADD CONSTRAINT ck_commerce_order_refund_request_provider_amount_money_format
        CHECK (provider_amount IS NULL OR provider_amount ~ '^[0-9]+$' OR provider_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_order_withdrawal_request.amount
DO $con$
BEGIN
    ALTER TABLE commerce_order_withdrawal_request
        ADD CONSTRAINT ck_commerce_order_withdrawal_request_amount_money_format
        CHECK (amount IS NULL OR amount ~ '^[0-9]+$' OR amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_order_withdrawal_request.provider_amount
DO $con$
BEGIN
    ALTER TABLE commerce_order_withdrawal_request
        ADD CONSTRAINT ck_commerce_order_withdrawal_request_provider_amount_money_format
        CHECK (provider_amount IS NULL OR provider_amount ~ '^[0-9]+$' OR provider_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_after_sales_request.approved_amount
DO $con$
BEGIN
    ALTER TABLE commerce_after_sales_request
        ADD CONSTRAINT ck_commerce_after_sales_request_approved_amount_money_format
        CHECK (approved_amount ~ '^[0-9]+$' OR approved_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

-- commerce_after_sales_request_item.requested_amount
DO $con$
BEGIN
    ALTER TABLE commerce_after_sales_request_item
        ADD CONSTRAINT ck_commerce_after_sales_request_item_requested_amount_money_format
        CHECK (requested_amount ~ '^[0-9]+$' OR requested_amount ~ '^[0-9]+[.][0-9]{1,2}$');
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- idempotent re-run
    WHEN undefined_table THEN NULL;   -- table arrives with a later migration
END $con$;

COMMIT;
