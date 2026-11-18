-- ============================================================================
-- Order carrier registry seed (tenant 100001)
-- ============================================================================
-- Reference data for the operator shipping surfaces: the carrier registry is
-- what the shipment create dialog and the logistics board resolve a
-- `carrier_code` against. Tracking URL templates render the last-mile link.
--
-- Idempotent: fixed ids plus an upsert on the natural key, so boot seeding and
-- `db:seed` re-runs never duplicate a carrier or overwrite an operator edit.
--
-- Carriers are reference data owned by Order. Shipments themselves stay
-- operational and are never seeded.
-- ============================================================================

INSERT INTO commerce_carrier
    (id, tenant_id, organization_id, carrier_code, carrier_name,
     tracking_url_template, status, sort_order, created_at, updated_at)
VALUES
    ('carrier-sf',  '100001', '0', 'sf',  '顺丰速运',
     'https://www.sf-express.com/chn/sc/waybill/waybill-detail/{trackingNo}', 'active', 10,
     NOW()::text, NOW()::text),
    ('carrier-jd',  '100001', '0', 'jd',  '京东物流',
     'https://www.jdl.com/orderSearch?waybillCode={trackingNo}', 'active', 20,
     NOW()::text, NOW()::text),
    ('carrier-yto', '100001', '0', 'yto', '圆通速递',
     'https://www.yto.net.cn/query/{trackingNo}', 'active', 30,
     NOW()::text, NOW()::text),
    ('carrier-dhl', '100001', '0', 'dhl', 'DHL 国际快递',
     'https://www.dhl.com/cn-zh/home/tracking/tracking-express.html?submit=1&tracking-id={trackingNo}', 'active', 40,
     NOW()::text, NOW()::text)
ON CONFLICT (tenant_id, organization_id, carrier_code) DO UPDATE SET
    carrier_name = EXCLUDED.carrier_name,
    tracking_url_template = EXCLUDED.tracking_url_template,
    status = EXCLUDED.status,
    sort_order = EXCLUDED.sort_order,
    updated_at = NOW()::text;
