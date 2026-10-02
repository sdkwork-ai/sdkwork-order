import '../logic/idempotency.dart';
import '../logic/money.dart';
import '../logic/envelope.dart';
import '../models/account_value_models.dart';
import '../transport/order_transport.dart';

/// Withdrawal (cash-out) domain service over
/// `/withdrawals/requests` (order-domain cash withdrawal; the OpenAPI
/// collection exposes POST create + GET retrieve-by-id only).
class WithdrawalService {
  WithdrawalService(this._transport);

  final OrderApiTransport _transport;

  /// `POST /withdrawals/requests` — freezes the requested cash server-side.
  /// `amount` is submitted as a minor-unit integer string derived from the
  /// major-unit form input. The key is stable within one submission attempt
  /// so transport retries dedupe instead of freezing twice.
  Future<AccountValueRequest> createWithdrawalRequest({
    required String majorAmount,
    required String payoutMethod,
    required String payoutAccountRef,
    String currencyCode = 'CNY',
    String? reasonCode,
  }) async {
    final amount = toMinorUnitString(majorAmount);
    if (amount == null || amount == '0') {
      throw OrderApiException('请输入有效的提现金额');
    }
    if (payoutMethod.isEmpty) {
      throw OrderApiException('请选择提现方式');
    }
    if (payoutAccountRef.isEmpty) {
      throw OrderApiException('请填写收款账号');
    }
    final data = await _transport.request(
      '/withdrawals/requests',
      method: 'POST',
      body: <String, dynamic>{
        'asset': 'cash',
        'amount': amount,
        'currencyCode': currencyCode,
        'payoutMethod': payoutMethod,
        'payoutAccountRef': payoutAccountRef,
        if (reasonCode != null && reasonCode.isNotEmpty)
          'reasonCode': reasonCode,
      },
      idempotencyKey: newIdempotencyKey(),
    );
    return AccountValueRequest.fromWire(resourceItem(data));
  }

  /// `GET /withdrawals/requests/{withdrawalRequestId}`.
  Future<AccountValueRequest> getWithdrawalRequest(String requestId) async {
    final data = await _transport.request('/withdrawals/requests/$requestId');
    return AccountValueRequest.fromWire(resourceItem(data));
  }
}
