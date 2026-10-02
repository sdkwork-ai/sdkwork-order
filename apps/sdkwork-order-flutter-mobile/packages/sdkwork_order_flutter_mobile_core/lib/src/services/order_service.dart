import '../logic/cashier_logic.dart';
import '../logic/envelope.dart';
import '../logic/idempotency.dart';
import '../models/account_value_models.dart';
import '../models/order_models.dart';
import '../transport/order_transport.dart';

/// Order-center domain service: list/detail/events/status/statistics, the
/// cashier write path (payments, payment-success terminal check),
/// cancellation, receipt confirmation, and coupon redemption. Calls go
/// through the injected [OrderApiTransport] seam only.
class OrderService {
  OrderService(this._transport);

  final OrderApiTransport _transport;

  static const orderPaymentMethods = <String>['wechat_pay', 'alipay', 'balance'];

  static const methodLabels = <String, String>{
    'wechat_pay': '微信支付',
    'alipay': '支付宝',
    'balance': '余额',
  };

  /// `GET /orders` — one server page (`status` omitted for the 全部 tab,
  /// matching the mobile-react `toOrderListStatusWire`).
  Future<OrderPage> listOrders({
    String? status,
    int page = 1,
    int pageSize = 20,
  }) async {
    final data = await _transport.request(
      '/orders',
      query: {
        'page': '$page',
        'page_size': '$pageSize',
        if (status != null && status.isNotEmpty && status != 'all')
          'status': status,
      },
    );
    return OrderPage(
      items: listItems(data).map(Order.fromWire).toList(),
      pageInfo: readPageInfo(data),
    );
  }

  /// `GET /orders/statistics`.
  Future<OrderStatistics> getStatistics() async {
    final data = await _transport.request('/orders/statistics');
    return OrderStatistics.fromWire(resourceItem(data));
  }

  /// `GET /orders/{orderId}`.
  Future<Order> getOrder(String orderId) async {
    final data = await _transport.request('/orders/$orderId');
    return Order.fromWire(resourceItem(data));
  }

  /// `GET /orders/{orderId}/events` — lifecycle timeline.
  Future<List<OrderEvent>> getOrderEvents(
    String orderId, {
    int page = 1,
    int pageSize = 50,
  }) async {
    final data = await _transport.request(
      '/orders/$orderId/events',
      query: {'page': '$page', 'page_size': '$pageSize'},
    );
    return listItems(data).map(OrderEvent.fromWire).toList();
  }

  /// `GET /orders/{orderId}/status`.
  Future<PaymentStatus> getOrderStatus(String orderId) async {
    final data = await _transport.request('/orders/$orderId/status');
    return PaymentStatus.fromWire(resourceItem(data));
  }

  /// `GET /orders/{orderId}/payment_success` — the authoritative terminal
  /// check behind the cashier's expiry final probe.
  Future<PaymentStatus> getPaymentSuccess(String orderId) async {
    final data = await _transport.request('/orders/$orderId/payment_success');
    return PaymentStatus.fromWire(resourceItem(data));
  }

  /// `POST /orders/{orderId}/payments` — creates the payment session with an
  /// `Idempotency-Key` scoped to one cashier attempt.
  Future<PaymentSession> createPayment(
    String orderId,
    String paymentMethod,
  ) async {
    if (!orderPaymentMethods.contains(paymentMethod)) {
      throw OrderApiException('不支持的支付方式：$paymentMethod');
    }
    final data = await _transport.request(
      '/orders/$orderId/payments',
      method: 'POST',
      body: <String, dynamic>{'paymentMethod': paymentMethod},
      idempotencyKey: newIdempotencyKey(),
    );
    return PaymentSession.fromWire(resourceItem(data));
  }

  /// `POST /orders/{orderId}/cancellations` — empty command body keeps the
  /// request well-formed while leaving `cancelReason`/`cancelType` unset
  /// (same rationale as the mobile-react service comment).
  Future<void> cancelOrder(String orderId, {String? cancelReason}) async {
    await _transport.request(
      '/orders/$orderId/cancellations',
      method: 'POST',
      body: <String, dynamic>{
        if (cancelReason != null && cancelReason.isNotEmpty)
          'cancelReason': cancelReason,
      },
      idempotencyKey: newIdempotencyKey(),
    );
  }

  /// `POST /orders/{orderId}/receipt_confirmations` (201, no body) — buyer
  /// confirms receipt of a fulfilled order. Empty JSON body `{}` keeps the
  /// command well-formed while carrying no fields (same rationale as
  /// [cancelOrder] and the mini-program `confirmReceipt` precedent); the
  /// fresh `Idempotency-Key` dedupes transport-level retries of one attempt.
  Future<void> confirmReceipt(String orderId) async {
    await _transport.request(
      '/orders/$orderId/receipt_confirmations',
      method: 'POST',
      body: <String, dynamic>{},
      idempotencyKey: newIdempotencyKey(),
    );
  }

  /// `POST /orders/coupon_redemptions` — code trimmed + uppercased like the
  /// mobile-react `redeemVoucher`.
  Future<CouponRedemptionResult> redeemCoupon(String code) async {
    final normalized = code.trim().toUpperCase();
    if (normalized.isEmpty) {
      throw OrderApiException('请输入券码');
    }
    final data = await _transport.request(
      '/orders/coupon_redemptions',
      method: 'POST',
      body: <String, dynamic>{'couponCode': normalized},
      idempotencyKey: newIdempotencyKey(),
    );
    return CouponRedemptionResult.fromWire(resourceItem(data));
  }

  /// Resolves the cashier phase from the authoritative payment-success check
  /// plus the order status (pure helper re-exported for screens).
  static CashierPhase resolvePhase(PaymentStatus status, String orderStatus) =>
      resolveCashierPhase(paid: status.paid, orderStatus: orderStatus);
}
