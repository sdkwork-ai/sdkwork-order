import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';
import 'package:sdkwork_order_flutter_mobile_orders/sdkwork_order_flutter_mobile_orders.dart';

/// Scripted fake transport for screen tests.
class ScriptedTransport implements OrderApiTransport {
  ScriptedTransport(this._routes);

  final Map<String, Map<String, dynamic>> _routes;
  final List<String> calls = <String>[];

  @override
  Future<Map<String, dynamic>> request(
    String path, {
    String method = 'GET',
    Map<String, dynamic>? body,
    Map<String, String>? query,
    String? idempotencyKey,
  }) async {
    calls.add('$method $path');
    final data = _routes['$method $path'];
    if (data == null) {
      throw OrderApiException('未路由的请求：$method $path');
    }
    return data;
  }
}

void main() {
  testWidgets('coupon screen blocks empty codes with a validation error',
      (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: CouponRedemptionScreen(orderService: OrderService(ScriptedTransport({}))),
      ),
    );
    await tester.tap(find.text('立即兑换'));
    await tester.pump();
    expect(find.text('请输入券码'), findsOneWidget);
  });

  testWidgets('coupon screen renders the benefit result', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: CouponRedemptionScreen(
          orderService: OrderService(
            ScriptedTransport({
              'POST /orders/coupon_redemptions': {
                'item': {
                  'orderId': 'r-1',
                  'orderNo': 'RN-1',
                  'status': 'completed',
                  'replayed': true,
                  'benefit': {
                    'kind': 'token_bank_credit',
                    'grantAmount': '5000',
                  },
                },
              },
            }),
          ),
        ),
      ),
    );
    await tester.enterText(find.byType(TextField), ' ab12 ');
    await tester.tap(find.text('立即兑换'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 200));

    expect(find.text('兑换成功'), findsOneWidget);
    expect(find.textContaining('Token Bank 额度'), findsOneWidget);
    expect(find.text('到账额度：¥50.00'), findsOneWidget);
    expect(find.text('兑换单号：RN-1'), findsOneWidget);
  });

  testWidgets('coupon screen surfaces API failures as an error card',
      (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: CouponRedemptionScreen(
          orderService: OrderService(ScriptedTransport({})),
        ),
      ),
    );
    await tester.enterText(find.byType(TextField), 'BADCODE');
    await tester.tap(find.text('立即兑换'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 200));

    expect(find.textContaining('未路由的请求'), findsOneWidget);
  });
}
