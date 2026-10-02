/// Typed read models for the order domain, mapped defensively from loose
/// wire payloads (the app-api list/resource items are
/// `additionalProperties: true` until the generated Dart SDK family lands —
/// see root `docs/decisions.md`). Field names follow the Rust route
/// serializers (`serde(rename_all = "camelCase")`); ids and money stay
/// strings (int64 wire contract, `API_SPEC.md` §13.6).
library;

import '../logic/envelope.dart';


/// Order line item (`OrderItemResponse`).
class OrderItem {
  const OrderItem({
    required this.id,
    required this.productName,
    required this.quantity,
    required this.unitPrice,
    required this.totalAmount,
  });

  final String id;
  final String productName;
  final int quantity;

  /// Minor-unit amount string.
  final String unitPrice;

  /// Minor-unit amount string.
  final String totalAmount;

  static OrderItem fromWire(Map<String, dynamic> wire) => OrderItem(
        id: asString(wire, const ['id']),
        productName: asString(wire, const ['productName', 'title', 'subject']),
        quantity: asInt(wire, const ['quantity']),
        unitPrice: asString(wire, const ['unitPrice'], fallback: '0'),
        totalAmount: asString(wire, const ['totalAmount'], fallback: '0'),
      );
}

/// Order summary/detail (`OrderSummaryResponse` / `OrderDetailResponse`).
class Order {
  const Order({
    required this.id,
    required this.orderSn,
    required this.status,
    required this.statusName,
    required this.subject,
    required this.totalAmount,
    required this.currencyCode,
    required this.quantity,
    required this.createdAt,
    required this.items,
    this.paidAmount,
    this.discountAmount,
    this.payTime,
    this.expireTime,
    this.paymentMethod,
    this.outTradeNo,
    this.transactionId,
  });

  final String id;
  final String orderSn;
  final String status;
  final String statusName;
  final String subject;

  /// Minor-unit amount string.
  final String totalAmount;

  /// Minor-unit amount string, present once paid.
  final String? paidAmount;

  /// Minor-unit amount string.
  final String? discountAmount;
  final String currencyCode;
  final int quantity;
  final String createdAt;
  final String? payTime;
  final String? expireTime;
  final String? paymentMethod;
  final List<OrderItem> items;
  final String? outTradeNo;
  final String? transactionId;

  bool get isPendingPayment => status.trim().toLowerCase() == 'pending_payment';

  static Order fromWire(Map<String, dynamic> wire) {
    final id = asString(wire, const ['orderId', 'id']);
    return Order(
      id: id,
      orderSn: asString(wire, const ['orderSn'], fallback: id),
      status: asString(wire, const ['status'], fallback: 'pending_payment'),
      statusName: asString(
        wire,
        const ['statusName'],
        fallback: asString(wire, const ['status'], fallback: 'pending_payment'),
      ),
      subject: asString(wire, const ['subject'], fallback: '订单'),
      totalAmount: asString(wire, const ['totalAmount'], fallback: '0'),
      paidAmount: _optional(wire, const ['paidAmount']),
      discountAmount: _optional(wire, const ['discountAmount']),
      currencyCode: asString(wire, const ['currencyCode'], fallback: 'CNY'),
      quantity: asInt(wire, const ['quantity']),
      createdAt: asString(wire, const ['createdAt']),
      payTime: _optional(wire, const ['payTime']),
      expireTime: _optional(wire, const ['expireTime']),
      paymentMethod: _optional(wire, const ['paymentMethod']),
      items: asList(wire['items']).map(OrderItem.fromWire).toList(),
      outTradeNo: _optional(wire, const ['outTradeNo']),
      transactionId: _optional(wire, const ['transactionId']),
    );
  }

  static String? _optional(Map<String, dynamic> wire, List<String> keys) {
    for (final key in keys) {
      final value = wire[key];
      if (value != null && value.toString().isNotEmpty) {
        return value.toString();
      }
    }
    return null;
  }
}

/// One page of orders (`data.items + data.pageInfo`).
class OrderPage {
  const OrderPage({required this.items, required this.pageInfo});

