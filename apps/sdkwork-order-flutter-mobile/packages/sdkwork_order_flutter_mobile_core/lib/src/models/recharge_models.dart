/// Typed read models for the recharge (Token Bank) domain. Wire shapes:
/// `TokenBankPlanResponse` (`GET /recharges/plans`), recharge package items
/// (`GET /recharges/packages`), and recharge order resources
/// (`GET|POST /recharges/orders`). Amounts stay minor-unit strings.
library;

import '../logic/envelope.dart';


/// Token Bank direct recharge plan (`TokenBankPlanResponse`). `priceAmount`
/// is a major-unit decimal string ("10.00") on this resource; convert to
/// minor units when placing the recharge order (see `RechargeService`).
class RechargePlan {
  const RechargePlan({
    required this.planCode,
    required this.displayName,
    required this.planPeriod,
    required this.grantAmount,
    required this.bonusAmount,
    required this.priceAmount,
    required this.currencyCode,
    required this.status,
    this.renewalPolicy,
  });

  final String planCode;
  final String displayName;
  final String planPeriod;

  /// Minor-unit grant string.
  final String grantAmount;

  /// Minor-unit bonus string.
  final String bonusAmount;

  /// Major-unit decimal string (catalog display price).
  final String priceAmount;
  final String currencyCode;
  final String status;
  final String? renewalPolicy;

  static RechargePlan fromWire(Map<String, dynamic> wire) => RechargePlan(
        planCode: asString(wire, const ['planCode']),
        displayName: asString(wire, const ['displayName'], fallback: '充值计划'),
        planPeriod: asString(wire, const ['planPeriod']),
        grantAmount: asString(wire, const ['grantAmount'], fallback: '0'),
        bonusAmount: asString(wire, const ['bonusAmount'], fallback: '0'),
        priceAmount: asString(wire, const ['priceAmount'], fallback: '0'),
        currencyCode: asString(wire, const ['currencyCode'], fallback: 'CNY'),
        status: asString(wire, const ['status'], fallback: 'active'),
        renewalPolicy: _optional(wire, const ['renewalPolicy']),
      );

  static String? _optional(Map<String, dynamic> wire, List<String> keys) {
    for (final key in keys) {
      final value = wire[key];
      if (value != null && value.toString().isNotEmpty) {
        return value.toString();
      }
    }
    return null;
  }
}

/// Recharge package (`GET /recharges/packages` items).
class RechargePackage {
  const RechargePackage({
    required this.id,
    required this.priceAmount,
    required this.currencyCode,
    required this.bonusPoints,
    required this.grantAmount,
    required this.points,
  });

  final String id;

  /// Minor-unit price string.
  final String priceAmount;
  final String currencyCode;
  final int bonusPoints;
  final int grantAmount;
  final int points;

  static RechargePackage fromWire(Map<String, dynamic> wire) =>
      RechargePackage(
        id: asString(wire, const ['id']),
        priceAmount: asString(wire, const ['priceAmount'], fallback: '0'),
        currencyCode: asString(wire, const ['currencyCode'], fallback: 'CNY'),
        bonusPoints: asInt(wire, const ['bonusPoints']),
        grantAmount: asInt(wire, const ['grantAmount']),
        points: asInt(wire, const ['points']),
      );
}

/// Recharge order (`GET|POST /recharges/orders` item). Carries the
/// order-bound cashier fields the H5 consumers read.
class RechargeOrder {
  const RechargeOrder({
    required this.orderId,
    required this.orderNo,
    required this.status,
    required this.amount,
    required this.currencyCode,
    this.cashierUrl,
    this.qrCode,
    this.expiresAt,
    this.success,
  });

  final String orderId;
  final String orderNo;
  final String status;

  /// Minor-unit amount string.
  final String amount;
  final String currencyCode;
  final String? cashierUrl;
  final String? qrCode;
  final String? expiresAt;
  final bool? success;

  static RechargeOrder fromWire(Map<String, dynamic> wire) {
    String? optional(List<String> keys) {
      for (final key in keys) {
        final value = wire[key];
        if (value != null && value.toString().isNotEmpty) {
          return value.toString();
        }
      }
      return null;
    }

    return RechargeOrder(
      orderId: asString(wire, const ['orderId', 'id']),
      orderNo: asString(wire, const ['orderNo']),
      status: asString(wire, const ['status', 'orderStatus'],
          fallback: 'pending'),
      amount: asString(wire, const ['amount'], fallback: '0'),
      currencyCode: asString(wire, const ['currencyCode'], fallback: 'CNY'),
      cashierUrl: optional(const ['cashierUrl']),
      qrCode: optional(const ['qrCode', 'qrCodePayload']),
      expiresAt: optional(const ['expiresAt', 'expireTime']),
      success: wire['success'] is bool ? wire['success'] as bool : null,
    );
  }
}
