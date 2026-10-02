import 'package:flutter_test/flutter_test.dart';

import 'package:sdkwork_order_flutter_mobile_core/src/services/shipment_service.dart';
import 'package:sdkwork_order_flutter_mobile_core/src/transport/order_transport.dart';

/// Scripted fake transport: records calls, replays canned envelopes
/// (same shape as `order_service_test.dart`).
class FakeOrderApiTransport implements OrderApiTransport {
  FakeOrderApiTransport(this._routes);

  final Map<String, Map<String, dynamic>> _routes;
  final List<Call> calls = <Call>[];

  @override
  Future<Map<String, dynamic>> request(
    String path, {
    String method = 'GET',
    Map<String, dynamic>? body,
    Map<String, String>? query,
    String? idempotencyKey,
  }) async {
    calls.add(Call(method, path, query));
    final data = _routes['$method $path'];
    if (data == null) {
      throw OrderApiException('no route for $method $path');
    }
    return data;
  }
}

class Call {
  const Call(this.method, this.path, this.query);

  final String method;
  final String path;
  final Map<String, String>? query;
}

void main() {
  test('getOrderShipment assembles fulfillment → shipment → tracking events',
      () async {
    final transport = FakeOrderApiTransport({
      'GET /fulfillments': {
        'items': [
          {
            'fulfillmentId': '9007199254740993',
            'fulfillmentNo': 'FL-1',
            'fulfillmentType': 'standard',
            'orderId': 'o-1',
            'status': 'shipped',
          }
        ],
        'pageInfo': {
          'mode': 'offset',
          'page': 1,
          'pageSize': 10,
          'totalItems': '1',
          'hasMore': false,
        },
      },
      'GET /shipments/9007199254740993': {
        'item': {
          'shipmentId': '9007199254740993',
          'shipmentNo': 'SH-1',
          'fulfillmentId': '9007199254740993',
          'carrierCode': 'sf_express',
          'status': 'in_transit',
          'trackingNo': 'SF123456789',
        },
      },
      'GET /shipments/9007199254740993/tracking_events': {
        'items': [
          {
            'eventId': 'e-2',
            'trackingEventNo': 'TE-2',
            'shipmentId': '9007199254740993',
            'eventType': 'in_transit',
            'eventStatus': 'transporting',
            'eventTime': '2026-10-03T09:30:00Z',
            'locationText': '杭州转运中心',
          },
          {
            'eventId': 'e-1',
            'trackingEventNo': 'TE-1',
            'shipmentId': '9007199254740993',
            'eventType': 'picked_up',
            'eventTime': '2026-10-03T08:00:00Z',
          },
        ],
        'pageInfo': <String, dynamic>{'hasMore': false},
      },
    });
    final service = ShipmentService(transport);

    final assembled = await service.getOrderShipment('o-1');

    expect(transport.calls, hasLength(3));
    expect(transport.calls[0].path, '/fulfillments');
    // Snake_case query wire name per the OpenAPI parameter.
    expect(transport.calls[0].query, {
      'order_id': 'o-1',
      'page': '1',
      'page_size': '10',
    });
    expect(transport.calls[1].path, '/shipments/9007199254740993');
    expect(transport.calls[2].path, '/shipments/9007199254740993/tracking_events');
    expect(transport.calls[2].query, {'page': '1', 'page_size': '50'});

    final fulfillment = assembled!.fulfillment;
    expect(fulfillment.fulfillmentId, '9007199254740993');
    expect(fulfillment.orderId, 'o-1');
    expect(fulfillment.status, 'shipped');
    final shipment = assembled.shipment!;
    expect(shipment.carrierCode, 'sf_express');
    expect(shipment.trackingNo, 'SF123456789');
    expect(assembled.events, hasLength(2));
    expect(assembled.events.first.locationText, '杭州转运中心');
    expect(assembled.events.last.eventStatus, isNull);
  });

  test('getOrderShipment returns null when the order has no fulfillment',
      () async {
    final transport = FakeOrderApiTransport({
      'GET /fulfillments': {
        'items': <Object?>[],
        'pageInfo': <String, dynamic>{'hasMore': false},
      },
    });
    final shipment = await ShipmentService(transport).getOrderShipment('o-1');
    expect(shipment, isNull);
    expect(transport.calls, hasLength(1));
  });

  test('getOrderShipment keeps the fulfillment header when the shipment read fails',
      () async {
    final transport = FakeOrderApiTransport({
      'GET /fulfillments': {
        'items': [
          {
            'fulfillmentId': 'f-1',
            'fulfillmentNo': 'FL-1',
            'fulfillmentType': 'standard',
            'orderId': 'o-1',
            'status': 'shipped',
          }
        ],
        'pageInfo': <String, dynamic>{'hasMore': false},
      },
      // No /shipments/f-1 route → OrderApiException.
    });
    final assembled = await ShipmentService(transport).getOrderShipment('o-1');
    expect(assembled, isNotNull);
    expect(assembled!.shipment, isNull);
    expect(assembled.events, isEmpty);
    expect(assembled.fulfillment.fulfillmentId, 'f-1');
  });
}
