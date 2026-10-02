import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';
import 'package:sdkwork_order_flutter_mobile_orders/src/l10n/strings.dart';
import 'package:sdkwork_order_flutter_mobile_orders/src/screens/order_detail_screen.dart';

/// Scripted transport with mutable order state: flips the order status to
/// `completed` once the receipt confirmation command is posted, so the test
/// covers both the command wire and the post-confirm re-render.
class ScriptedTransport implements OrderApiTransport {
  ScriptedTransport();

  String orderStatus = 'fulfilled';
  bool withLogistics = false;
  final List<Call> calls = <Call>[];

  Map<String, dynamic> get _orderBody => <String, dynamic>{
        'item': <String, dynamic>{
          'orderId': 'o-1',
          'orderSn': 'SN-1',
          'status': orderStatus,
          'statusName': orderStatus == 'fulfilled' ? '待收货' : '已完成',
          'subject': '测试订单',
          'totalAmount': '6990',
          'currencyCode': 'CNY',
          'quantity': 1,
          'createdAt': '2026-10-03T08:00:00Z',
          'items': <Object?>[],
        },
      };

  Map<String, dynamic> _data(String method, String path) {
    if (method == 'GET' && path == '/orders/o-1') {
      return _orderBody;
    }
    if (method == 'GET' && path == '/orders/o-1/events') {
      return <String, dynamic>{
        'items': <Object?>[],
        'pageInfo': <String, dynamic>{'hasMore': false},
      };
    }
    if (method == 'GET' && path == '/fulfillments') {
      return <String, dynamic>{
        'items': <Object?>[
          if (withLogistics)
            <String, dynamic>{
              'fulfillmentId': 'f-1',
              'fulfillmentNo': 'FL-1',
              'fulfillmentType': 'standard',
              'orderId': 'o-1',
              'status': 'shipped',
            },
        ],
        'pageInfo': <String, dynamic>{'hasMore': false},
      };
    }
    if (method == 'GET' && path == '/shipments/f-1') {
      return <String, dynamic>{
        'item': <String, dynamic>{
          'shipmentId': 'f-1',
          'shipmentNo': 'SH-1',
          'fulfillmentId': 'f-1',
          'carrierCode': 'sf_express',
          'status': 'in_transit',
          'trackingNo': 'SF123456789',
        },
      };
    }
    if (method == 'GET' && path == '/shipments/f-1/tracking_events') {
      return <String, dynamic>{
        'items': <Object?>[
          <String, dynamic>{
            'eventId': 'e-1',
            'trackingEventNo': 'TE-1',
            'shipmentId': 'f-1',
            'eventType': 'picked_up',
            'eventTime': '2026-10-03T08:00:00Z',
            'locationText': '杭州转运中心',
          },
        ],
        'pageInfo': <String, dynamic>{'hasMore': false},
      };
    }
    if (method == 'POST' && path == '/orders/o-1/receipt_confirmations') {
      orderStatus = 'completed';
      return <String, dynamic>{};
    }
    throw OrderApiException('未路由的请求：$method $path');
  }

  @override
  Future<Map<String, dynamic>> request(
    String path, {
    String method = 'GET',
    Map<String, dynamic>? body,
    Map<String, String>? query,
    String? idempotencyKey,
  }) async {
    calls.add(Call(method, path, body, idempotencyKey));
    return _data(method, path);
  }
}

class Call {
  const Call(this.method, this.path, this.body, this.idempotencyKey);

  final String method;
  final String path;
  final Map<String, dynamic>? body;
  final String? idempotencyKey;
}

Widget _host(OrderService orderService, {ShipmentService? shipmentService}) {
  return MaterialApp(
    home: OrderDetailScreen(
      orderService: orderService,
      shipmentService: shipmentService,
      orderId: 'o-1',
    ),
  );
}

void main() {
  testWidgets('fulfilled order shows the 确认收货 primary button',
      (tester) async {
    final transport = ScriptedTransport();
    await tester.pumpWidget(
      _host(OrderService(transport), shipmentService: ShipmentService(transport)),
    );
    await tester.pump();

    expect(find.text(confirmReceiptButton), findsOneWidget);
    expect(find.text('申请退款'), findsOneWidget);
    expect(find.text('去支付'), findsNothing);
  });

  testWidgets('pending_payment order does not show 确认收货', (tester) async {
    final transport = ScriptedTransport()..orderStatus = 'pending_payment';
    await tester.pumpWidget(_host(OrderService(transport)));
    await tester.pump();

    expect(find.text(confirmReceiptButton), findsNothing);
    expect(find.text('去支付'), findsOneWidget);
    expect(find.text('取消订单'), findsOneWidget);
  });

  testWidgets('completed order does not show 确认收货', (tester) async {
    final transport = ScriptedTransport()..orderStatus = 'completed';
    await tester.pumpWidget(_host(OrderService(transport)));
    await tester.pump();

    expect(find.text(confirmReceiptButton), findsNothing);
    expect(find.text('申请退款'), findsOneWidget);
  });

  testWidgets('confirming receipt posts the command and refreshes the detail',
      (tester) async {
    final transport = ScriptedTransport();
    await tester.pumpWidget(_host(OrderService(transport)));
    await tester.pump();
    expect(transport.orderStatus, 'fulfilled');

    await tester.tap(find.text(confirmReceiptButton));
    await tester.pumpAndSettle();

    // The dialog's only FilledButton is the confirm action (its label equals
    // the dialog title, so scope by widget type).
    await tester.tap(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.byType(FilledButton),
      ),
    );
    await tester.pumpAndSettle();

    final command = transport.calls.singleWhere(
      (call) => call.path == '/orders/o-1/receipt_confirmations',
    );
    expect(command.method, 'POST');
    expect(command.body, <String, dynamic>{});
    expect(command.idempotencyKey, isNotEmpty);
    expect(find.text(confirmReceiptSuccessToast), findsOneWidget);
    // Detail reloaded into the completed state: the button is gone.
    expect(find.text(confirmReceiptButton), findsNothing);
  });

  testWidgets('fulfilled order with a shipment renders the logistics card',
      (tester) async {
    final transport = ScriptedTransport()..withLogistics = true;
    await tester.pumpWidget(
      _host(OrderService(transport), shipmentService: ShipmentService(transport)),
    );
    await tester.pump();

    expect(find.text(logisticsCardTitle), findsOneWidget);
    expect(find.text('sf_express'), findsOneWidget);
    expect(find.text('SF123456789'), findsOneWidget);
    expect(find.text('picked_up'), findsOneWidget);
    expect(find.textContaining('杭州转运中心'), findsOneWidget);
  });

  testWidgets('fulfilled order without a fulfillment hides the logistics card',
      (tester) async {
    final transport = ScriptedTransport();
    await tester.pumpWidget(
      _host(OrderService(transport), shipmentService: ShipmentService(transport)),
    );
    await tester.pump();

    // The shipment read degrades to "no logistics"; the detail still renders.
    expect(find.text(logisticsCardTitle), findsNothing);
    expect(find.text('测试订单'), findsOneWidget);
  });
}
