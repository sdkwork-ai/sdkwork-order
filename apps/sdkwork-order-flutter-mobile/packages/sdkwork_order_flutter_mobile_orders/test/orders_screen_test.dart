import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';
import 'package:sdkwork_order_flutter_mobile_orders/sdkwork_order_flutter_mobile_orders.dart';

class ScriptedTransport implements OrderApiTransport {
  ScriptedTransport(this._routes);

  final Map<String, Map<String, dynamic>> _routes;

  @override
  Future<Map<String, dynamic>> request(
    String path, {
    String method = 'GET',
    Map<String, dynamic>? body,
    Map<String, String>? query,
    String? idempotencyKey,
  }) async {
    final data = _routes['$method $path'];
    if (data == null) {
      throw OrderApiException('未路由的请求：$method $path');
    }
    return data;
  }
}

void main() {
  testWidgets('orders screen renders a pending order with pay/cancel actions',
      (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: OrdersScreen(
            orderService: OrderService(
              ScriptedTransport({
                'GET /orders': {
                  'items': [
                    {
                      'orderId': '9007199254740993',
                      'orderSn': 'SN-1',
                      'status': 'pending_payment',
                      'statusName': '待付款',
                      'subject': '积分充值订单',
                      'totalAmount': '6990',
                      'currencyCode': 'CNY',
                      'quantity': 1,
                      'createdAt': '2026-10-03T08:00:00Z',
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
              }),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(find.text('积分充值订单'), findsOneWidget);
    expect(find.text('¥69.90'), findsOneWidget);
    expect(find.text('去支付'), findsOneWidget);
    expect(find.text('订单详情'), findsOneWidget);
  });

  testWidgets('orders screen covers the error state with retry', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: OrdersScreen(orderService: OrderService(ScriptedTransport({}))),
        ),
      ),
    );
    await tester.pump();
    expect(find.textContaining('未路由的请求'), findsOneWidget);
    expect(find.text('重试'), findsOneWidget);
  });

  testWidgets('orders screen covers the empty state', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: OrdersScreen(
            orderService: OrderService(
              ScriptedTransport({
                'GET /orders': {
                  'items': <Object?>[],
                  'pageInfo': <String, dynamic>{'hasMore': false},
                },
              }),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(find.text('暂无订单'), findsOneWidget);
  });
}
