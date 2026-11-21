-- sdkwork:migration
-- id: 0019_demand_claims
-- engine: postgres
-- module: sdkwork-order
-- purpose: Materialize the demand-claim (抢单) lifecycle tables: the claim
--   record a provider submits against a published demand, and its append-only
--   event timeline. The claim is the pre-order stage of the demand-delivery
--   lifecycle owned by this capability; the demand itself stays with the
--   company domain and is snapshotted onto the claim at submission time.
--   The partial unique index allows exactly one awarded claim per demand.
-- reversible: false
-- rollback: forward-fix (claims and their event timeline are canonical state)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

CREATE TABLE IF NOT EXISTS commerce_demand_claim (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    claim_no TEXT NOT NULL,
    demand_id TEXT NOT NULL,
    demand_title TEXT NOT NULL,
    demand_category TEXT,
    demand_type TEXT NOT NULL DEFAULT 'other',
    app_form TEXT,
    budget_text TEXT,
    budget_min TEXT,
    budget_max TEXT,
    bid_deadline TIMESTAMPTZ,
    publisher_user_id TEXT NOT NULL,
    publisher_company_name TEXT,
    bidder_user_id TEXT NOT NULL,
    bidder_company_id TEXT,
    quote_amount_minor NUMERIC(20, 0) NOT NULL,
    quote_currency TEXT NOT NULL DEFAULT 'CNY',
    delivery_days INTEGER,
    proposal TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'submitted',
    reject_reason TEXT,
    withdraw_reason TEXT,
    cancel_reason TEXT,
    delivery_note TEXT,
    delivery_attachment_urls TEXT NOT NULL DEFAULT '[]',
    order_id TEXT,
    order_no TEXT,
    request_no TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    submitted_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    delivered_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT ck_commerce_demand_claim_quote_minor_non_negative
        CHECK (quote_amount_minor >= 0),
    CONSTRAINT ck_commerce_demand_claim_delivery_days_positive
        CHECK (delivery_days IS NULL OR delivery_days > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_commerce_demand_claim_bidder_demand
    ON commerce_demand_claim(tenant_id, demand_id, bidder_user_id);

CREATE INDEX IF NOT EXISTS idx_commerce_demand_claim_bidder_status
    ON commerce_demand_claim(tenant_id, bidder_user_id, status, updated_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_commerce_demand_claim_publisher_status
    ON commerce_demand_claim(tenant_id, publisher_user_id, status, updated_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_commerce_demand_claim_demand
    ON commerce_demand_claim(tenant_id, demand_id, created_at DESC, id DESC);

-- One demand can only have one claim in an awarded-or-later state; the state
-- machine guard on each transition keeps that invariant from the write side.
CREATE UNIQUE INDEX IF NOT EXISTS uq_commerce_demand_claim_awarded_demand
    ON commerce_demand_claim(tenant_id, demand_id)
    WHERE status IN ('accepted', 'in_delivery', 'delivered', 'completed');

CREATE TABLE IF NOT EXISTS commerce_demand_claim_event (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    claim_id TEXT NOT NULL,
    event_no TEXT NOT NULL,
    event_type TEXT NOT NULL,
    from_status TEXT,
    to_status TEXT NOT NULL,
    actor_type TEXT NOT NULL,
    actor_id TEXT,
    message TEXT,
    payload_json TEXT NOT NULL DEFAULT '{}',
    request_id TEXT,
    idempotency_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_commerce_demand_claim_event_claim
    ON commerce_demand_claim_event(tenant_id, claim_id, created_at ASC, id ASC);

CREATE INDEX IF NOT EXISTS idx_commerce_demand_claim_event_tenant_type
    ON commerce_demand_claim_event(tenant_id, event_type, created_at DESC, id DESC);

COMMIT;
