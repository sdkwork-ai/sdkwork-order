-- ============================================================================
-- Order demo fulfillment/shipment seed (tenant 100001)
-- ============================================================================
-- The Product Center shipment queue, logistics board, package dialog and
-- tracking timeline are empty on a fresh deployment because shipments only
-- appear after an order is fulfilled. This seed materializes a small, clearly
-- labelled demo fulfillment chain so the surfaces are demo-ready in every
-- environment:
--
--   demo-order-1 -> fulfillment 1 -> shipment 1 (sf, delivered, 3 track events)
--   demo-order-2 -> fulfillment 2 -> shipment 2 (jd, shipped, 2 track events)
--   demo-order-3 -> fulfillment 3 -> shipment 3 (yto, created)
--
-- Carriers come from 002_carriers.sql. Ids are `demo-*` and every statement
-- upserts on the primary key, so boot seeding is idempotent. Real deployments
-- keep their own fulfillment flow; these rows are demo reference data and can be
-- removed by an operator.
-- ============================================================================

INSERT INTO commerce_fulfillment_order
    (id, tenant_id, organization_id, fulfillment_no, order_id, fulfillment_type, status,
     warehouse_id, address_snapshot_id, provider_code, created_at, completed_at, updated_at)
VALUES
    ('demo-fulfillment-1', '100001', '0', 'FUL-DEMO-1', 'demo-order-1', 'physical_shipment',
     'shipped', 'wh-main', NULL, 'merchant', (NOW() - INTERVAL '4 days'), NULL,
     (NOW() - INTERVAL '1 day')),
    ('demo-fulfillment-2', '100001', '0', 'FUL-DEMO-2', 'demo-order-2', 'physical_shipment',
     'awaiting_shipment', 'wh-main', NULL, 'merchant', (NOW() - INTERVAL '2 days'), NULL,
     (NOW() - INTERVAL '1 day')),
    ('demo-fulfillment-3', '100001', '0', 'FUL-DEMO-3', 'demo-order-3', 'physical_shipment',
     'completed', 'wh-main', NULL, 'merchant', (NOW() - INTERVAL '10 days'),
     (NOW() - INTERVAL '6 days'), (NOW() - INTERVAL '6 days'))
ON CONFLICT (id) DO UPDATE SET
    fulfillment_no = EXCLUDED.fulfillment_no,
    order_id = EXCLUDED.order_id,
    fulfillment_type = EXCLUDED.fulfillment_type,
    status = EXCLUDED.status,
    warehouse_id = EXCLUDED.warehouse_id,
    provider_code = EXCLUDED.provider_code,
    updated_at = EXCLUDED.updated_at;

INSERT INTO commerce_shipment
    (id, tenant_id, organization_id, shipment_no, fulfillment_id, carrier_code, tracking_no,
     status, shipped_at, delivered_at, created_at, updated_at)
VALUES
    ('demo-shipment-1', '100001', '0', 'SHP-DEMO-1', 'demo-fulfillment-1', 'sf', 'SF-DEMO-1',
     'delivered', (NOW() - INTERVAL '3 days'), (NOW() - INTERVAL '1 day'),
     (NOW() - INTERVAL '4 days'), (NOW() - INTERVAL '1 day')),
    ('demo-shipment-2', '100001', '0', 'SHP-DEMO-2', 'demo-fulfillment-2', 'jd', 'JD-DEMO-2',
     'shipped', (NOW() - INTERVAL '1 day'), NULL,
     (NOW() - INTERVAL '2 days'), (NOW() - INTERVAL '1 day')),
    ('demo-shipment-3', '100001', '0', 'SHP-DEMO-3', 'demo-fulfillment-3', 'yto', 'YTO-DEMO-3',
     'created', NULL, NULL, (NOW() - INTERVAL '6 days'), (NOW() - INTERVAL '6 days'))
ON CONFLICT (id) DO UPDATE SET
    shipment_no = EXCLUDED.shipment_no,
    fulfillment_id = EXCLUDED.fulfillment_id,
    carrier_code = EXCLUDED.carrier_code,
    tracking_no = EXCLUDED.tracking_no,
    status = EXCLUDED.status,
    shipped_at = EXCLUDED.shipped_at,
    delivered_at = EXCLUDED.delivered_at,
    updated_at = EXCLUDED.updated_at;

INSERT INTO commerce_shipment_package
    (id, tenant_id, organization_id, shipment_id, package_no, package_type, tracking_no, status, created_at)
VALUES
    ('demo-package-1', '100001', '0', 'demo-shipment-1', 'PKG-DEMO-1', 'box', 'SF-DEMO-1', 'delivered',
     (NOW() - INTERVAL '4 days')),
    ('demo-package-2', '100001', '0', 'demo-shipment-1', 'PKG-DEMO-2', 'envelope', 'SF-DEMO-1', 'delivered',
     (NOW() - INTERVAL '4 days')),
    ('demo-package-3', '100001', '0', 'demo-shipment-2', 'PKG-DEMO-3', 'box', 'JD-DEMO-2', 'shipped',
     (NOW() - INTERVAL '2 days')),
    ('demo-package-4', '100001', '0', 'demo-shipment-3', 'PKG-DEMO-4', 'box', 'YTO-DEMO-3', 'created',
     (NOW() - INTERVAL '6 days'))
ON CONFLICT (id) DO UPDATE SET
    shipment_id = EXCLUDED.shipment_id,
    package_no = EXCLUDED.package_no,
    package_type = EXCLUDED.package_type,
    tracking_no = EXCLUDED.tracking_no,
    status = EXCLUDED.status;

INSERT INTO commerce_shipment_tracking_event
    (id, tenant_id, organization_id, shipment_id, tracking_event_no, event_type, event_status,
     event_time, location_text, created_at)
VALUES
    ('demo-tracking-1', '100001', '0', 'demo-shipment-1', 'TRK-DEMO-1', 'picked_up', 'shipped',
     (NOW() - INTERVAL '3 days')::text, '上海转运中心', (NOW() - INTERVAL '3 days')),
    ('demo-tracking-2', '100001', '0', 'demo-shipment-1', 'TRK-DEMO-2', 'in_transit', 'shipped',
     (NOW() - INTERVAL '2 days')::text, '杭州分拨中心', (NOW() - INTERVAL '2 days')),
    ('demo-tracking-3', '100001', '0', 'demo-shipment-1', 'TRK-DEMO-3', 'delivered', 'delivered',
     (NOW() - INTERVAL '1 day')::text, '北京朝阳区（已签收，本人签收）', (NOW() - INTERVAL '1 day')),
    ('demo-tracking-4', '100001', '0', 'demo-shipment-2', 'TRK-DEMO-4', 'picked_up', 'shipped',
     (NOW() - INTERVAL '1 day')::text, '深圳前置仓', (NOW() - INTERVAL '1 day')),
    ('demo-tracking-5', '100001', '0', 'demo-shipment-2', 'TRK-DEMO-5', 'in_transit', 'shipped',
     (NOW() - INTERVAL '6 hours')::text, '广州转运中心', (NOW() - INTERVAL '6 hours'))
ON CONFLICT (id) DO UPDATE SET
    shipment_id = EXCLUDED.shipment_id,
    tracking_event_no = EXCLUDED.tracking_event_no,
    event_type = EXCLUDED.event_type,
    event_status = EXCLUDED.event_status,
    event_time = EXCLUDED.event_time,
    location_text = EXCLUDED.location_text;
