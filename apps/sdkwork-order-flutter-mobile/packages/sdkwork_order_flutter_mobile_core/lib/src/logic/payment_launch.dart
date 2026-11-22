/// Provider launch planning for the cashier, mirroring the server contract.
///
/// The backend already normalizes every channel into a `nextAction` plus a
/// payload (`jsapiPayload` / `appPayload` / `orderStr` / `clientSecret` /
/// `payUrl` / `qrCodeUrl`). The client must act on that plan instead of
/// re-deriving per-provider branches: the Flutter cashier created a real payment,
/// showed nothing and polled until it expired because no surface consumed these
/// keys at all.
library;

/// What the payer has to do next.
enum PaymentLaunchKind {
  /// Invoke the WeChat in-app SDK (`wx.requestPayment` on MP, PayReq on native).
  jsapi,

  /// Invoke the WeChat App SDK with the PayReq parameter set.
  wechatApp,

  /// Invoke the Alipay App SDK with the signed order string.
  alipayApp,

  /// Confirm a Stripe PaymentIntent with Stripe.js / the native Stripe SDK.
  stripeConfirm,

  /// Navigate to a hosted cashier page.
  redirect,

  /// Render the returned payload as a scannable QR code.
  qr,

  /// Nothing launchable: fall back to the authenticated cashier.
  cashier,
}

/// A resolved launch instruction.
class PaymentLaunchPlan {
  const PaymentLaunchPlan(this.kind, this.payload);

  final PaymentLaunchKind kind;

  /// URL, JSON parameter set or QR payload, depending on [kind].
  final String? payload;

  /// True when completing the payment needs the provider's own app on the
  /// device, so the cashier must say so instead of silently waiting.
  bool get requiresProviderApp =>
      kind == PaymentLaunchKind.jsapi ||
      kind == PaymentLaunchKind.wechatApp ||
      kind == PaymentLaunchKind.alipayApp;

  /// True when the plan is a navigation the client can open itself.
  bool get isNavigation => kind == PaymentLaunchKind.redirect;

  /// True when the payload should be drawn as a QR code.
  bool get isScannable => kind == PaymentLaunchKind.qr;
}

String? _read(Map<String, String> params, String key) {
  final value = params[key]?.trim();
  return value == null || value.isEmpty ? null : value;
}

/// Resolves what the payer must do next from the payment session parameters.
///
/// `nextAction` is authoritative when present (it is the server's normalized
/// decision); the payload keys are the fallback for older responses, in the same
/// precedence the server uses: in-app SDK > signed App string > client secret >
/// redirect page > scan QR > nothing.
PaymentLaunchPlan resolvePaymentLaunchPlan(Map<String, String> paymentParams) {
  final action = _read(paymentParams, 'nextAction')?.toLowerCase();
  switch (action) {
    case 'jsapi':
      return PaymentLaunchPlan(
        PaymentLaunchKind.jsapi,
        _read(paymentParams, 'jsapiPayload'),
      );
    case 'app':
      final alipayOrder = _read(paymentParams, 'orderStr');
      if (alipayOrder != null) {
        return PaymentLaunchPlan(PaymentLaunchKind.alipayApp, alipayOrder);
      }
      return PaymentLaunchPlan(
        PaymentLaunchKind.wechatApp,
        _read(paymentParams, 'appPayload'),
      );
    case 'stripe_confirm':
      return PaymentLaunchPlan(
        PaymentLaunchKind.stripeConfirm,
        _read(paymentParams, 'clientSecret'),
      );
    case 'redirect':
      return PaymentLaunchPlan(
        PaymentLaunchKind.redirect,
        _read(paymentParams, 'payUrl'),
      );
    case 'qr_code':
      return PaymentLaunchPlan(
        PaymentLaunchKind.qr,
        _read(paymentParams, 'qrCodeUrl'),
      );
    case 'cashier':
      return PaymentLaunchPlan(
        PaymentLaunchKind.cashier,
        _read(paymentParams, 'cashierUrl'),
      );
  }

  // No (or an unknown) action: fall back to the payload precedence.
  final orderStr = _read(paymentParams, 'orderStr');
  if (orderStr != null) {
    return PaymentLaunchPlan(PaymentLaunchKind.alipayApp, orderStr);
  }
  final jsapi = _read(paymentParams, 'jsapiPayload');
  if (jsapi != null) {
    return PaymentLaunchPlan(PaymentLaunchKind.jsapi, jsapi);
  }
  final appPayload = _read(paymentParams, 'appPayload');
  if (appPayload != null) {
    return PaymentLaunchPlan(PaymentLaunchKind.wechatApp, appPayload);
  }
  final clientSecret = _read(paymentParams, 'clientSecret');
  if (clientSecret != null) {
    return PaymentLaunchPlan(PaymentLaunchKind.stripeConfirm, clientSecret);
  }
  final payUrl = _read(paymentParams, 'payUrl');
  if (payUrl != null) {
    return PaymentLaunchPlan(PaymentLaunchKind.redirect, payUrl);
  }
  final qr = _read(paymentParams, 'qrCodeUrl');
  if (qr != null) {
    return PaymentLaunchPlan(PaymentLaunchKind.qr, qr);
  }
  return PaymentLaunchPlan(
    PaymentLaunchKind.cashier,
    _read(paymentParams, 'cashierUrl'),
  );
}

/// User-facing instruction for a launch plan.
///
/// The native provider SDKs are not linked into this app yet, so the cashier must
/// at least tell the payer what the session expects instead of appearing to wait:
/// the payer can move to the provider's app, open the hosted page, or scan the
/// payload. Kept here (pure) so the wording is unit-tested alongside the plan.
String describePaymentLaunch(PaymentLaunchPlan plan) {
  switch (plan.kind) {
    case PaymentLaunchKind.jsapi:
      return '已生成微信支付参数，请在微信中完成支付。';
    case PaymentLaunchKind.wechatApp:
    case PaymentLaunchKind.alipayApp:
      return '请在微信或支付宝 App 中完成支付。';
    case PaymentLaunchKind.stripeConfirm:
      return '请完成银行卡验证以支付该订单。';
    case PaymentLaunchKind.redirect:
      return plan.payload == null
          ? '请继续前往收银台完成支付。'
          : '请在浏览器中打开支付页完成支付。';
    case PaymentLaunchKind.qr:
      return plan.payload == null
          ? '请使用手机扫码完成支付。'
          : '请使用手机扫描支付二维码完成支付。';
    case PaymentLaunchKind.cashier:
      return '支付尚未开始，请返回收银台重新选择支付方式。';
  }
}