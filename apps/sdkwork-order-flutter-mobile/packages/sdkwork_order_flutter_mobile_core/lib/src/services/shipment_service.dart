/// Logistics domain service: buyer-side fulfillment / shipment / tracking
/// reads assembled into one order-shipment view. Calls go through the
/// injected [OrderApiTransport] seam only. Wire contract:
/// `apis/app-api/order/order-app-api.openapi.json` — `GET /fulfillments`
/// filters by the snake_case `order_id` query, `GET /shipments/{shipmentId}`
/// returns `data.item`, `GET /shipments/{shipmentId}/tracking_events` returns
/// one bounded server page.
library;

import '../logic/envelope.dart';
import '../models/fulfillment_models.dart';
import '../transport/order_transport.dart';

class ShipmentService {
  ShipmentService(this._transport);

  final OrderApiTransport _transport;

  /// `GET /fulfillments?order_id={orderId}` — first server page (bounded,
  /// `PAGINATION_SPEC.md`); the buyer flow needs the fulfillment rows of one
  /// order only.
  Future<List<Fulfillment>> listFulfillments(
    String orderId, {
    int page = 1,
    int pageSize = 10,
  }) async {
    final data = await _transport.request(
      '/fulfillments',
      query: {
        'order_id': orderId,
        'page': '$page',
        'page_size': '$pageSize',
      },
    );
    return listItems(data).map(Fulfillment.fromWire).toList();
  }

  /// `GET /shipments/{shipmentId}`.
  Future<Shipment> retrieveShipment(String shipmentId) async {
    final data = await _transport.request('/shipments/$shipmentId');
    return Shipment.fromWire(resourceItem(data));
  }

  /// `GET /shipments/{shipmentId}/tracking_events` — one bounded page.
  Future<List<TrackingEvent>> listTrackingEvents(
    String shipmentId, {
    int page = 1,
    int pageSize = 50,
  }) async {
    final data = await _transport.request(
      '/shipments/$shipmentId/tracking_events',
      query: {'page': '$page', 'page_size': '$pageSize'},
    );
    return listItems(data).map(TrackingEvent.fromWire).toList();
  }

  /// Assembles the order's logistics view: fulfillment rows → first row's
  /// shipment → its tracking events. Returns `null` when the order has no
  /// fulfillment yet (“该订单暂无物流” — the UI hides the timeline card).
  /// Per-read failures degrade: a missing shipment/events read keeps the
  /// fulfillment header instead of failing the whole card.
  Future<OrderShipment?> getOrderShipment(String orderId) async {
    final fulfillments = await listFulfillments(orderId);
    if (fulfillments.isEmpty) {
      return null;
    }
    final fulfillment = fulfillments.first;
    Shipment? shipment;
    try {
      shipment = await retrieveShipment(fulfillment.fulfillmentId);
    } on OrderApiException {
      shipment = null;
    }
    var events = const <TrackingEvent>[];
    if (shipment != null) {
      try {
        events = await listTrackingEvents(shipment.shipmentId);
      } on OrderApiException {
        events = const <TrackingEvent>[];
      }
    }
    return OrderShipment(fulfillment: fulfillment, shipment: shipment, events: events);
  }
}
