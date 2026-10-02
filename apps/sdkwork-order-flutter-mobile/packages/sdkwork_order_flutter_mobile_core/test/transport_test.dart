import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

import 'package:sdkwork_order_flutter_mobile_core/src/transport/order_transport.dart';

/// Spins a loopback [HttpServer] and verifies the single transport seam:
/// envelope unwrap, ProblemDetail mapping, bearer header, Idempotency-Key
/// header, query encoding, and the bounded timeout.
void main() {
  late HttpServer server;
  late HttpOrderApiTransport transport;
  final captured = <CapturedCall>[];

  setUp(() async {
    captured.clear();
    server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    server.listen((request) async {
      // Drain the request body first so keep-alive and assertions behave.
      final body = await utf8.decoder.bind(request).join();
      captured.add(CapturedCall(request, body));
      final path = request.uri.path;
      if (path == '/orders/ok') {
        request.response.headers.contentType = ContentType.json;
        request.response.write(
          jsonEncode({
            'code': 0,
            'data': {'item': <String, dynamic>{'orderId': 'o-1'}},
            'traceId': '7e57d004-2b97-0e7a-b45f-5387367791cd',
          }),
        );
        await request.response.close();
        return;
      }
      if (path == '/orders/fail') {
        request.response.statusCode = HttpStatus.conflict;
        request.response.headers.contentType =
            ContentType('application', 'problem+json', charset: 'utf-8');
        request.response.write(
          jsonEncode({
            'title': '冲突',
            'detail': '订单状态不允许取消',
            'code': 40901,
            'traceId': 'aaa-bbb',
          }),
        );
        await request.response.close();
        return;
      }
      if (path == '/orders/slow') {
        // Never closes within the transport timeout.
        return;
      }
      request.response.statusCode = HttpStatus.notFound;
      await request.response.close();
    });
    transport = HttpOrderApiTransport(
      appApiBaseUrl: 'http://127.0.0.1:${server.port}',
      tokenProvider: () => 'token-123',
      responseTimeout: const Duration(milliseconds: 500),
    );
  });

  tearDown(() async {
    await server.close(force: true);
  });

  test('unwraps the {code,data,traceId} envelope to data', () async {
    final data = await transport.request('/orders/ok');
    expect(data['item'], <String, dynamic>{'orderId': 'o-1'});
  });

  test('sends bearer auth, json content type, query, and body', () async {
    await transport.request(
      '/orders/ok',
      method: 'POST',
      query: {'page': '1', 'page_size': '20', 'status': ''},
      body: <String, dynamic>{'paymentMethod': 'balance'},
      idempotencyKey: 'key-abc',
    );
    final call = captured.single;
    expect(
      call.request.headers.value(HttpHeaders.authorizationHeader),
      'Bearer token-123',
    );
    expect(call.request.headers.value('Idempotency-Key'), 'key-abc');
    expect(call.request.uri.queryParameters['page'], '1');
    expect(call.request.uri.queryParameters['page_size'], '20');
    // Empty query values are dropped.
    expect(call.request.uri.queryParameters.containsKey('status'), isFalse);
    expect(jsonDecode(call.body), <String, dynamic>{'paymentMethod': 'balance'});
  });

  test('omits the bearer header without a token', () async {
    final anonymous = HttpOrderApiTransport(
      appApiBaseUrl: 'http://127.0.0.1:${server.port}',
      tokenProvider: () => '',
    );
    await anonymous.request('/orders/ok');
    expect(
      captured.single.request.headers.value(HttpHeaders.authorizationHeader),
      isNull,
    );
  });

  test('maps ProblemDetail failures to a typed exception', () async {
    await expectLater(
      transport.request('/orders/fail'),
      throwsA(
        isA<OrderApiException>()
            .having((e) => e.message, 'message', '订单状态不允许取消')
            .having((e) => e.code, 'code', '40901')
            .having((e) => e.traceId, 'traceId', 'aaa-bbb')
            .having((e) => e.statusCode, 'statusCode', HttpStatus.conflict),
      ),
    );
    expect(captured.single.response.statusCode, HttpStatus.conflict);
  });

  test('surfaces a 2xx non-zero code as an API failure (legacy envelope)',
      () async {
    await server.close(force: true);
    final rogue = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    addTearDown(() => rogue.close(force: true));
    rogue.listen((request) async {
      await utf8.decoder.bind(request).join();
      request.response.headers.contentType = ContentType.json;
      request.response.write(jsonEncode({
        'code': 40002,
        'message': '参数错误',
        'traceId': 't-1',
        'data': null,
      }));
      await request.response.close();
    });
    final rogueTransport = HttpOrderApiTransport(
      appApiBaseUrl: 'http://127.0.0.1:${rogue.port}',
      tokenProvider: () => '',
    );
    await expectLater(
      rogueTransport.request('/x'),
      throwsA(
        isA<OrderApiException>()
            .having((e) => e.message, 'message', '参数错误')
            .having((e) => e.code, 'code', '40002'),
      ),
    );
  });

  test('times out bounded responses', () async {
    await expectLater(
      transport.request('/orders/slow'),
      throwsA(isA<TimeoutException>()),
    );
  });
}

class CapturedCall {
  CapturedCall(this.request, this.body);

  final HttpRequest request;
  final String body;

  HttpResponse get response => request.response;
}
