import 'package:flutter_test/flutter_test.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';
import 'package:sdkwork_order_flutter_mobile_orders/sdkwork_order_flutter_mobile_orders.dart';

import 'package:sdkwork_order_flutter_mobile/bootstrap/providers.dart';
import 'package:sdkwork_order_flutter_mobile/shell/mobile_shell.dart';

/// Scripted fake transport: throws for un-routed calls so widget tests only
/// exercise the surfaces they stub.
class FakeOrderApiTransport implements OrderApiTransport {
  FakeOrderApiTransport(this._routes);

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

AppEnvironment get _testEnvironment => const AppEnvironment(
      environment: 'development',
      deploymentProfile: 'standalone',
      profileId: 'sdkwork-order-flutter-mobile',
      runtimeTarget: 'mobile',
      orderAppApiBaseUrl: 'https://api-dev.example.test/app/v3/api',
    );

void main() {
  tearDown(() {
    OrderMobileProviders.resetForTesting();
  });

  testWidgets('mobile shell pumps the three-tab order surface', (tester) async {
    OrderMobileProviders.initForTesting(
      _testEnvironment,
      FakeOrderApiTransport({
        'GET /orders': {
          'items': <Object?>[],
          'pageInfo': <String, dynamic>{'hasMore': false},
        },
        'GET /recharges/plans': {
          'items': <Object?>[],
          'pageInfo': <String, dynamic>{},
        },
        'GET /recharges/packages': {
          'items': <Object?>[],
          'pageInfo': <String, dynamic>{},
        },
      }),
    );
    await tester.pumpWidget(const SdkworkOrderMobileShell());
    await tester.pump(const Duration(milliseconds: 300));

    expect(find.text('订单中心'), findsOneWidget);
    expect(find.text(tabRecharge), findsOneWidget);
    expect(find.text(tabProfile), findsOneWidget);
    expect(find.text('全部'), findsOneWidget);
    expect(find.text('暂无订单'), findsOneWidget);
  });

  testWidgets('profile tab shows the login entry and services', (tester) async {
    OrderMobileProviders.initForTesting(
      _testEnvironment,
      FakeOrderApiTransport({
        'GET /orders': {
          'items': <Object?>[],
          'pageInfo': <String, dynamic>{},
        },
        'GET /recharges/plans': {
          'items': <Object?>[],
          'pageInfo': <String, dynamic>{},
        },
        'GET /recharges/packages': {
          'items': <Object?>[],
          'pageInfo': <String, dynamic>{},
        },
      }),
    );
    await tester.pumpWidget(const SdkworkOrderMobileShell());
    await tester.tap(find.text(tabProfile));
    await tester.pump(const Duration(milliseconds: 300));

    expect(find.text('未登录'), findsOneWidget);
    expect(find.text('点击登录'), findsOneWidget);
    expect(find.text('常用服务'), findsOneWidget);
    expect(find.text('券码兑换'), findsOneWidget);
    expect(find.text('余额提现'), findsOneWidget);
    expect(find.text('退款申请'), findsOneWidget);
    expect(find.text('登录后查看订单统计'), findsOneWidget);
  });

  testWidgets('recharge tab shows empty plan/package sections offline-safe',
      (tester) async {
    OrderMobileProviders.initForTesting(
      _testEnvironment,
      FakeOrderApiTransport({
        'GET /orders': {
          'items': <Object?>[],
          'pageInfo': <String, dynamic>{},
        },
        'GET /recharges/plans': {
          'items': <Object?>[],
          'pageInfo': <String, dynamic>{},
        },
        'GET /recharges/packages': {
          'items': <Object?>[],
          'pageInfo': <String, dynamic>{},
        },
      }),
    );
    await tester.pumpWidget(const SdkworkOrderMobileShell());
    await tester.tap(find.text(tabRecharge));
    await tester.pump(const Duration(milliseconds: 300));

    expect(find.text('充值中心'), findsOneWidget);
    expect(find.text('Token Bank 直充计划'), findsOneWidget);
    expect(find.text('充值套餐'), findsOneWidget);
  });

  testWidgets('orders tab renders one pending order card', (tester) async {
    OrderMobileProviders.initForTesting(
      _testEnvironment,
      FakeOrderApiTransport({
        'GET /orders': {
          'items': [
            {
              'orderId': '9007199254740993',
              'orderSn': 'SN-1',
              'status': 'pending_payment',
              'statusName': '待付款',
              'subject': 'Token Bank 充值订单',
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
        'GET /recharges/plans': {
          'items': <Object?>[],
          'pageInfo': <String, dynamic>{},
        },
        'GET /recharges/packages': {
          'items': <Object?>[],
          'pageInfo': <String, dynamic>{},
        },
      }),
    );
    await tester.pumpWidget(const SdkworkOrderMobileShell());
    await tester.pump(const Duration(milliseconds: 300));

    expect(find.text('Token Bank 充值订单'), findsOneWidget);
    expect(find.text('¥69.90'), findsOneWidget);
    expect(find.text('待付款'), findsWidgets);
    expect(find.text('去支付'), findsOneWidget);
  });
}
