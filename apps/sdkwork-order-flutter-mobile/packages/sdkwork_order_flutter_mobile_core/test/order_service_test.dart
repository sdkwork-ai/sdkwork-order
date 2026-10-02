import 'package:flutter_test/flutter_test.dart';

import 'package:sdkwork_order_flutter_mobile_core/src/services/order_service.dart';
import 'package:sdkwork_order_flutter_mobile_core/src/transport/order_transport.dart';

/// Scripted fake transport: records calls, replays canned envelopes.
class FakeOrderApiTransport implements OrderApiTransport {
  FakeOrderApiTransport(this._routes);

  final Map<String, Map<String, dynamic>> _routes;
  final List<Call> calls = <Call>[];

  String? lastIdempotencyKey;

  void route(String key, Map<String, dynamic> data) => _routes[key] = data;

  @override
  Future<Map<String, dynamic>> request(
    String path, {
    String method = 'GET',
    Map<String, dynamic>? body,
    Map<String, String>? query,
    String? idempotencyKey,
  }) async {
    calls.add(Call(method, path, body, query, idempotencyKey));
    lastIdempotencyKey = idempotencyKey;
    final data = _routes['$method $path'];
    if (data == null) {
      throw OrderApiException('no route for $method $path');
    }
    return data;
  }
}

class Call {
  const Call(this.method, this.path, this.body, this.query, this.idempotencyKey);

  final String method;
  final String path;
  final Map<String, dynamic>? body;
  final Map<String, String>? query;
  final String? idempotencyKey;
}

void main() {
  test('listOrders hits /orders with page params and maps items + pageInfo',
      () async {
    final transport = FakeOrderApiTransport({
      'GET /orders': {
        'items': [
          {
            'orderId': '9007199254740993',
            'orderSn': 'SN1',
            'status': 'pending_payment',
            'statusName': '待付款',
            'subject': '测试订单',
            'totalAmount': '6990',
            'currencyCode': 'CNY',
            'quantity': 1,
            'createdAt': '2026-10-03T08:00:00Z',
            'expireTime': '2026-10-03T08:15:00Z',
            'items': [
              {
                'id': 'i-1',
                'productName': '商品 A',
                'quantity': 2,
                'unitPrice': '3495',
                'totalAmount': '6990',
              }
            ],
          }
        ],
        'pageInfo': {
          'mode': 'offset',
          'page': 1,
          'pageSize': 20,
          'totalItems': '1',
          'hasMore': false,
        },
      },
    });
    final service = OrderService(transport);

    final page = await service.listOrders(status: 'pending_payment', page: 1);

    expect(transport.calls.single.method, 'GET');
    expect(transport.calls.single.path, '/orders');
    expect(transport.calls.single.query, {
      'page': '1',
      'page_size': '20',
      'status': 'pending_payment',
    });
    expect(page.items.single.id, '9007199254740993');
    expect(page.items.single.items.single.productName, '商品 A');
    expect(page.items.single.totalAmount, '6990');
    expect(page.pageInfo.totalItems, '1');
    expect(page.pageInfo.hasMore, isFalse);
  });

  test('listOrders omits the status query for the 全部 tab', () async {
    final transport = FakeOrderApiTransport({
      'GET /orders': {
        'items': <Object?>[],
        'pageInfo': <String, dynamic>{},
      },
    });
    await OrderService(transport).listOrders();
    expect(transport.calls.single.query, {'page': '1', 'page_size': '20'});
  });

  test('createPayment posts paymentMethod with an Idempotency-Key', () async {
    final transport = FakeOrderApiTransport({
      'POST /orders/o-1/payments': {
        'item': {
          'amount': '6990',
          'orderId': 'o-1',
          'outTradeNo': 'T1',
          'paymentId': 'p-1',
          'paymentMethod': 'balance',
          'paymentParams': {'channel': 'balance'},
        },
      },
    });
    final session =
        await OrderService(transport).createPayment('o-1', 'balance');

    expect(transport.calls.single.idempotencyKey, isNotEmpty);
    expect(transport.calls.single.body, {'paymentMethod': 'balance'});
    expect(session.paymentId, 'p-1');
    expect(session.amount, '6990');
    expect(session.paymentParams['channel'], 'balance');
  });

  test('createPayment rejects non-cashier methods', () async {
    final transport = FakeOrderApiTransport({});
    await expectLater(
      OrderService(transport).createPayment('o-1', 'crypto'),
      throwsA(isA<OrderApiException>()),
    );
  });

  test('cancelOrder posts an empty command with an idempotency key',
      () async {
    final transport = FakeOrderApiTransport({
      'POST /orders/o-1/cancellations': {'accepted': true, 'status': 'cancelled'},
    });
    await OrderService(transport).cancelOrder('o-1');
    expect(transport.calls.single.body, <String, dynamic>{});
    expect(transport.lastIdempotencyKey, isNotEmpty);
  });

  test('confirmReceipt posts an empty command with an idempotency key',
      () async {
    final transport = FakeOrderApiTransport({
      'POST /orders/o-1/receipt_confirmations': <String, dynamic>{},
    });
    await OrderService(transport).confirmReceipt('o-1');
    expect(transport.calls.single.method, 'POST');
    expect(transport.calls.single.path, '/orders/o-1/receipt_confirmations');
    expect(transport.calls.single.body, <String, dynamic>{});
    expect(transport.lastIdempotencyKey, isNotEmpty);
  });

  test('redeemCoupon uppercases the code and unwraps the benefit', () async {
    final transport = FakeOrderApiTransport({
      'POST /orders/coupon_redemptions': {
        'item': {
          'orderId': 'r-1',
          'orderNo': 'RN1',
          'status': 'completed',
          'replayed': false,
          'benefit': {'kind': 'token_bank_credit', 'grantAmount': '5000'},
        },
      },
    });
    final result = await OrderService(transport).redeemCoupon(' ab12 ');

    expect(transport.calls.single.body, {'couponCode': 'AB12'});
    expect(transport.lastIdempotencyKey, isNotEmpty);
    expect(result.completed, isTrue);
    expect(result.orderId, 'r-1');
    expect(result.benefitKind, 'token_bank_credit');
    expect(result.grantAmount, '5000');
  });

  test('getPaymentSuccess unwraps data.item', () async {
    final transport = FakeOrderApiTransport({
      'GET /orders/o-1/payment_success': {
        'item': {'paid': true, 'status': 'paid', 'statusName': '已支付'},
      },
    });
    final status = await OrderService(transport).getPaymentSuccess('o-1');
    expect(status.paid, isTrue);
    expect(status.status, 'paid');
  });

  test('statistics map the camelCase wire fields', () async {
    final transport = FakeOrderApiTransport({
      'GET /orders/statistics': {
        'item': {
          'totalOrders': 12,
          'pendingPayment': 2,
          'pendingShipment': 3,
          'pendingReceipt': 4,
          'completed': 3,
          'totalAmount': '123456',
        },
      },
    });
    final stats = await OrderService(transport).getStatistics();
    expect(stats.totalOrders, 12);
    expect(stats.completed, 3);
    expect(stats.totalAmount, '123456');
  });
}
