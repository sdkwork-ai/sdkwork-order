import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('nextAction is authoritative and maps to the matching launch kind', () {
    expect(
      resolvePaymentLaunchPlan({'nextAction': 'jsapi', 'jsapiPayload': '{"a":1}'}).kind,
      PaymentLaunchKind.jsapi,
    );
    expect(
      resolvePaymentLaunchPlan({'nextAction': 'app', 'orderStr': 'app_id=1&sign=2'}).kind,
      PaymentLaunchKind.alipayApp,
    );
    expect(
      resolvePaymentLaunchPlan({'nextAction': 'app', 'appPayload': '{"appid":"wx"}'}).kind,
      PaymentLaunchKind.wechatApp,
    );
    expect(
      resolvePaymentLaunchPlan({'nextAction': 'stripe_confirm', 'clientSecret': 'pi_1_secret'}).kind,
      PaymentLaunchKind.stripeConfirm,
    );
    expect(
      resolvePaymentLaunchPlan({'nextAction': 'redirect', 'payUrl': 'https://pay.example'}).kind,
      PaymentLaunchKind.redirect,
    );
    expect(
      resolvePaymentLaunchPlan({'nextAction': 'qr_code', 'qrCodeUrl': 'weixin://wxpay/x'}).kind,
      PaymentLaunchKind.qr,
    );
    expect(
      resolvePaymentLaunchPlan({'nextAction': 'cashier', 'cashierUrl': 'https://c/1'}).kind,
      PaymentLaunchKind.cashier,
    );
  });

  test('without nextAction the payload precedence matches the server', () {
    // Signed App string wins over a QR, a redirect wins over a QR.
    expect(
      resolvePaymentLaunchPlan({'orderStr': 's', 'qrCodeUrl': 'q'}).kind,
      PaymentLaunchKind.alipayApp,
    );
    expect(
      resolvePaymentLaunchPlan({'payUrl': 'https://p', 'qrCodeUrl': 'q'}).kind,
      PaymentLaunchKind.redirect,
    );
    expect(resolvePaymentLaunchPlan({'qrCodeUrl': 'q'}).kind, PaymentLaunchKind.qr);
    expect(resolvePaymentLaunchPlan({}).kind, PaymentLaunchKind.cashier);
  });

  test('launch plans describe what the payer still has to do', () {
    final jsapi = resolvePaymentLaunchPlan({'nextAction': 'jsapi', 'jsapiPayload': '{}'});
    expect(jsapi.requiresProviderApp, isTrue);
    expect(jsapi.isNavigation, isFalse);
    final redirect =
        resolvePaymentLaunchPlan({'nextAction': 'redirect', 'payUrl': 'https://p'});
    expect(redirect.isNavigation, isTrue);
    final qr = resolvePaymentLaunchPlan({'nextAction': 'qr_code', 'qrCodeUrl': 'q'});
    expect(qr.isScannable, isTrue);
    // An empty payload stays visible as a plan, not as a silent no-op.
    final empty = resolvePaymentLaunchPlan({'nextAction': 'qr_code'});
    expect(empty.kind, PaymentLaunchKind.qr);
    expect(empty.payload, isNull);
  });
}