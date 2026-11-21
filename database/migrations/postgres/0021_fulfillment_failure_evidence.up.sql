-- sdkwork:migration
-- id: 0021_fulfillment_failure_evidence
-- engine: postgres
-- module: sdkwork-order
-- purpose: Record why a coupon-redemption fulfilment failed and when, so an
--   order that was attempted and failed is distinguishable from one that was
--   never attempted. `commerce_order.fulfillment_status` stays free TEXT with
--   no CHECK constraint; the two nullable evidence columns carry the failure
--   reason (max 512 characters) and instant, and the partial index lets support
--   and finance list failed fulfilments by tenant/organization/subject.
--   `failed` remains reservable, so a subsequent retry under the same
--   idempotency key can still claim the order.
-- reversible: false
-- rollback: forward-fix (the evidence columns are additive and nullable)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

ALTER TABLE commerce_order ADD COLUMN IF NOT EXISTS fulfillment_failure_reason TEXT;
ALTER TABLE commerce_order ADD COLUMN IF NOT EXISTS fulfillment_failed_at TIMESTAMPTZ;
CREATE INDEX IF NOT EXISTS idx_commerce_order_failed_fulfillment
  ON commerce_order (tenant_id, organization_id, subject, fulfillment_failed_at DESC)
  WHERE fulfillment_status = 'failed';

COMMIT;
