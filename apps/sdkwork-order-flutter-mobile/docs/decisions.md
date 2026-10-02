# Flutter Mobile Decisions — sdkwork-order-flutter-mobile

- 2026-10-03: v1 ships the spec-shaped Flutter mobile root for the order
  capability (`apps/sdkwork-order-flutter-mobile`, `flutter create` android/ios,
  org `com.sdkwork`, thin `lib/` = entry + bootstrap + shell). Bottom tabs:
  订单 / 充值 / 我的;「我的」hosts 券码兑换、提现、退款入口 plus login. Two
  snake_case packages: `sdkwork_order_flutter_mobile_core` (transport seam,
  environment, session, order/recharge/withdrawal/refund services, pure
  logic) and `sdkwork_order_flutter_mobile_orders` (all screens/widgets,
  constructor-injected services, no `dart:io`, no transport imports).

- Transport seam (same precedent as `sdkwork-mall`): the generated Dart SDK
  family for the order authority has not been produced yet (this repo's
  `sdks/` only carries TypeScript families), so `core` keeps ONE handwritten
  seam — `lib/src/transport/order_transport.dart` — speaking the same order
  app-api contract the H5/PC consumers use: `{code, data, traceId}` envelope
  unwrap (`data.item` / `data.items + pageInfo`), HTTP 4xx/5xx
  `application/problem+json` error mapping (numeric `code` + `traceId`),
  `Authorization: Bearer` injection, `Idempotency-Key` headers on write
  commands, and bounded 15s timeouts via `dart:io HttpClient` — zero pub.dev
  network dependencies. When the Dart SDK family lands, services swap the
  transport for the generated client without screen changes.

- Presentation state: one primary pattern per root = `setState` +
  `ChangeNotifier` (session store only). No riverpod/bloc/provider package —
  the surface is a handful of flows; screens keep local state and call
  injected services directly, matching `APP_FLUTTER_UI_SPEC.md` §3 ("choose
  one pattern and stay consistent"). Recorded here and in
  `specs/component.spec.json`.

- i18n: 中文为主文案 kept as typed constants in the orders package
  (`src/i18n/strings.dart`). A full ARB/gen-l10n framework is deferred until
  a second locale is actually required; when it lands, fragments move under
  `lib/src/i18n/<locale>/…` per `I18N_SPEC.md` §6.1 without screen changes.

- Money: all amounts are minor-unit integer strings on the wire
  (`"6990"` = ¥69.90, `API_SPEC.md` §13.6 int64-as-string discipline applied
  to money too). Pure helpers live in `core` `logic/money.dart` (BigInt
  formatting, major→minor form conversion); no doubles anywhere in the
  amount path. Plan price amounts (`priceAmount`, major-unit decimal from
  `GET /recharges/plans`) are converted to minor strings at recharge-order
  creation — same rule the H5 subscription service follows.

- Cashier: mirrors the mobile-react `CashierLogic` contract — poll
  `payment_success` + order status every 3s, back off to 6s after 3
  consecutive poll failures (notice 网络不稳定), countdown =
  min(order `expireTime`, paymentCreatedAt + 15min TTL), and one final
  authoritative check at zero before declaring the cashier expired. Native
  WeChat/Alipay channel invokers are platform-host work (see follow-ups);
  the cashier still creates the payment server-side and polls, so `balance`
  completes end-to-end today.

- Payment methods offered: `wechat_pay` / `alipay` / `balance` (the CN
  browser matrix from `ORDER_PAYMENT_METHODS`); wire methods
  `wechat_jsapi` / `alipay_wap` stay out until platform hosts exist.

- Login: IAM Dart family pending — token session only
  (`core` `session/app_session.dart`), seeded by the login page (manual
  token entry) or a bootstrap-injected dev token; transport reads the
  session through a provider function. Secure-storage persistence plugs in
  with the identity contract.

- Follow-ups: generate `sdkwork_order_*` Dart SDK family and swap the
  services over; persist the session in secure storage; wire
  `wechat_jsapi`/`alipay_wap` native channels behind typed platform
  adapters (host package); add console/admin package families if operator
  surfaces are approved; wire store signing profiles (applicationId is the
  flutter-create default `com.sdkwork.sdkwork_order_flutter_mobile` while
  the manifest declares the storefront id `com.sdkwork.order.flutter`).
