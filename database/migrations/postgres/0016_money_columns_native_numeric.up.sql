-- sdkwork:migration
-- id: 0016_money_columns_native_numeric
-- engine: postgres
-- module: sdkwork-order
-- purpose: Promote every order-domain money column to native NUMERIC(20,0)
--   (DATABASE_SPEC 8.1; 325 "documented scaled integer"). Store semantics are
--   documented scaled integers: canonical minor units (1 CNY = 100 minor),
--   exactly what `money_amount.rs` reads and writes. Legacy major-unit
--   decimal rows ('50.00') normalize to minor units first (x100); canonical
--   integer rows pass through untouched. Reads keep the wire-identical text
--   form via `CAST(col AS TEXT)`; writes bind canonical minor-unit text
--   through `CAST($n AS NUMERIC)`. The 0014 text-format CHECK constraints
--   MUST be dropped before the ALTER TYPE (ALTER re-validates CHECK
--   expressions against the NEW column type, where `~ text` no longer
--   resolves) and are superseded by the numeric domain plus non-negative
--   checks.
-- reversible: false
-- rollback: forward-fix (minor-unit values re-encode to text losslessly)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 60s

BEGIN;

-- Phase A: drop the 0014 text-format CHECK constraints (their expressions
-- would fail to re-resolve against the numeric column type during ALTER).

ALTER TABLE commerce_order_item DROP CONSTRAINT IF EXISTS ck_commerce_order_item_unit_price_amount_money_format;
ALTER TABLE commerce_order_item DROP CONSTRAINT IF EXISTS ck_commerce_order_item_discount_amount_money_format;
ALTER TABLE commerce_order_item DROP CONSTRAINT IF EXISTS ck_commerce_order_item_tax_amount_money_format;
ALTER TABLE commerce_order_item DROP CONSTRAINT IF EXISTS ck_commerce_order_item_total_amount_money_format;
ALTER TABLE commerce_checkout_line DROP CONSTRAINT IF EXISTS ck_commerce_checkout_line_price_amount_snapshot_money_format;
ALTER TABLE commerce_checkout_quote DROP CONSTRAINT IF EXISTS ck_commerce_checkout_quote_original_amount_money_format;
ALTER TABLE commerce_checkout_quote DROP CONSTRAINT IF EXISTS ck_commerce_checkout_quote_discount_amount_money_format;
ALTER TABLE commerce_checkout_quote DROP CONSTRAINT IF EXISTS ck_commerce_checkout_quote_payable_amount_money_format;
ALTER TABLE commerce_order_amount_breakdown DROP CONSTRAINT IF EXISTS ck_commerce_order_amount_breakdown_original_amount_money_format;
ALTER TABLE commerce_order_amount_breakdown DROP CONSTRAINT IF EXISTS ck_commerce_order_amount_breakdown_discount_amount_money_format;
ALTER TABLE commerce_order_amount_breakdown DROP CONSTRAINT IF EXISTS ck_commerce_order_amount_breakdown_payable_amount_money_format;
ALTER TABLE commerce_recharge_package DROP CONSTRAINT IF EXISTS ck_commerce_recharge_package_price_amount_money_format;
ALTER TABLE commerce_account_value_package DROP CONSTRAINT IF EXISTS ck_commerce_account_value_package_grant_amount_money_format;
ALTER TABLE commerce_account_value_package DROP CONSTRAINT IF EXISTS ck_commerce_account_value_package_bonus_amount_money_format;
ALTER TABLE commerce_account_value_package DROP CONSTRAINT IF EXISTS ck_commerce_account_value_package_price_amount_money_format;
ALTER TABLE commerce_token_bank_plan DROP CONSTRAINT IF EXISTS ck_commerce_token_bank_plan_grant_amount_money_format;
ALTER TABLE commerce_token_bank_plan DROP CONSTRAINT IF EXISTS ck_commerce_token_bank_plan_bonus_amount_money_format;
ALTER TABLE commerce_token_bank_plan DROP CONSTRAINT IF EXISTS ck_commerce_token_bank_plan_price_amount_money_format;
ALTER TABLE commerce_order_refund_request DROP CONSTRAINT IF EXISTS ck_commerce_order_refund_request_amount_money_format;
ALTER TABLE commerce_order_refund_request DROP CONSTRAINT IF EXISTS ck_commerce_order_refund_request_provider_amount_money_format;
ALTER TABLE commerce_order_withdrawal_request DROP CONSTRAINT IF EXISTS ck_commerce_order_withdrawal_request_amount_money_format;
ALTER TABLE commerce_order_withdrawal_request DROP CONSTRAINT IF EXISTS ck_commerce_order_withdrawal_request_provider_amount_money_format;
ALTER TABLE commerce_after_sales_request DROP CONSTRAINT IF EXISTS ck_commerce_after_sales_request_approved_amount_money_format;
ALTER TABLE commerce_after_sales_request_item DROP CONSTRAINT IF EXISTS ck_commerce_after_sales_request_item_requested_amount_money_format;