  final List<Order> items;
  final PageInfo pageInfo;
}

/// Order lifecycle event (`OrderEventResponse`).
class OrderEvent {
  const OrderEvent({
    required this.eventId,
    required this.orderId,
    required this.eventType,
    required this.toStatus,
    required this.createdAt,
    this.fromStatus,
    this.actorType,
    this.actorId,
    this.message,
  });

  final String eventId;
  final String orderId;
  final String eventType;
  final String? fromStatus;
  final String toStatus;
  final String? actorType;
  final String? actorId;
  final String? message;
  final String createdAt;

  static OrderEvent fromWire(Map<String, dynamic> wire) => OrderEvent(
        eventId: asString(wire, const ['eventId', 'id']),
        orderId: asString(wire, const ['orderId']),
        eventType: asString(wire, const ['eventType']),
        fromStatus: _optional(wire, const ['fromStatus']),
        toStatus: asString(wire, const ['toStatus']),
        actorType: _optional(wire, const ['actorType']),
        actorId: _optional(wire, const ['actorId']),
        message: _optional(wire, const ['message']),
        createdAt: asString(wire, const ['createdAt']),
      );

  static String? _optional(Map<String, dynamic> wire, List<String> keys) {
    for (final key in keys) {
      final value = wire[key];
      if (value != null && value.toString().isNotEmpty) {
        return value.toString();
      }
    }
    return null;
  }
}

/// Buyer order statistics (`OrderStatisticsResponse`).
class OrderStatistics {
  const OrderStatistics({
    required this.totalOrders,
    required this.pendingPayment,
    required this.pendingShipment,
    required this.pendingReceipt,
    required this.completed,
    required this.totalAmount,
  });

  final int totalOrders;
  final int pendingPayment;
  final int pendingShipment;
  final int pendingReceipt;
  final int completed;

  /// Minor-unit amount string.
  final String totalAmount;

  static OrderStatistics fromWire(Map<String, dynamic> wire) =>
      OrderStatistics(
        totalOrders: asInt(wire, const ['totalOrders']),
        pendingPayment: asInt(wire, const ['pendingPayment']),
        pendingShipment: asInt(wire, const ['pendingShipment']),
        pendingReceipt: asInt(wire, const ['pendingReceipt']),
        completed: asInt(wire, const ['completed']),
        totalAmount: asString(wire, const ['totalAmount'], fallback: '0'),
      );
}

/// Payment session returned by `POST /orders/{orderId}/payments`
/// (`OrderPaymentParamsResponse`).
class PaymentSession {
  const PaymentSession({
    required this.amount,
    required this.orderId,
    required this.outTradeNo,
    required this.paymentId,
    required this.paymentMethod,
    required this.paymentParams,
  });

  /// Minor-unit amount string.
  final String amount;
  final String orderId;
  final String outTradeNo;
  final String paymentId;
  final String paymentMethod;
  final Map<String, String> paymentParams;

  static PaymentSession fromWire(Map<String, dynamic> wire) {
    final params = <String, String>{};
    wire['paymentParams']?.forEach((key, value) {
      if (value != null) {
        params['$key'] = '$value';
      }
    });
    final paymentId = asString(wire, const ['paymentId', 'id']);
    return PaymentSession(
      amount: asString(wire, const ['amount'], fallback: '0'),
      orderId: asString(wire, const ['orderId']),
      outTradeNo: asString(wire, const ['outTradeNo']),
      paymentId: paymentId,
      paymentMethod: asString(wire, const ['paymentMethod']),
      paymentParams: params,
    );
  }
}

/// Payment status returned by `GET /orders/{orderId}/payment_success`
/// (`OrderPaymentSuccessResponse`).
class PaymentStatus {
  const PaymentStatus({
    required this.paid,
    required this.status,
    required this.statusName,
  });

  final bool paid;
  final String status;
  final String statusName;

  static PaymentStatus fromWire(Map<String, dynamic> wire) => PaymentStatus(
        paid: asBool(wire, const ['paid']),
        status: asString(wire, const ['status']),
        statusName: asString(wire, const ['statusName']),
      );
}
