# sdkwork_order_flutter_mobile_orders

Capability package holding every screen/widget of the SDKWork Order Flutter
mobile app (`FLUTTER_APP_MOBILE_ARCHITECTURE_SPEC.md` §3 capability role).

- `src/screens/` — route-level UI (orders, order detail, cashier, payment
  result, recharge, coupon redemption, withdrawal, refund requests, login,
  profile).
- `src/widgets/` — domain-neutral loading/empty/error primitives.
- `src/i18n/` — Chinese copy constants (pre-l10n fragment; see root
  `docs/decisions.md`).

Rules enforced here: screens receive services through constructor injection,
never construct clients, never read dart-define, and never import `dart:io`.
Presentation state is `setState` + `ChangeNotifier` (one pattern per root).
