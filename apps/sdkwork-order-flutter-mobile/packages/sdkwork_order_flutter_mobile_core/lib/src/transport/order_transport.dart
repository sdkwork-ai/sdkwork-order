/// The SINGLE transport seam of the Flutter mobile root.
///
/// The generated Dart SDK family for the order authority has not been
/// produced yet (root `docs/decisions.md`, `sdks/README.md`), so this file
/// owns the whole order app-api wire: `{code, data, traceId}` envelope
/// unwrap (`data.item` / `data.items + pageInfo`), HTTP 4xx/5xx
/// `application/problem+json` (`ProblemDetail`) error mapping, bearer auth,
/// `Idempotency-Key` headers, and bounded timeouts over `dart:io HttpClient`
/// — zero pub.dev network dependencies. When the Dart SDK family lands,
/// services swap this seam for the generated client; screens never change.
///
/// This is the only file in the entire app root allowed to import `dart:io`
/// for networking.
library;

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import '../session/app_session.dart';

const Duration orderTransportConnectTimeout = Duration(seconds: 15);
const Duration orderTransportResponseTimeout = Duration(seconds: 15);

/// Transport port consumed by domain services. Fakes implement this for
/// service/widget tests; [HttpOrderApiTransport] is the production wire.
abstract class OrderApiTransport {
  /// Performs one order app-api call and returns the unwrapped envelope
  /// `data` payload (`data` itself, not `data.item`).
  Future<Map<String, dynamic>> request(
    String path, {
    String method = 'GET',
    Map<String, dynamic>? body,
    Map<String, String>? query,
    String? idempotencyKey,
  });
}

/// `dart:io HttpClient` production implementation of [OrderApiTransport].
class HttpOrderApiTransport implements OrderApiTransport {
  HttpOrderApiTransport({
    required this.appApiBaseUrl,
    String? Function()? tokenProvider,
    this.connectTimeout = orderTransportConnectTimeout,
    this.responseTimeout = orderTransportResponseTimeout,
    this.httpClientFactory,
  }) : _tokenProvider = tokenProvider ?? _defaultTokenProvider;

  /// Order app-api origin plus mount path, e.g.
  /// `https://api-dev.sdkwork.com/app/v3/api`.
  final String appApiBaseUrl;

  final String? Function() _tokenProvider;
  final Duration connectTimeout;
  final Duration responseTimeout;

  /// Optional `HttpClient` factory override (tests inject custom clients).
  final HttpClient Function()? httpClientFactory;

  HttpClient? _client;

  HttpClient get _http => _client ??= httpClientFactory?.call() ??
      (HttpClient()..connectionTimeout = connectTimeout);

  static String _defaultTokenProvider() => AppSession.instance.token;

  @override
  Future<Map<String, dynamic>> request(
    String path, {
    String method = 'GET',
    Map<String, dynamic>? body,
    Map<String, String>? query,
    String? idempotencyKey,
  }) async {
    final url = _buildUrl(path, query);
    final request = await _http.openUrl(method, Uri.parse(url));
    request.headers.contentType = ContentType.json;
    final token = _tokenProvider() ?? '';
    if (token.isNotEmpty) {
      request.headers.set(HttpHeaders.authorizationHeader, 'Bearer $token');
    }
    if (idempotencyKey != null && idempotencyKey.isNotEmpty) {
      request.headers.set('Idempotency-Key', idempotencyKey);
    }
    if (body != null) {
      request.write(jsonEncode(body));
    }

    final response = await request.close().timeout(responseTimeout);
    final text = await response
        .transform(utf8.decoder)
        .join()
        .timeout(responseTimeout);
    final decoded = text.isEmpty
        ? <String, dynamic>{}
        : jsonDecode(text) as Map<String, dynamic>;

    if (response.statusCode >= 200 && response.statusCode < 300) {
      return _unwrapSuccess(decoded);
    }
    throw orderApiExceptionFromProblem(response.statusCode, decoded);
  }

  String _buildUrl(String path, Map<String, String>? query) {
    var url = '${appApiBaseUrl.replaceAll(RegExp(r'/+$'), '')}'
        '$path';
    if (query != null) {
      final params = <String>[];
      query.forEach((key, value) {
        if (value.isNotEmpty) {
          params.add(
            '${Uri.encodeQueryComponent(key)}='
            '${Uri.encodeQueryComponent(value)}',
          );
        }
      });
      if (params.isNotEmpty) {
        url = '$url?${params.join('&')}';
      }
    }
    return url;
  }

  /// Unwraps `{code: 0, data}`; responses without an envelope pass through
  /// unchanged (vendor compatibility only — order app-api always envelopes).
  Map<String, dynamic> _unwrapSuccess(Map<String, dynamic> decoded) {
    if (!decoded.containsKey('code')) {
      return decoded;
    }
    if (num.tryParse('${decoded['code']}') == 0) {
      final data = decoded['data'];
      return data is Map<String, dynamic> ? data : <String, dynamic>{};
    }
    // 2xx with a non-zero code is a legacy envelope violation; surface it as
    // an API failure instead of silently treating it as success.
    throw OrderApiException(
      '${decoded['message'] ?? decoded['detail'] ?? '请求失败'}',
      code: decoded['code']?.toString(),
      traceId: decoded['traceId']?.toString(),
    );
  }
}

/// Builds the typed API failure from a ProblemDetail body
/// (`application/problem+json`, numeric `code`, `traceId`).
OrderApiException orderApiExceptionFromProblem(
  int statusCode,
  Map<String, dynamic> problem,
) {
  return OrderApiException(
    (problem['detail'] ??
            problem['title'] ??
            problem['message'] ??
            '请求失败（HTTP $statusCode）')
        .toString(),
    code: (problem['code'] ?? statusCode).toString(),
    traceId: problem['traceId']?.toString(),
    statusCode: statusCode,
  );
}

/// Typed order app-api failure carrying the numeric ProblemDetail `code`,
/// the server `traceId`, and the HTTP status when the failure came from the
/// status line.
class OrderApiException implements Exception {
  OrderApiException(
    this.message, {
    this.code,
    this.traceId,
    this.statusCode,
  });

  final String message;
  final String? code;
  final String? traceId;
  final int? statusCode;

  @override
  String toString() => message;
}
