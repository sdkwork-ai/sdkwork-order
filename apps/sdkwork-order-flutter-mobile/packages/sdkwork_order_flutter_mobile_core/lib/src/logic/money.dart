/// Minor-unit money helpers (pure).
///
/// All commerce amounts travel as minor-unit integer strings
/// (`"6990"` = ¥69.90). Formatting uses BigInt arithmetic — amounts are
/// int64-scale wire strings and must never pass through `double`.
library;

const String defaultCurrencySymbol = '¥';

/// Formats a minor-unit integer string as a display amount.
///
/// Returns `'--'` for null/empty, and the raw input for malformed values so
/// backend data stays visible instead of silently disappearing.
String formatMinorAmount(String? minor, {String symbol = defaultCurrencySymbol}) {
  final value = minor?.trim() ?? '';
  if (value.isEmpty) {
    return '--';
  }
  final parsed = BigInt.tryParse(value);
  if (parsed == null) {
    return value;
  }
  final negative = parsed.isNegative;
  final abs = parsed.abs();
  final cents = BigInt.from(100);
  final yuan = abs ~/ cents;
  final fraction = (abs % cents).toString().padLeft(2, '0');
  final body = '$yuan.$fraction';
  return negative ? '-$symbol$body' : '$symbol$body';
}

/// Converts a user-entered major-unit amount (`"10"`, `"10.5"`, `"10.50"`)
/// into a minor-unit integer string (`"1000"`, `"1050"`, `"1050"`).
///
/// Returns null when the input is not a positive decimal with at most two
/// fraction digits. Zero passes validation as `"0"`; callers reject it.
String? toMinorUnitString(String major) {
  final value = major.trim();
  if (value.isEmpty) {
    return null;
  }
  final match = RegExp(r'^(\d+)(?:\.(\d{1,2}))?$').firstMatch(value);
  if (match == null) {
    return null;
  }
  final yuan = match.group(1)!;
  final cents = (match.group(2) ?? '').padRight(2, '0');
  final combined = '$yuan$cents'.replaceFirst(RegExp(r'^0+(?=\d)'), '');
  return combined;
}

/// Converts a minor-unit integer string back into a major-unit display
/// string with two decimals (`"6990"` → `"69.90"`). Returns null for
/// malformed input.
String? minorToMajorString(String? minor) {
  final value = minor?.trim() ?? '';
  if (value.isEmpty) {
    return null;
  }
  final parsed = BigInt.tryParse(value);
  if (parsed == null) {
    return null;
  }
  final negative = parsed.isNegative;
  final abs = parsed.abs();
  final cents = BigInt.from(100);
  final body = '${abs ~/ cents}.${(abs % cents).toString().padLeft(2, '0')}';
  return negative ? '-$body' : body;
}

/// Maps a minor-unit amount plus currency onto a display string. Only CNY
/// carries the `¥` symbol today; unknown codes fall back to `${code} ${minor}`.
String formatAmount(String? minor, {String? currencyCode}) {
  final code = (currencyCode ?? 'CNY').toUpperCase();
  if (code == 'CNY') {
    return formatMinorAmount(minor);
  }
  final value = minor?.trim() ?? '';
  if (value.isEmpty) {
    return '--';
  }
  final parsed = BigInt.tryParse(value);
  if (parsed == null) {
    return value;
  }
  return formatMinorAmount(minor, symbol: '$code ');
}
