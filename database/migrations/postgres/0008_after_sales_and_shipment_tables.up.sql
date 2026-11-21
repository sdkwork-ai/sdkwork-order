-- sdkwork:migration
-- id: 0008_after_sales_and_shipment_tables
-- engine: postgres
-- module: sdkwork-order
-- purpose: Materialize the after-sales lifecycle tables
--   (`commerce_after_sales_request` / `_item` / `_event` / `_return_shipment`)
--   and the shipment lifecycle tables (`commerce_shipment` /
--   `commerce_shipment_package` / `commerce_shipment_tracking_event`) for
--   databases initialized before the baseline snapshot contained them.
--   Greenfield databases get these through the baseline; this migration
--   upgrades existing deployments idempotently.
-- reversible: false
-- rollback: forward-fix
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

CREATE TABLE IF NOT EXISTS commerce_after_sales_request (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    after_sales_no TEXT NOT NULL,
    order_id TEXT NOT NULL,
    owner_user_id TEXT NOT NULL,
    after_sales_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'submitted',
    refund_status TEXT NOT NULL DEFAULT 'none',
    return_status TEXT NOT NULL DEFAULT 'none',
    exchange_status TEXT NOT NULL DEFAULT 'none',
    reason_code TEXT NOT NULL,
    description TEXT,
    requested_amount TEXT NOT NULL,
    approved_amount TEXT NOT NULL DEFAULT '0.00',
    currency_code TEXT NOT NULL,
    requested_by_type TEXT NOT NULL DEFAULT 'buyer',
    requested_by TEXT,
    request_no TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_after_sales_request_tenant_owner
    ON commerce_after_sales_request(tenant_id, owner_user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_after_sales_request_idempotency
    ON commerce_after_sales_request(tenant_id, order_id, idempotency_key);

CREATE TABLE IF NOT EXISTS commerce_after_sales_request_item (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    after_sales_id TEXT NOT NULL,
    order_item_id TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    requested_amount TEXT NOT NULL,
    currency_code TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (after_sales_id) REFERENCES commerce_after_sales_request(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_after_sales_request_item_request
    ON commerce_after_sales_request_item(after_sales_id);

CREATE TABLE IF NOT EXISTS commerce_after_sales_event (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    after_sales_id TEXT NOT NULL,
    event_no TEXT NOT NULL,
    event_type TEXT NOT NULL,
    from_status TEXT,
    to_status TEXT NOT NULL,
    actor_type TEXT NOT NULL DEFAULT 'buyer',
    actor_id TEXT,
    request_id TEXT,
    idempotency_key TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (after_sales_id) REFERENCES commerce_after_sales_request(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_after_sales_event_request
    ON commerce_after_sales_event(tenant_id, after_sales_id, created_at ASC);

CREATE TABLE IF NOT EXISTS commerce_after_sales_return_shipment (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    after_sales_id TEXT NOT NULL,
    return_shipment_no TEXT NOT NULL,
    carrier_code TEXT,
    tracking_no TEXT,
    status TEXT NOT NULL DEFAULT 'submitted',
    request_no TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (after_sales_id) REFERENCES commerce_after_sales_request(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_after_sales_return_shipment_idempotency
    ON commerce_after_sales_return_shipment(tenant_id, after_sales_id, idempotency_key);

CREATE TABLE IF NOT EXISTS commerce_shipment (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    shipment_no TEXT NOT NULL,
    fulfillment_id TEXT NOT NULL,
    carrier_code TEXT NOT NULL,
    tracking_no TEXT,
    status TEXT NOT NULL,
    shipped_at TEXT,
    delivered_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_commerce_shipment_tenant_created
    ON commerce_shipment(tenant_id, created_at DESC);

CREATE TABLE IF NOT EXISTS commerce_shipment_package (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    shipment_id TEXT NOT NULL,
    package_no TEXT NOT NULL,
    package_type TEXT NOT NULL,
    tracking_no TEXT,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_commerce_shipment_package_shipment
    ON commerce_shipment_package(tenant_id, shipment_id, created_at ASC);

CREATE TABLE IF NOT EXISTS commerce_shipment_tracking_event (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    shipment_id TEXT NOT NULL,
    tracking_event_no TEXT NOT NULL,
    event_type TEXT NOT NULL,
    event_status TEXT,
    event_time TEXT NOT NULL,
    location_text TEXT,
    created_at TEXT NOT NULL
);

COMMIT;
