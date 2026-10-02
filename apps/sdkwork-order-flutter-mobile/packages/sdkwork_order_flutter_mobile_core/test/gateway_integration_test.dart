import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

/// Live-gateway transport integration tests.
///
/// Skipped unless `SDKWORK_ORDER_GATEWAY_BASE_URL` points at a running
/// order standalone gateway (for example
/// `http://127.0.0.1:3901`). These prove the single dart:io transport seam
/// against the real wire: `{code,data,traceId}` unwrapping, seeded public
/// reads, and problem+json mapping for auth failures.
void main() {
  final baseUrl = Platform.environment['SDKWORK_ORDER_GATEWAY_BASE_URL']?.trim() ?? '';

  if (baseUrl.isEmpty) {
    test('gateway integration skipped without SDKWORK_ORDER_GATEWAY_BASE_URL', () {
      // A silent skip keeps `flutter test` green in offline/unit runs.
      return;
    });
    return;
  }

  // No TestWidgetsFlutterBinding here: the binding replaces HttpClient with
  // a mock channel, while these tests intentionally drive real sockets.

  test('public recharge plans read returns seeded items over the real wire', () async {
    final transport = HttpOrderApiTransport(
      appApiBaseUrl: '$baseUrl/app/v3/api',
      tokenProvider: () => null,
    );
    final data = await transport.request(
      '/recharges/plans',
      query: const {'page': '1', 'page_size': '3'},
    );
    final items = data['items'];
    expect(items, isA<List<dynamic>>());
    expect((items as List<dynamic>), isNotEmpty);
    final first = items.first as Map<String, dynamic>;
    expect(first['planCode'], isA<String>());
    // Money/points cross the wire as strings (int64-safe), never doubles.
    expect(first['priceAmount'], isA<String>());
  });

  test('unauthenticated order list is rejected as problem+json 401', () async {
    final transport = HttpOrderApiTransport(
      appApiBaseUrl: '$baseUrl/app/v3/api',
      tokenProvider: () => null,
    );
    await expectLater(
      transport.request('/orders', query: const {'page': '1', 'page_size': '1'}),
      throwsA(
        isA<OrderApiException>()
            .having((error) => error.statusCode, 'statusCode', 401)
            .having((error) => error.code, 'code', isNotNull),
      ),
    );
  });
}
