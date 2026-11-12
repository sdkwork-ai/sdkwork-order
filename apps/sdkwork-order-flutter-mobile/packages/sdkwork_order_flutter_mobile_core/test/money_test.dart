import 'package:flutter_test/flutter_test.dart';

import 'package:sdkwork_order_flutter_mobile_core/src/logic/money.dart';

void main() {
  group('formatMinorAmount', () {
    test('formats minor-unit strings with two decimals', () {
      expect(formatMinorAmount('6990'), '¥69.90');
      expect(formatMinorAmount('0'), '¥0.00');
      expect(formatMinorAmount('1'), '¥0.01');
      expect(formatMinorAmount('100000'), '¥1000.00');
    });

    test('keeps int64-scale precision (no double round-trip)', () {
      expect(formatMinorAmount('9007199254740993'), '¥90071992547409.93');
    });

    test('handles negative amounts', () {
      expect(formatMinorAmount('-6990'), '-¥69.90');
    });

    test('falls back for null, empty, and malformed input', () {
      expect(formatMinorAmount(null), '--');
      expect(formatMinorAmount(''), '--');
      expect(formatMinorAmount('abc'), 'abc');
    });
  });

  group('formatTokenBankPoints', () {
    test('converts ledger micro-points into whole Compute Credits', () {
      expect(formatTokenBankPoints('110000000'), '110');
      expect(formatTokenBankPoints('130000000'), '130');
      expect(formatTokenBankPoints('1500000'), '1.5');
      expect(formatTokenBankPoints('1'), '0.000001');
      expect(formatTokenBankPoints('0'), '0');
    });

    test('keeps int64-scale precision and falls back for bad input', () {
      expect(
        formatTokenBankPoints('9007199254740993000000'),
        '9007199254740993',
      );
      expect(formatTokenBankPoints(null), '--');
      expect(formatTokenBankPoints(''), '--');
      expect(formatTokenBankPoints('abc'), 'abc');
    });
  });

  group('toMinorUnitString', () {
    test('converts major-unit input to minor units', () {
      expect(toMinorUnitString('10'), '1000');
      expect(toMinorUnitString('10.5'), '1050');
      expect(toMinorUnitString('10.50'), '1050');
      expect(toMinorUnitString('0.01'), '1');
      expect(toMinorUnitString('0'), '0');
    });

    test('rejects invalid input', () {
      expect(toMinorUnitString(''), isNull);
      expect(toMinorUnitString('abc'), isNull);
      expect(toMinorUnitString('1.234'), isNull);
      expect(toMinorUnitString('-5'), isNull);
      expect(toMinorUnitString('1.'), isNull);
    });
  });

  group('formatAmount', () {
    test('uses the yuan symbol for CNY', () {
      expect(formatAmount('6990', currencyCode: 'CNY'), '¥69.90');
      expect(formatAmount('6990'), '¥69.90');
    });

    test('prefixes the currency code for non-CNY', () {
      expect(formatAmount('1250', currencyCode: 'USD'), 'USD 12.50');
    });

    test('falls back for missing values', () {
      expect(formatAmount(null), '--');
      expect(formatAmount('oops'), 'oops');
    });
  });
}
