-- sdkwork:migration
-- id: 0026_order_event_organization_scope
-- engine: postgres
-- module: sdkwork-order
-- purpose: Align `commerce_order_event.organization_id` with DATABASE_SPEC
--   DB089, which requires the organization scope to be `NOT NULL DEFAULT '0'`
--   (the platform sentinel) rather than nullable. A NULL scope forces every
--   reader into the `(organization_id = $2 OR organization_id IS NULL)` fuzz,
--   which can both hide events from an organization-scoped reader and let one
--   organization's scope decorate another's timeline. The order centre renders
--   this table as the order timeline, so the invariant matters there.
-- reversible: false
-- rollback: forward-fix (nulling the scope again would reintroduce the fuzzy
--   matching this migration removes)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

-- Backfill first so the NOT NULL constraint can be added without a rewrite
-- rewrite on a populated table.
UPDATE commerce_order_event
SET organization_id = '0'
WHERE organization_id IS NULL;

ALTER TABLE commerce_order_event
    ALTER COLUMN organization_id SET DEFAULT '0';

ALTER TABLE commerce_order_event
    ALTER COLUMN organization_id SET NOT NULL;
