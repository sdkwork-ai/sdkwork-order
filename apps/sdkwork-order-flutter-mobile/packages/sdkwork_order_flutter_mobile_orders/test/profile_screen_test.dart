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
  setUp(() {
    // 会话是进程级单例：每个用例前清空，保证隔离。
    AppSession.instance.signOut();
  });

  testWidgets('profile screen shows login entry and service tiles when signed out',
      (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ProfileScreen(
            orderService: OrderService(ScriptedTransport({})),
          ),
        ),
      ),
    );
    await tester.pump();

    expect(find.text('未登录'), findsOneWidget);
    expect(find.text('点击登录'), findsOneWidget);
    expect(find.text('登录后查看订单统计'), findsOneWidget);
    expect(find.text('常用服务'), findsOneWidget);
    expect(find.text('券码兑换'), findsOneWidget);
    expect(find.text('余额提现'), findsOneWidget);
    expect(find.text('退款申请'), findsOneWidget);
  });

  testWidgets('profile screen loads statistics once signed in', (tester) async {
    AppSession.instance.signIn('token-e2e');
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ProfileScreen(
            orderService: OrderService(
              ScriptedTransport({
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
              }),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.text('已登录'), findsOneWidget);
    expect(find.text('订单统计'), findsOneWidget);
    expect(find.text('12'), findsOneWidget);
    expect(find.text('累计金额：¥1234.56'), findsOneWidget);
  });

  testWidgets('withdrawal screen validates the amount before submitting',
      (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: WithdrawalScreen(
            withdrawalService: WithdrawalService(ScriptedTransport({})),
          ),
        ),
      ),
    );
    await tester.enterText(find.byType(TextField).first, 'abc');
    await tester.tap(find.text('提交提现申请'));
    await tester.pump();
    expect(find.textContaining('请输入有效的提现金额'), findsOneWidget);
  });

  testWidgets('refund screen renders the form and empty records state',
      (tester) async {
    // The form is tall; widen the test surface so the records section is on
    // screen without scrolling.
    tester.view.physicalSize = const Size(1080, 2400);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RefundRequestScreen(
            refundService: RefundService(
              ScriptedTransport({
                'GET /orders/refund_requests': {
                  'items': <Object?>[],
                  'pageInfo': <String, dynamic>{},
                },
              }),
            ),
            prefillOrderId: 'o-1',
            prefillAmount: '69.90',
          ),
        ),
      ),
    );
    await tester.pump();
    expect(find.text('原始订单号'), findsOneWidget);
    expect(find.text('我的退款记录'), findsOneWidget);
    expect(find.text('暂无退款记录'), findsOneWidget);
    expect(find.text('Token Bank'), findsOneWidget);
  });
}
