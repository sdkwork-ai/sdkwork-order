import 'package:flutter_test/flutter_test.dart';

import 'package:sdkwork_order_flutter_mobile_core/src/logic/idempotency.dart';

void main() {
  test('mints UUID-v4-shaped keys', () {
    final key = newIdempotencyKey();
    expect(
      RegExp(
        r'^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$',
      ).hasMatch(key),
      isTrue,
      reason: key,
    );
  });

  test('keys are unique across calls', () {
    final keys = {for (var i = 0; i < 100; i++) newIdempotencyKey()};
    expect(keys.length, 100);
  });
}
