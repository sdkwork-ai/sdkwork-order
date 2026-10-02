/// Typed read models for account-value requests owned by the order domain:
/// withdrawal requests (`/withdrawals/requests`) and refund requests
/// (`/orders/refund_requests`), plus the coupon redemption result. Wire
/// resources are `AccountValueRequestView`-shaped (`requestId`, `requestNo`,
/// `amount`, `status`, …); reads stay defensive and ids/money stay strings.
library;

import '../logic/envelope.dart';


/// Withdrawal / refund request view (`AccountValueRequestView`).
class AccountValueRequest {
  const AccountValueRequest({
    required this.requestId,
    required this.requestNo,
    required this.subject,
    required this.targetAsset,
    required this.amount,
    required this.currencyCode,
    required this.status,
    required this.createdAt,
    required this.updatedAt,
    this.originalOrderId,
  });

  final String requestId;
  final String requestNo;
  final String subject;
  final String targetAsset;

  /// Minor-unit amount string.
  final String amount;
  final String currencyCode;
  final String status;
  final String createdAt;
  final String updatedAt;
  final String? originalOrderId;

  static AccountValueRequest fromWire(Map<String, dynamic> wire) {
    String? optional(List<String> keys) {
      for (final key in keys) {
        final value = wire[key];
        if (value != null && value.toString().isNotEmpty) {
          return value.toString();
        }
      }
      return null;
    }

    return AccountValueRequest(
      requestId: asString(
        wire,
        const ['withdrawalRequestId', 'refundRequestId', 'requestId', 'id'],
      ),
      requestNo: asString(wire, const ['requestNo']),
      subject: asString(wire, const ['subject']),
      targetAsset: asString(wire, const ['targetAsset', 'asset']),
      amount: asString(wire, const ['amount'], fallback: '0'),
      currencyCode: asString(wire, const ['currencyCode'], fallback: 'CNY'),
      status: asString(wire, const ['status'], fallback: 'requested'),
      createdAt: asString(wire, const ['createdAt']),
      updatedAt: asString(wire, const ['updatedAt']),
      originalOrderId: optional(const ['originalOrderId']),
    );
  }
}

/// Coupon redemption result (`CouponRedemptionResult` behind `data.item`).
class CouponRedemptionResult {
  const CouponRedemptionResult({
    required this.orderId,
    required this.orderNo,
    required this.status,
    required this.replayed,
    required this.completed,
    this.benefitKind,
    this.grantAmount,
    this.durationDays,
  });

  final String orderId;
  final String orderNo;
  final String status;
  final bool replayed;

  /// True when `status` is `completed`/`succeeded`.
  final bool completed;
  final String? benefitKind;

  /// Minor-unit grant string (token bank / cash benefits).
  final String? grantAmount;
  final int? durationDays;

  static CouponRedemptionResult fromWire(Map<String, dynamic> wire) {
    final benefit = asMap(wire['benefit']);
    final rawStatus = asString(wire, const ['status', 'orderStatus'],
        fallback: 'pending');
    final completed = rawStatus == 'completed' || rawStatus == 'succeeded';
    final duration = benefit['durationDays'];
    final grant = benefit['grantAmount'] ?? benefit['grantPoints'];
    return CouponRedemptionResult(
      orderId: asString(wire, const ['orderId', 'id']),
      orderNo: asString(wire, const ['orderNo']),
      status: rawStatus,
      replayed: asBool(wire, const ['replayed']),
      completed: completed,
      benefitKind: benefit['kind']?.toString(),
      grantAmount: grant?.toString(),
      durationDays: duration is num ? duration.toInt() : int.tryParse('$duration'),
    );
  }
}
