import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

import 'package:sdkwork_order_flutter_mobile_core/src/logic/idempotency.dart';
import 'package:sdkwork_order_flutter_mobile_core/src/services/order_service.dart';
import 'package:sdkwork_order_flutter_mobile_core/src/transport/order_transport.dart';

/// Loopback [HttpServer] test for the buyer receipt-confirmation command:
/// asserts the wire method, path, `Idempotency-Key` header, and the empty
/// JSON command body on the real `dart:io` transport (precedent:
/// `transport_test.dart`). Mirrors the mini-program `confirmReceipt`
/// authority: `POST /orders/{orderId}/receipt_confirmations`, 201, no body.
void main() {
  late HttpServer server;
  late OrderService service;
  final captured = <CapturedCall>[];

  setUp(() async {
    captured.clear();
    server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    server.listen((request) async {
      final body = await utf8.decoder.bind(request).join();
      captured.add(CapturedCall(request, body));
      if (request.method == 'POST' &&
          request.uri.path == '/orders/o-77/receipt_confirmations') {
        request.response.statusCode = HttpStatus.created;
        // 201 with no body (SdkWorkCommandData accepted shape).
        await request.response.close();
        return;
      }
      request.response.statusCode = HttpStatus.notFound;
      await request.response.close();
    });
    final transport = HttpOrderApiTransport(
      appApiBaseUrl: 'http://127.0.0.1:${server.port}',
      tokenProvider: () => 'token-123',
    );
    service = OrderService(transport);
  });

  tearDown(() async {
    await server.close(force: true);
  });

  test('confirmReceipt posts an empty body with a fresh Idempotency-Key',
      () async {
    await service.confirmReceipt('o-77');

    final call = captured.single;
    expect(call.request.method, 'POST');
    expect(call.request.uri.path, '/orders/o-77/receipt_confirmations');
    expect(
      call.request.headers.value(HttpHeaders.authorizationHeader),
      'Bearer token-123',
    );
    final idempotencyKey = call.request.headers.value('Idempotency-Key');
    expect(idempotencyKey, isNotNull);
    expect(idempotencyKey, isNotEmpty);
    // RFC 4122 v4 shape from the shared idempotency helper.
    expect(
      RegExp(
        r'^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$',
      ).hasMatch(idempotencyKey!),
      isTrue,
    );
    expect(newIdempotencyKey(), isNot(idempotencyKey));
    expect(jsonDecode(call.body), <String, dynamic>{});
    expect(call.response.statusCode, HttpStatus.created);
  });

  test('confirmReceipt tolerates a 201 problem-free empty response', () async {
    // The command completes without throwing: 201 + empty body unwraps to an
    // empty data payload and resolves.
    await expectLater(service.confirmReceipt('o-77'), completes);
    expect(captured, hasLength(1));
  });

  test('confirmReceipt maps a problem+json rejection to a typed failure',
      () async {
    await server.close(force: true);
    final rogue = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    addTearDown(() => rogue.close(force: true));
    rogue.listen((request) async {
      await utf8.decoder.bind(request).join();
      request.response.statusCode = HttpStatus.conflict;
      request.response.headers.contentType =
          ContentType('application', 'problem+json', charset: 'utf-8');
      request.response.write(jsonEncode({
        'title': '冲突',
        'detail': '订单状态不允许确认收货',
        'code': 40902,
        'traceId': 'trace-rc-1',
      }));
      await request.response.close();
    });
    final rogueService = OrderService(
      HttpOrderApiTransport(
        appApiBaseUrl: 'http://127.0.0.1:${rogue.port}',
        tokenProvider: () => '',
      ),
    );

    await expectLater(
      rogueService.confirmReceipt('o-77'),
      throwsA(
        isA<OrderApiException>()
            .having((e) => e.message, 'message', '订单状态不允许确认收货')
            .having((e) => e.code, 'code', '40902')
            .having((e) => e.traceId, 'traceId', 'trace-rc-1')
            .having((e) => e.statusCode, 'statusCode', HttpStatus.conflict),
      ),
    );
  });
}

class CapturedCall {
  CapturedCall(this.request, this.body);

  final HttpRequest request;
  final String body;

  HttpResponse get response => request.response;
}
