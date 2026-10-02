/// Client retry keys for write commands (the `Idempotency-Key` header per
/// `API_SPEC.md` section 14). A key is derived when a user attempt starts
/// and reused across transport retries of that attempt; a fresh key is
/// minted per new attempt.
library;

import 'dart:math';

/// Generates a random UUID v4 string. `dart:math`-based so the core package
/// needs no pub.dev dependency.
String newIdempotencyKey() {
  final random = Random.secure();
  final bytes = List<int>.generate(16, (_) => random.nextInt(256));
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  final hex = bytes.map((byte) => byte.toRadixString(16).padLeft(2, '0')).join();
  return '${hex.substring(0, 8)}-'
      '${hex.substring(8, 12)}-'
      '${hex.substring(12, 16)}-'
      '${hex.substring(16, 20)}-'
      '${hex.substring(20)}';
}
