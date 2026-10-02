/**
 * Logistics domain service (fulfillments / shipments / tracking events).
 *
 * Wire contract: apis/app-api/order/order-app-api.openapi.json
 * (fulfillments.list / shipments.retrieve / shipments.trackingEvents.list).
 * Request/response field names mirror the backend camelCase read models;
 * ids and tracking numbers stay strings end to end (int64-as-string
 * contract). All traffic funnels through the single transport seam.
 */
const { request, unwrapResource, unwrapList } = require("./transport");

/**
 * Lists the fulfillment rows of one buyer order.
 * GET /fulfillments?order_id={orderId}&page=1&page_size=10 — the snake_case
 * `order_id` query name is the OpenAPI wire contract.
 */
async function listFulfillments(orderId, options = {}) {
  const payload = await request({
    path: "/fulfillments",
    query: {
      order_id: orderId,
      page: options.page ?? 1,
      page_size: options.pageSize ?? 10,
    },
  });
  const { items, pageInfo } = unwrapList(payload);
  return {
    items: items.map((value) => {
      const record = value && typeof value === "object" ? value : {};
      return {
        fulfillmentId: String(record.fulfillmentId ?? record.id ?? ""),
        fulfillmentNo: String(record.fulfillmentNo ?? ""),
        fulfillmentType: String(record.fulfillmentType ?? ""),
        orderId: String(record.orderId ?? ""),
        status: String(record.status ?? ""),
      };
    }),
    pageInfo,
  };
}

/** GET /shipments/{shipmentId} → data.item. */
async function retrieveShipment(shipmentId) {
  const item = unwrapResource(await request({ path: `/shipments/${shipmentId}` }));
  const record = item && typeof item === "object" ? item : {};
  return {
    shipmentId: String(record.shipmentId ?? record.id ?? ""),
    shipmentNo: String(record.shipmentNo ?? ""),
    fulfillmentId: String(record.fulfillmentId ?? ""),
    carrierCode: String(record.carrierCode ?? ""),
    status: String(record.status ?? ""),
    trackingNo:
      record.trackingNo !== undefined && record.trackingNo !== null && record.trackingNo !== ""
        ? String(record.trackingNo)
        : undefined,
  };
}

/**
 * GET /shipments/{shipmentId}/tracking_events?page=1&page_size=50 — one
 * bounded server page; the events are returned in carrier order and the
 * page renders them as-is (no client-side reordering).
 */
async function listTrackingEvents(shipmentId, options = {}) {
  const payload = await request({
    path: `/shipments/${shipmentId}/tracking_events`,
    query: { page: options.page ?? 1, page_size: options.pageSize ?? 50 },
  });
  const { items, pageInfo } = unwrapList(payload);
  return {
    items: items.map((value) => {
      const record = value && typeof value === "object" ? value : {};
      return {
        eventId: String(record.eventId ?? record.id ?? ""),
        trackingEventNo: String(record.trackingEventNo ?? ""),
        shipmentId: String(record.shipmentId ?? ""),
        eventType: String(record.eventType ?? ""),
        eventStatus:
          record.eventStatus !== undefined && record.eventStatus !== null && record.eventStatus !== ""
            ? String(record.eventStatus)
            : undefined,
        eventTime: String(record.eventTime ?? ""),
        locationText:
          record.locationText !== undefined && record.locationText !== null && record.locationText !== ""
            ? String(record.locationText)
            : undefined,
      };
    }),
    pageInfo,
  };
}

/**
 * Assembles the order's logistics view: fulfillments → first row's shipment
 * → its tracking events. Returns null when the order has no fulfillment yet
 * (该订单暂无物流 — the page hides the 物流追踪 card). Shipment/events read
 * failures degrade to a fulfillment-header-only view instead of failing the
 * whole assembly.
 */
async function getOrderShipment(orderId) {
  const { items } = await listFulfillments(orderId);
  if (items.length === 0) {
    return null;
  }
  const fulfillment = items[0];
  let shipment = null;
  try {
    shipment = await retrieveShipment(fulfillment.fulfillmentId);
  } catch (cause) {
    shipment = null;
  }
  let events = [];
  if (shipment) {
    try {
      events = (await listTrackingEvents(shipment.shipmentId)).items;
    } catch (cause) {
      events = [];
    }
  }
  return { fulfillment, shipment, events };
}

module.exports = {
  listFulfillments,
  retrieveShipment,
  listTrackingEvents,
  getOrderShipment,
};
