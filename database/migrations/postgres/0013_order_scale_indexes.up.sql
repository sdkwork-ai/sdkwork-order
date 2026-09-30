-- sdkwork:migration
-- id: 0013_order_scale_indexes
-- engine: postgres
-- module: sdkwork-order
-- purpose: Scale-path indexes for the three unindexed hot access paths:
--   1. the expiration scheduler sweep (status-filtered `expired_at` due scan
--      with no tenant predicate — the existing owner/partner list indexes
--      cannot serve it, so every tick degenerated to a full table scan);
--   2. the backend management order list (tenant+organization scoped
--      `created_at DESC` page scan, previously served by owner-keyed indexes
--      only when the operator happened to filter by owner);
--   3. the cumulative in-flight refund bound (`original_order_id` sum inside
--      refund creation, previously a scan per creation attempt).
-- reversible: false
-- rollback: forward-fix (additive indexes; drop by name if ever needed)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

-- Expiration sweep: expression index over the canonical RFC 3339 instant with
-- the partial predicate matching the scheduler's expirable-status filter, so
-- the sweep reads only live payment windows (the index self-shrinks as orders
-- expire). `id` is appended to match the deterministic tiebreak ordering.
CREATE INDEX IF NOT EXISTS idx_order_expiration_due
    ON commerce_order (
        (NULLIF(expired_at, '')::timestamptz),
        id
    )
    WHERE LOWER(COALESCE(status, ''))
          IN ('draft', 'pending', 'pending_payment', 'unpaid', 'wait_pay');

-- Backend management order list: tenant+organization page scan ordered by
-- creation recency (matches LIST_MANAGEMENT_ORDERS ORDER BY exactly).
CREATE INDEX IF NOT EXISTS idx_order_management_list
    ON commerce_order (tenant_id, organization_id, created_at DESC, id DESC);

-- Cumulative in-flight refund bound: the over-refund predicate sums refund
-- requests per original order inside every refund creation.
CREATE INDEX IF NOT EXISTS idx_order_refund_request_original
    ON commerce_order_refund_request (tenant_id, original_order_id, status);

COMMIT;
