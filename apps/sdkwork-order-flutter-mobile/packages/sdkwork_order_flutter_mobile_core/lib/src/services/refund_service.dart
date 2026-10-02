import '../logic/idempotency.dart';
import '../logic/money.dart';
import '../logic/envelope.dart';
import '../models/account_value_models.dart';
import '../transport/order_transport.dart';

/// Refund-request domain service over `/orders/refund_requests`
/// (`GET` list + `POST` create + `GET` retrieve).
class RefundService {
  RefundService(this._transport);

  final OrderApiTransport _transport;

  /// `GET /orders/refund_requests`.
  Future<List<AccountValueRequest>> listRefundRequests({
    String? status,
    int page = 1,
    int pageSize = 20,
  }) async {
    final data = await _transport.request(
      '/orders/refund_requests',
      query: {
        'page': '$page',
        'page_size': '$pageSize',
        if (status != null && status.isNotEmpty) 'status': status,
      },
    );
    return listItems(data).map(AccountValueRequest.fromWire).toList();
  }

  /// `POST /orders/refund_requests` — [majorAmount] is the user-entered
  /// amount. `cash` refunds are money: converted to the minor-unit integer
  /// string (¥10.50 → "1050"). `points`/`token_bank` refunds are asset
  /// units: validated as positive integers and submitted verbatim (the
  /// server normalizes decimals to minor units and passes integers through).
  /// [targetAsset] is one of `points` / `token_bank` / `cash`
  /// (OpenAPI `RefundRequestCreateCommand.targetAsset` enum).
  Future<AccountValueRequest> createRefundRequest({
    required String originalOrderId,
    required String targetAsset,
    required String amount,
    String currencyCode = 'CNY',
    String? reasonCode,
    String? reasonDetail,
  }) async {
    if (originalOrderId.isEmpty) {
      throw OrderApiException('缺少原始订单');
    }
    const allowedAssets = <String>['points', 'token_bank', 'cash'];
    if (!allowedAssets.contains(targetAsset)) {
      throw OrderApiException('不支持的退款资产：$targetAsset');
    }
    final trimmed = amount.trim();
    final wireAmount = targetAsset == 'cash'
        ? toMinorUnitString(trimmed)
        : RegExp(r'^\d+$').stringMatch(trimmed);
    if (wireAmount == null || wireAmount == '0') {
      throw OrderApiException(
        targetAsset == 'cash' ? '请输入有效的退款金额' : '请输入有效的退款额度（正整数）',
      );
    }
    final data = await _transport.request(
      '/orders/refund_requests',
      method: 'POST',
      body: <String, dynamic>{
        'originalOrderId': originalOrderId,
        'targetAsset': targetAsset,
        'amount': wireAmount,
        'currencyCode': currencyCode,
        if (reasonCode != null && reasonCode.isNotEmpty)
          'reasonCode': reasonCode,
        if (reasonDetail != null && reasonDetail.isNotEmpty)
          'reasonDetail': reasonDetail,
      },
      idempotencyKey: newIdempotencyKey(),
    );
    return AccountValueRequest.fromWire(resourceItem(data));
  }

  /// `GET /orders/refund_requests/{refundRequestId}`.
  Future<AccountValueRequest> getRefundRequest(String requestId) async {
    final data = await _transport.request('/orders/refund_requests/$requestId');
    return AccountValueRequest.fromWire(resourceItem(data));
  }
}
