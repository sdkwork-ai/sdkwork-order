/// Typed read models for the logistics domain (fulfillments / shipments /
/// tracking events), mapped defensively from loose wire payloads. Wire
/// contract: `apis/app-api/order/order-app-api.openapi.json`
/// (`fulfillments.list` / `shipments.retrieve` /
/// `shipments.trackingEvents.list`; generic `SdkWorkListResponse` /
/// `SdkWorkResourceResponse` envelopes — field names follow the Rust route
/// serializers, `serde(rename_all = "camelCase")`). Ids stay strings
/// (int64 wire contract, `API_SPEC.md` §13.6).
library;

import '../logic/envelope.dart';

/// Fulfillment line (`data.items[]` of `GET /fulfillments`).
class Fulfillment {
  const Fulfillment({
    required this.fulfillmentId,
    required this.fulfillmentNo,
    required this.fulfillmentType,
    required this.orderId,
    required this.status,
  });

  final String fulfillmentId;
  final String fulfillmentNo;
  final String fulfillmentType;
  final String orderId;
  final String status;

  static Fulfillment fromWire(Map<String, dynamic> wire) => Fulfillment(
        fulfillmentId: asString(wire, const ['fulfillmentId', 'id']),
        fulfillmentNo: asString(wire, const ['fulfillmentNo']),
        fulfillmentType: asString(wire, const ['fulfillmentType']),
        orderId: asString(wire, const ['orderId']),
        status: asString(wire, const ['status']),
      );
}

/// Shipment header (`data.item` of `GET /shipments/{shipmentId}`).
class Shipment {
  const Shipment({
    required this.shipmentId,
    required this.shipmentNo,
    required this.fulfillmentId,
    required this.carrierCode,
    required this.status,
    this.trackingNo,
  });

  final String shipmentId;
  final String shipmentNo;
  final String fulfillmentId;
  final String carrierCode;
  final String status;

  /// Carrier tracking number; optional until the carrier assigns one.
  final String? trackingNo;

  static Shipment fromWire(Map<String, dynamic> wire) {
    String? optional(List<String> keys) {
      final value = asString(wire, keys);
      return value.isEmpty ? null : value;
    }

    return Shipment(
      shipmentId: asString(wire, const ['shipmentId', 'id']),
      shipmentNo: asString(wire, const ['shipmentNo']),
      fulfillmentId: asString(wire, const ['fulfillmentId']),
      carrierCode: asString(wire, const ['carrierCode']),
      status: asString(wire, const ['status']),
      trackingNo: optional(const ['trackingNo']),
    );
  }
}

/// One tracking event (`data.items[]` of
/// `GET /shipments/{shipmentId}/tracking_events`).
class TrackingEvent {
  const TrackingEvent({
    required this.eventId,
    required this.trackingEventNo,
    required this.shipmentId,
    required this.eventType,
    required this.eventTime,
    this.eventStatus,
    this.locationText,
  });

  final String eventId;
  final String trackingEventNo;
  final String shipmentId;
  final String eventType;

  /// Optional carrier status; `eventType` is the authoritative discriminator.
  final String? eventStatus;
  final String eventTime;
  final String? locationText;

  static TrackingEvent fromWire(Map<String, dynamic> wire) {
    String? optional(List<String> keys) {
      final value = asString(wire, keys);
      return value.isEmpty ? null : value;
    }

    return TrackingEvent(
      eventId: asString(wire, const ['eventId', 'id']),
      trackingEventNo: asString(wire, const ['trackingEventNo']),
      shipmentId: asString(wire, const ['shipmentId']),
      eventType: asString(wire, const ['eventType']),
      eventStatus: optional(const ['eventStatus']),
      eventTime: asString(wire, const ['eventTime']),
      locationText: optional(const ['locationText']),
    );
  }
}

/// Assembled logistics read model for one buyer order: the first fulfillment
/// page row plus its shipment header and tracking events. `shipment` /
/// `events` stay null/empty when the carrier leg has not started.
class OrderShipment {
  const OrderShipment({required this.fulfillment, this.shipment, this.events = const <TrackingEvent>[]});

  final Fulfillment fulfillment;
  final Shipment? shipment;
  final List<TrackingEvent> events;
}
