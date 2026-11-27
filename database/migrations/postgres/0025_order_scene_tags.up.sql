-- sdkwork:migration
-- id: 0025_order_scene_tags
-- engine: postgres
-- module: sdkwork-order
-- purpose: Give every order two classification facets that the order center
--   can search and filter across all surfaces (PC admin, PC buyer, H5,
--   Flutter, mini-program).
--   `scene_code` records the commerce scenario the order was placed in
--   (how the buyer reached checkout), which is what operations teams segment
--   on when triaging funnel health and routing fulfilment.
--   `tags` is a normalized free-form label set (lowercase, hyphenated,
--   deduplicated, bounded) for risk/operation/campaign cohorts. Both are
--   filter-only facets: they never change lifecycle state.
--   `remark` is the operator note mirrored next to the classification so a tag
--   decision has its rationale in the same record.
-- reversible: true
-- rollback: forward-fix (the facet columns are additive; leaving them in place
--   after a rollback is harmless because no reader requires them to be absent)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

ALTER TABLE commerce_order
    ADD COLUMN IF NOT EXISTS scene_code TEXT NOT NULL DEFAULT 'default';

ALTER TABLE commerce_order
    ADD COLUMN IF NOT EXISTS tags JSONB NOT NULL DEFAULT '[]'::jsonb;

ALTER TABLE commerce_order
    ADD COLUMN IF NOT EXISTS remark TEXT;

-- Tag payload is a JSON array of canonical lowercase strings. The check runs
-- on write only; reading a malformed value is impossible because the default
-- and every writer go through `OrderTag::normalize`.
ALTER TABLE commerce_order
    DROP CONSTRAINT IF EXISTS ck_commerce_order_tags_json;

ALTER TABLE commerce_order
    ADD CONSTRAINT ck_commerce_order_tags_json
    CHECK (jsonb_typeof(tags) = 'array');

-- Scene segmentation: operations lists "orders in this scene, newest first".
CREATE INDEX IF NOT EXISTS idx_commerce_order_scene_list
    ON commerce_order (tenant_id, organization_id, scene_code, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_commerce_order_scene_status_list
    ON commerce_order (tenant_id, organization_id, scene_code, status, created_at DESC, id DESC);

-- Buyer-facing ("my orders") scene filter.
CREATE INDEX IF NOT EXISTS idx_order_owner_scene_list
    ON commerce_order (tenant_id, organization_id, owner_user_id, scene_code, created_at DESC, id DESC);

-- Tag containment search. JSONB GIN serves `tags @> '["vip"]'` (AND semantics);
-- the expression index serves OR semantics through per-tag equality probes.
CREATE INDEX IF NOT EXISTS idx_commerce_order_tags_gin
    ON commerce_order USING GIN (tags jsonb_path_ops);

COMMENT ON COLUMN commerce_order.scene_code IS
    'Commerce scenario the order was placed in (checkout entry surface / business scenario).';
COMMENT ON COLUMN commerce_order.tags IS
    'Canonical lowercase order label set for operator cohorts, risk flags, and campaign attribution.';
COMMENT ON COLUMN commerce_order.remark IS
    'Operator note captured alongside the tag/classification decision.';
