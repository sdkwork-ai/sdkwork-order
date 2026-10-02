import '../logic/idempotency.dart';
import '../logic/money.dart';
import '../logic/envelope.dart';
import '../models/recharge_models.dart';
import '../transport/order_transport.dart';

/// Recharge (Token Bank) domain service: direct plans, packages, recharge
/// orders, and recharge-order cancel. Write paths carry `Idempotency-Key`.
class RechargeService {
  RechargeService(this._transport);

  final OrderApiTransport _transport;

  /// `GET /recharges/plans` — Token Bank direct recharge plans.
  Future<List<RechargePlan>> listPlans({
    String? status,
    int page = 1,
    int pageSize = 50,
  }) async {
    final data = await _transport.request(
      '/recharges/plans',
      query: {
        'page': '$page',
        'page_size': '$pageSize',
        if (status != null && status.isNotEmpty) 'status': status,
      },
    );
    return listItems(data).map(RechargePlan.fromWire).toList();
  }

  /// `GET /recharges/packages` — prepaid recharge packages.
  Future<List<RechargePackage>> listPackages({
    int page = 1,
    int pageSize = 50,
  }) async {
    final data = await _transport.request(
      '/recharges/packages',
      query: {'page': '$page', 'page_size': '$pageSize'},
    );
    return listItems(data).map(RechargePackage.fromWire).toList();
  }

  /// `GET /recharges/orders`.
  Future<List<RechargeOrder>> listRechargeOrders({
    String? subject,
    String? status,
    int page = 1,
    int pageSize = 20,
  }) async {
    final data = await _transport.request(
      '/recharges/orders',
      query: {
        'page': '$page',
        'page_size': '$pageSize',
        if (subject != null && subject.isNotEmpty) 'subject': subject,
        if (status != null && status.isNotEmpty) 'status': status,
      },
    );
    return listItems(data).map(RechargeOrder.fromWire).toList();
  }

  /// `POST /recharges/orders` — places a Token Bank recharge order for a
  /// direct plan. The catalog `priceAmount` is a major-unit decimal
  /// ("10.00"); the command takes minor-unit integers ("1000"), converted
  /// here (same rule as the H5 subscription service). The recharge order id
  /// feeds the shared cashier (`POST /orders/{orderId}/payments`).
  Future<RechargeOrder> createPlanRechargeOrder(
    RechargePlan plan, {
    String paymentMethod = 'wechat_pay',
  }) async {
    final amountMinor = planToMinorAmount(plan);
    if (amountMinor == null) {
      throw OrderApiException('所选充值计划价格无效：${plan.priceAmount}');
    }
    final data = await _transport.request(
      '/recharges/orders',
      method: 'POST',
      body: <String, dynamic>{
        'subject': 'token_bank_recharge',
        'targetAsset': 'token_bank',
        'planCode': plan.planCode,
        if (plan.planPeriod.isNotEmpty) 'planPeriod': plan.planPeriod,
        'amount': amountMinor,
        'currencyCode': plan.currencyCode,
        'grantAmount': plan.grantAmount,
        'paymentMethod': paymentMethod,
        'paymentProduct': 'mobile_cashier_h5',
        'source': 'flutter-token-bank',
      },
      idempotencyKey: newIdempotencyKey(),
    );
    return RechargeOrder.fromWire(resourceItem(data));
  }

  /// `POST /recharges/orders` for a prepaid package (`account_recharge_package`).
  Future<RechargeOrder> createPackageRechargeOrder(
    RechargePackage package, {
    String paymentMethod = 'wechat_pay',
  }) async {
    if (package.priceAmount.isEmpty || BigInt.tryParse(package.priceAmount) == null) {
      throw OrderApiException('所选充值套餐价格无效：${package.priceAmount}');
    }
    final data = await _transport.request(
      '/recharges/orders',
      method: 'POST',
      body: <String, dynamic>{
        'subject': 'account_recharge_package',
        'targetAsset': 'token_bank',
        'packageId': package.id,
        'amount': package.priceAmount,
        'currencyCode': package.currencyCode,
        'grantAmount': '${package.grantAmount}',
        'paymentMethod': paymentMethod,
        'paymentProduct': 'mobile_cashier_h5',
        'source': 'flutter-token-bank',
      },
      idempotencyKey: newIdempotencyKey(),
    );
    return RechargeOrder.fromWire(resourceItem(data));
  }

  /// `GET /recharges/orders/{orderId}` — recharge status polling for
  /// non-cashier flows.
  Future<RechargeOrder> getRechargeOrder(String orderId) async {
    final data = await _transport.request('/recharges/orders/$orderId');
    return RechargeOrder.fromWire(resourceItem(data));
  }

  /// `POST /recharges/orders/{orderId}/cancel`.
  Future<void> cancelRechargeOrder(String orderId) async {
    await _transport.request(
      '/recharges/orders/$orderId/cancel',
      method: 'POST',
      body: const <String, dynamic>{},
      idempotencyKey: newIdempotencyKey(),
    );
  }

  /// Converts a plan catalog price (major-unit decimal) into the minor-unit
  /// integer string the command takes; null when invalid or non-positive.
  static String? planToMinorAmount(RechargePlan plan) {
    final minor = toMinorUnitString(plan.priceAmount);
    if (minor == null || minor == '0') {
      return null;
    }
    return minor;
  }
}
