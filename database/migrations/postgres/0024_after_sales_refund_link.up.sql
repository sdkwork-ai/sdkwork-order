-- sdkwork:migration
-- id: 0024_after_sales_refund_link
-- engine: postgres
-- module: sdkwork-order
-- purpose: Link an approved after-sales refund to the order refund request it
--   triggers. The after-sales review now creates (or reuses) the refund request
--   and runs the existing hold -> provider refund -> hold settlement chain;
--   `refund_request_id` is the durable join between the two surfaces and lets
--   the refund webhook settlement mirror the terminal state back onto the case.
-- reversible: false
-- rollback: forward-fix (dropping the link column would orphan refund traces)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

ALTER TABLE commerce_after_sales_request
    ADD COLUMN IF NOT EXISTS refund_request_id TEXT;

CREATE INDEX IF NOT EXISTS idx_after_sales_request_refund_request
    ON commerce_after_sales_request(tenant_id, refund_request_id);
