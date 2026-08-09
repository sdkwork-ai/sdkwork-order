-- Platform points recharge catalog. Amounts are stored as major-unit decimals.
-- The SKU ids are stable references to the shared commerce merchandise catalog.
-- Retire the legacy demo catalog that was previously bootstrapped outside Order ownership.
UPDATE commerce_recharge_package
SET status = 'inactive', updated_at = CURRENT_TIMESTAMP
WHERE tenant_id = '100001'
  AND (organization_id = '0' OR organization_id IS NULL)
  AND id LIKE 'bootstrap-admin-recharge-package-%';

INSERT INTO commerce_recharge_package (
    id, tenant_id, organization_id, external_id, package_no, sku_id, name,
    price_amount, currency_code, bonus_points, status, valid_from, valid_to,
    sort_weight, request_no, idempotency_key, created_at, updated_at
) VALUES
    ('recharge-500', '100001', '0', 500, 'points-500', 'recharge-sku-500', '500 compute points', '50.00', 'CNY', 0, 'active', NULL, NULL, 10, 'seed-recharge-500', 'seed-recharge-500', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('recharge-750', '100001', '0', 750, 'points-750', 'recharge-sku-750', '750 compute points', '75.00', 'CNY', 0, 'active', NULL, NULL, 20, 'seed-recharge-750', 'seed-recharge-750', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('recharge-1500', '100001', '0', 1500, 'points-1500', 'recharge-sku-1500', '1500 compute points', '150.00', 'CNY', 0, 'active', NULL, NULL, 30, 'seed-recharge-1500', 'seed-recharge-1500', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('recharge-2250', '100001', '0', 2250, 'points-2250', 'recharge-sku-2250', '2250 compute points', '223.00', 'CNY', 20, 'active', NULL, NULL, 40, 'seed-recharge-2250', 'seed-recharge-2250', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('recharge-4500', '100001', '0', 4500, 'points-4500', 'recharge-sku-4500', '4500 compute points', '450.00', 'CNY', 0, 'active', NULL, NULL, 50, 'seed-recharge-4500', 'seed-recharge-4500', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('recharge-9000', '100001', '0', 9000, 'points-9000', 'recharge-sku-9000', '9000 compute points', '899.00', 'CNY', 10, 'active', NULL, NULL, 60, 'seed-recharge-9000', 'seed-recharge-9000', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
ON CONFLICT (id) DO UPDATE SET
    tenant_id = EXCLUDED.tenant_id,
    organization_id = EXCLUDED.organization_id,
    external_id = EXCLUDED.external_id,
    package_no = EXCLUDED.package_no,
    sku_id = EXCLUDED.sku_id,
    name = EXCLUDED.name,
    price_amount = EXCLUDED.price_amount,
    currency_code = EXCLUDED.currency_code,
    bonus_points = EXCLUDED.bonus_points,
    status = EXCLUDED.status,
    valid_from = EXCLUDED.valid_from,
    valid_to = EXCLUDED.valid_to,
    sort_weight = EXCLUDED.sort_weight,
    request_no = EXCLUDED.request_no,
    idempotency_key = EXCLUDED.idempotency_key,
    updated_at = EXCLUDED.updated_at;

-- Token Bank recharge plans. Amounts are stored as major-unit decimals.
-- These are the standard compute-credit tiers offered by the H5 Token Bank
-- cashier (`GET /app/v3/api/recharges/plans`); the plan codes are stable
-- references used by the recharge order flow.
INSERT INTO commerce_token_bank_plan (
    id, tenant_id, organization_id, plan_code, display_name, plan_period,
    grant_amount, bonus_amount, price_amount, currency_code, renewal_policy,
    status, sort_weight, request_no, idempotency_key, created_at, updated_at
) VALUES
    ('token-bank-100', '100001', '0', 'tb-100', '100 T 算力包', 'monthly', '100', '0', '10.00', 'CNY', 'non_renewable', 'active', 10, 'seed-token-bank-100', 'seed-token-bank-100', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('token-bank-500', '100001', '0', 'tb-500', '500 T 算力包', 'monthly', '500', '50', '48.00', 'CNY', 'non_renewable', 'active', 20, 'seed-token-bank-500', 'seed-token-bank-500', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('token-bank-1000', '100001', '0', 'tb-1000', '1000 T 算力包', 'monthly', '1000', '120', '95.00', 'CNY', 'non_renewable', 'active', 30, 'seed-token-bank-1000', 'seed-token-bank-1000', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('token-bank-5000', '100001', '0', 'tb-5000', '5000 T 算力包', 'monthly', '5000', '800', '450.00', 'CNY', 'non_renewable', 'active', 40, 'seed-token-bank-5000', 'seed-token-bank-5000', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
ON CONFLICT (id) DO UPDATE SET
    tenant_id = EXCLUDED.tenant_id,
    organization_id = EXCLUDED.organization_id,
    plan_code = EXCLUDED.plan_code,
    display_name = EXCLUDED.display_name,
    plan_period = EXCLUDED.plan_period,
    grant_amount = EXCLUDED.grant_amount,
    bonus_amount = EXCLUDED.bonus_amount,
    price_amount = EXCLUDED.price_amount,
    currency_code = EXCLUDED.currency_code,
    renewal_policy = EXCLUDED.renewal_policy,
    status = EXCLUDED.status,
    sort_weight = EXCLUDED.sort_weight,
    request_no = EXCLUDED.request_no,
    idempotency_key = EXCLUDED.idempotency_key,
    updated_at = EXCLUDED.updated_at;
