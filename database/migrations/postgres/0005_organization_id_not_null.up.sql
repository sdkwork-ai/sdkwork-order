-- sdkwork:migration
-- id: 0005_organization_id_not_null
-- engine: postgres
-- module: sdkwork-order
-- purpose: Enforce organization_id NOT NULL DEFAULT on all tables in the
--   consolidated baseline. NULL rows (pre-standard data anomalies) are
--   backfilled with the platform sentinel before NOT NULL is set, and
--   NOT NULL columns without an explicit default receive the sentinel
--   default, keeping existing deployments consistent with fresh baseline
--   installs.
-- reversible: false
-- rollback: forward-fix (sentinel backfill is the canonical fix; NULL
--   organization rows are data anomalies)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

ALTER TABLE commerce_order ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_order SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_order ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_order ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE commerce_order_item ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_order_item SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_order_item ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_order_item ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE commerce_checkout_session ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_checkout_session SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_checkout_session ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_checkout_session ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE commerce_checkout_line ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_checkout_line SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_checkout_line ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_checkout_line ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE commerce_checkout_quote ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_checkout_quote SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_checkout_quote ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_checkout_quote ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE commerce_fulfillment_order ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_fulfillment_order SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_fulfillment_order ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_fulfillment_order ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE commerce_order_amount_breakdown ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_order_amount_breakdown SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_order_amount_breakdown ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_order_amount_breakdown ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE commerce_recharge_package ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_recharge_package SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_recharge_package ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_recharge_package ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE commerce_account_value_package ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_account_value_package SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_account_value_package ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_account_value_package ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE commerce_token_bank_plan ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_token_bank_plan SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_token_bank_plan ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_token_bank_plan ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE commerce_order_refund_request ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_order_refund_request SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_order_refund_request ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_order_refund_request ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE commerce_order_withdrawal_request ADD COLUMN IF NOT EXISTS organization_id TEXT NOT NULL DEFAULT '0';
UPDATE commerce_order_withdrawal_request SET organization_id = '0' WHERE organization_id IS NULL;
ALTER TABLE commerce_order_withdrawal_request ALTER COLUMN organization_id SET DEFAULT '0';
ALTER TABLE commerce_order_withdrawal_request ALTER COLUMN organization_id SET NOT NULL;

COMMIT;