-- Phase B: normalize legacy decimals, then promote each column.

-- commerce_order_item.unit_price_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_order_item
SET unit_price_amount = (CAST(unit_price_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE unit_price_amount IS NOT NULL AND unit_price_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_order_item
    ALTER COLUMN unit_price_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(unit_price_amount, '') AS NUMERIC(20,0));

-- commerce_order_item.discount_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_order_item
SET discount_amount = (CAST(discount_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE discount_amount IS NOT NULL AND discount_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_order_item
    ALTER COLUMN discount_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(discount_amount, '') AS NUMERIC(20,0));

-- commerce_order_item.tax_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_order_item
SET tax_amount = (CAST(tax_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE tax_amount IS NOT NULL AND tax_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_order_item
    ALTER COLUMN tax_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(tax_amount, '') AS NUMERIC(20,0));

-- commerce_order_item.total_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_order_item
SET total_amount = (CAST(total_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE total_amount IS NOT NULL AND total_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_order_item
    ALTER COLUMN total_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(total_amount, '') AS NUMERIC(20,0));

-- commerce_checkout_line.price_amount_snapshot: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_checkout_line
SET price_amount_snapshot = (CAST(price_amount_snapshot AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE price_amount_snapshot IS NOT NULL AND price_amount_snapshot ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_checkout_line
    ALTER COLUMN price_amount_snapshot TYPE NUMERIC(20,0)
    USING CAST(NULLIF(price_amount_snapshot, '') AS NUMERIC(20,0));

-- commerce_checkout_quote.original_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_checkout_quote
SET original_amount = (CAST(original_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE original_amount IS NOT NULL AND original_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_checkout_quote
    ALTER COLUMN original_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(original_amount, '') AS NUMERIC(20,0));

-- commerce_checkout_quote.discount_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_checkout_quote
SET discount_amount = (CAST(discount_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE discount_amount IS NOT NULL AND discount_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_checkout_quote
    ALTER COLUMN discount_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(discount_amount, '') AS NUMERIC(20,0));

-- commerce_checkout_quote.payable_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_checkout_quote
SET payable_amount = (CAST(payable_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE payable_amount IS NOT NULL AND payable_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_checkout_quote
    ALTER COLUMN payable_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(payable_amount, '') AS NUMERIC(20,0));

-- commerce_order_amount_breakdown.original_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_order_amount_breakdown
SET original_amount = (CAST(original_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE original_amount IS NOT NULL AND original_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_order_amount_breakdown ALTER COLUMN original_amount DROP DEFAULT;
ALTER TABLE commerce_order_amount_breakdown
    ALTER COLUMN original_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(original_amount, '') AS NUMERIC(20,0));
ALTER TABLE commerce_order_amount_breakdown ALTER COLUMN original_amount SET DEFAULT 0;

-- commerce_order_amount_breakdown.discount_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_order_amount_breakdown
SET discount_amount = (CAST(discount_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE discount_amount IS NOT NULL AND discount_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_order_amount_breakdown ALTER COLUMN discount_amount DROP DEFAULT;
ALTER TABLE commerce_order_amount_breakdown
    ALTER COLUMN discount_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(discount_amount, '') AS NUMERIC(20,0));
ALTER TABLE commerce_order_amount_breakdown ALTER COLUMN discount_amount SET DEFAULT 0;

-- commerce_order_amount_breakdown.payable_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_order_amount_breakdown
SET payable_amount = (CAST(payable_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE payable_amount IS NOT NULL AND payable_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_order_amount_breakdown ALTER COLUMN payable_amount DROP DEFAULT;
ALTER TABLE commerce_order_amount_breakdown
    ALTER COLUMN payable_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(payable_amount, '') AS NUMERIC(20,0));
ALTER TABLE commerce_order_amount_breakdown ALTER COLUMN payable_amount SET DEFAULT 0;

-- commerce_recharge_package.price_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_recharge_package
SET price_amount = (CAST(price_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE price_amount IS NOT NULL AND price_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_recharge_package
    ALTER COLUMN price_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(price_amount, '') AS NUMERIC(20,0));

-- commerce_account_value_package.grant_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_account_value_package
SET grant_amount = (CAST(grant_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE grant_amount IS NOT NULL AND grant_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_account_value_package
    ALTER COLUMN grant_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(grant_amount, '') AS NUMERIC(20,0));

-- commerce_account_value_package.bonus_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_account_value_package
SET bonus_amount = (CAST(bonus_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE bonus_amount IS NOT NULL AND bonus_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_account_value_package ALTER COLUMN bonus_amount DROP DEFAULT;
ALTER TABLE commerce_account_value_package
    ALTER COLUMN bonus_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(bonus_amount, '') AS NUMERIC(20,0));
ALTER TABLE commerce_account_value_package ALTER COLUMN bonus_amount SET DEFAULT 0;

-- commerce_account_value_package.price_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_account_value_package
SET price_amount = (CAST(price_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE price_amount IS NOT NULL AND price_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_account_value_package
    ALTER COLUMN price_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(price_amount, '') AS NUMERIC(20,0));

-- commerce_token_bank_plan.grant_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_token_bank_plan
SET grant_amount = (CAST(grant_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE grant_amount IS NOT NULL AND grant_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_token_bank_plan
    ALTER COLUMN grant_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(grant_amount, '') AS NUMERIC(20,0));

-- commerce_token_bank_plan.bonus_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_token_bank_plan
SET bonus_amount = (CAST(bonus_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE bonus_amount IS NOT NULL AND bonus_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_token_bank_plan ALTER COLUMN bonus_amount DROP DEFAULT;
ALTER TABLE commerce_token_bank_plan
    ALTER COLUMN bonus_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(bonus_amount, '') AS NUMERIC(20,0));
ALTER TABLE commerce_token_bank_plan ALTER COLUMN bonus_amount SET DEFAULT 0;

-- commerce_token_bank_plan.price_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_token_bank_plan
SET price_amount = (CAST(price_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE price_amount IS NOT NULL AND price_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_token_bank_plan
    ALTER COLUMN price_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(price_amount, '') AS NUMERIC(20,0));

-- commerce_order_refund_request.amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_order_refund_request
SET amount = (CAST(amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE amount IS NOT NULL AND amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_order_refund_request
    ALTER COLUMN amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(amount, '') AS NUMERIC(20,0));

-- commerce_order_refund_request.provider_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_order_refund_request
SET provider_amount = (CAST(provider_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE provider_amount IS NOT NULL AND provider_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_order_refund_request
    ALTER COLUMN provider_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(provider_amount, '') AS NUMERIC(20,0));

-- commerce_order_withdrawal_request.amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_order_withdrawal_request
SET amount = (CAST(amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE amount IS NOT NULL AND amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_order_withdrawal_request
    ALTER COLUMN amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(amount, '') AS NUMERIC(20,0));

-- commerce_order_withdrawal_request.provider_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_order_withdrawal_request
SET provider_amount = (CAST(provider_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE provider_amount IS NOT NULL AND provider_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_order_withdrawal_request
    ALTER COLUMN provider_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(provider_amount, '') AS NUMERIC(20,0));

-- commerce_after_sales_request.approved_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_after_sales_request
SET approved_amount = (CAST(approved_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE approved_amount IS NOT NULL AND approved_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_after_sales_request ALTER COLUMN approved_amount DROP DEFAULT;
ALTER TABLE commerce_after_sales_request
    ALTER COLUMN approved_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(approved_amount, '') AS NUMERIC(20,0));
ALTER TABLE commerce_after_sales_request ALTER COLUMN approved_amount SET DEFAULT 0;

-- commerce_after_sales_request_item.requested_amount: legacy '50.00' -> '5000' (canonical minor units)
UPDATE commerce_after_sales_request_item
SET requested_amount = (CAST(requested_amount AS NUMERIC(20,2)) * 100)::NUMERIC(20,0)::TEXT
WHERE requested_amount IS NOT NULL AND requested_amount ~ '^[0-9]+[.][0-9]+$';
ALTER TABLE commerce_after_sales_request_item
    ALTER COLUMN requested_amount TYPE NUMERIC(20,0)
    USING CAST(NULLIF(requested_amount, '') AS NUMERIC(20,0));

-- Phase C: numeric-domain non-negative checks.

DO $con$
BEGIN
    ALTER TABLE commerce_order_item
        ADD CONSTRAINT ck_commerce_order_item_unit_price_amount_non_negative
        CHECK (unit_price_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_order_item
        ADD CONSTRAINT ck_commerce_order_item_discount_amount_non_negative
        CHECK (discount_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_order_item
        ADD CONSTRAINT ck_commerce_order_item_tax_amount_non_negative
        CHECK (tax_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_order_item
        ADD CONSTRAINT ck_commerce_order_item_total_amount_non_negative
        CHECK (total_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_checkout_line
        ADD CONSTRAINT ck_commerce_checkout_line_price_amount_snapshot_non_negative
        CHECK (price_amount_snapshot >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_checkout_quote
        ADD CONSTRAINT ck_commerce_checkout_quote_original_amount_non_negative
        CHECK (original_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_checkout_quote
        ADD CONSTRAINT ck_commerce_checkout_quote_discount_amount_non_negative
        CHECK (discount_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_checkout_quote
        ADD CONSTRAINT ck_commerce_checkout_quote_payable_amount_non_negative
        CHECK (payable_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_order_amount_breakdown
        ADD CONSTRAINT ck_commerce_order_amount_breakdown_original_amount_non_negative
        CHECK (original_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_order_amount_breakdown
        ADD CONSTRAINT ck_commerce_order_amount_breakdown_discount_amount_non_negative
        CHECK (discount_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_order_amount_breakdown
        ADD CONSTRAINT ck_commerce_order_amount_breakdown_payable_amount_non_negative
        CHECK (payable_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_recharge_package
        ADD CONSTRAINT ck_commerce_recharge_package_price_amount_non_negative
        CHECK (price_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_account_value_package
        ADD CONSTRAINT ck_commerce_account_value_package_grant_amount_non_negative
        CHECK (grant_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_account_value_package
        ADD CONSTRAINT ck_commerce_account_value_package_bonus_amount_non_negative
        CHECK (bonus_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_account_value_package
        ADD CONSTRAINT ck_commerce_account_value_package_price_amount_non_negative
        CHECK (price_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_token_bank_plan
        ADD CONSTRAINT ck_commerce_token_bank_plan_grant_amount_non_negative
        CHECK (grant_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_token_bank_plan
        ADD CONSTRAINT ck_commerce_token_bank_plan_bonus_amount_non_negative
        CHECK (bonus_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_token_bank_plan
        ADD CONSTRAINT ck_commerce_token_bank_plan_price_amount_non_negative
        CHECK (price_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_order_refund_request
        ADD CONSTRAINT ck_commerce_order_refund_request_amount_non_negative
        CHECK (amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_order_refund_request
        ADD CONSTRAINT ck_commerce_order_refund_request_provider_amount_non_negative
        CHECK (provider_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_order_withdrawal_request
        ADD CONSTRAINT ck_commerce_order_withdrawal_request_amount_non_negative
        CHECK (amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_order_withdrawal_request
        ADD CONSTRAINT ck_commerce_order_withdrawal_request_provider_amount_non_negative
        CHECK (provider_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_after_sales_request
        ADD CONSTRAINT ck_commerce_after_sales_request_approved_amount_non_negative
        CHECK (approved_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

DO $con$
BEGIN
    ALTER TABLE commerce_after_sales_request_item
        ADD CONSTRAINT ck_commerce_after_sales_request_item_requested_amount_non_negative
        CHECK (requested_amount >= 0);
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN undefined_table THEN NULL;
END $con$;

COMMIT;
