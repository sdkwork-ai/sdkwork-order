/// Typed accessors for the SDKWork response envelope and loose wire payloads
/// (`API_SPEC.md` §4.5/§16).
///
/// Success: `{code: 0, data, traceId}` where list operations carry
/// `data.items + data.pageInfo` and single-resource operations carry
/// `data.item`. The transport unwraps to `data`; these helpers unwrap the
/// `item`/`items+pageInfo` layer and defensively read loose payload fields
/// (int64 ids and money stay strings — never converted to num).
library;

Map<String, dynamic> asMap(Object? value) =>
    value is Map<String, dynamic> ? value : <String, dynamic>{};

List<Map<String, dynamic>> asList(Object? value) => value is List
    ? value.whereType<Map<String, dynamic>>().toList()
    : <Map<String, dynamic>>[];

String asString(
  Map<String, dynamic> map,
  List<String> keys, {
  String fallback = '',
}) {
  for (final key in keys) {
    final value = map[key];
    if (value != null && value.toString().isNotEmpty) {
      return value.toString();
    }
  }
  return fallback;
}

int asInt(Map<String, dynamic> map, List<String> keys, {int fallback = 0}) {
  for (final key in keys) {
    final value = map[key];
    if (value is num && value.toInt() == value) {
      return value.toInt();
    }
    if (value is String) {
      final parsed = int.tryParse(value);
      if (parsed != null) {
        return parsed;
      }
    }
  }
  return fallback;
}

bool asBool(Map<String, dynamic> map, List<String> keys, {bool fallback = false}) {
  for (final key in keys) {
    final value = map[key];
    if (value is bool) {
      return value;
    }
  }
  return fallback;
}

/// Unwraps `data.item` (single-resource reads). Tolerates an
/// already-unwrapped payload for forward compatibility with generated-SDK
/// shapes, mirroring the TS `unwrapSdkworkOrderResource` behavior.
Map<String, dynamic> resourceItem(Map<String, dynamic> data) {
  final item = data['item'];
  if (item is Map<String, dynamic>) {
    return item;
  }
  if (data.containsKey('item')) {
    return <String, dynamic>{};
  }
  return data;
}

/// Unwraps `data.items` (list reads).
List<Map<String, dynamic>> listItems(Map<String, dynamic> data) =>
    asList(data['items']);

/// Unwraps `data.pageInfo` with the offset defaults applied.
PageInfo readPageInfo(Map<String, dynamic> data) {
  final raw = asMap(data['pageInfo']);
  return PageInfo(
    mode: asString(raw, const ['mode'], fallback: 'offset'),
    page: asInt(raw, const ['page'], fallback: 1),
    pageSize: asInt(raw, const ['pageSize'], fallback: 20),
    totalItems: asString(raw, const ['totalItems'], fallback: '0'),
    hasMore: asBool(raw, const ['hasMore']),
  );
}

/// One page descriptor from `data.pageInfo` (`totalItems` stays a string —
/// int64 wire contract).
class PageInfo {
  const PageInfo({
    required this.mode,
    required this.page,
    required this.pageSize,
    required this.totalItems,
    required this.hasMore,
  });

  final String mode;
  final int page;
  final int pageSize;
  final String totalItems;
  final bool hasMore;
}
