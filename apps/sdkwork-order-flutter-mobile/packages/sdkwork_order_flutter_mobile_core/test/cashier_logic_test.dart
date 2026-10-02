import 'package:flutter_test/flutter_test.dart';

import 'package:sdkwork_order_flutter_mobile_core/src/logic/cashier_logic.dart';

void main() {
  group('resolveCashierPhase', () {
    test('paid wins immediately', () {
      expect(
        resolveCashierPhase(paid: true, orderStatus: 'pending_payment'),
        CashierPhase.paid,
      );
    });

    test('maps terminal order statuses', () {
      expect(
        resolveCashierPhase(paid: false, orderStatus: 'cancelled'),
        CashierPhase.cancelled,
      );
      expect(
        resolveCashierPhase(paid: false, orderStatus: 'closed'),
        CashierPhase.cancelled,
      );
      expect(
        resolveCashierPhase(paid: false, orderStatus: 'expired'),
        CashierPhase.expired,
      );
      expect(
        resolveCashierPhase(paid: false, orderStatus: 'timeout'),
        CashierPhase.expired,
      );
      expect(
        resolveCashierPhase(paid: false, orderStatus: 'failed'),
        CashierPhase.failed,
      );
    });

    test('maps post-paid and refunding statuses to paid', () {
      for (final status in const [
        'paid',
        'fulfilled',
        'completed',
        'shipped',
        'delivered',
        'refunding',
        'refunded',
      ]) {
        expect(
          resolveCashierPhase(paid: false, orderStatus: status),
          CashierPhase.paid,
          reason: status,
        );
      }
    });

    test('stays pending otherwise', () {
      expect(
        resolveCashierPhase(paid: false, orderStatus: 'pending_payment'),
        CashierPhase.pending,
      );
    });
  });

  group('nextPollDelayMs', () {
    test('normal cadence is 3s', () {
      expect(nextPollDelayMs(consecutiveFailures: 0), 3000);
      expect(nextPollDelayMs(consecutiveFailures: 2), 3000);
    });

    test('backs off to 6s from the third consecutive failure', () {
      expect(nextPollDelayMs(consecutiveFailures: 3), 6000);
      expect(nextPollDelayMs(consecutiveFailures: 9), 6000);
    });
  });

  group('computeCashierRemainingSeconds', () {
    final createdAt = DateTime.utc(2026, 10, 3, 12, 0, 0).millisecondsSinceEpoch;

    test('uses the earlier of expireTime and createdAt + 15min TTL', () {
      // expireTime after the TTL → TTL wins.
      final ttlWins = computeCashierRemainingSeconds(
        '2026-10-03T13:00:00Z',
        createdAt,
        createdAt,
      );
      expect(ttlWins, cashierTtlSeconds);

      // expireTime before the TTL → expireTime wins.
      final expireWins = computeCashierRemainingSeconds(
        '2026-10-03T12:05:00Z',
        createdAt,
        createdAt,
      );
      expect(expireWins, 300);
    });

    test('floors at zero once the deadline passed', () {
      expect(
        computeCashierRemainingSeconds(
          '2026-10-03T11:00:00Z',
          createdAt,
          createdAt + 10 * 60 * 1000,
        ),
        0,
      );
      expect(
        computeCashierRemainingSeconds(null, createdAt, createdAt + 16 * 60 * 1000),
        0,
      );
    });

    test('ignores malformed expireTime and falls back to the TTL', () {
      expect(
        computeCashierRemainingSeconds('not-a-date', createdAt, createdAt),
        cashierTtlSeconds,
      );
    });
  });

  group('formatCashierCountdown', () {
    test('formats mm:ss and clamps negatives', () {
      expect(formatCashierCountdown(0), '00:00');
      expect(formatCashierCountdown(65), '01:05');
      expect(formatCashierCountdown(600), '10:00');
      expect(formatCashierCountdown(-3), '00:00');
    });
  });

  test('failed and expired phases are retryable', () {
    expect(isCashierRetryablePhase(CashierPhase.failed), isTrue);
    expect(isCashierRetryablePhase(CashierPhase.expired), isTrue);
    expect(isCashierRetryablePhase(CashierPhase.pending), isFalse);
    expect(isCashierRetryablePhase(CashierPhase.paid), isFalse);
    expect(isCashierRetryablePhase(CashierPhase.cancelled), isFalse);
  });
}
