# AGENTS.md — sdkwork-order-flutter-mobile

## Capability Identity

- Repository: `sdkwork-order` (domain `commerce`, capability `order`)
- App root: `apps/sdkwork-order-flutter-mobile`
- Surface: app/user-facing Flutter mobile (`appType APP_FLUTTER`, `runtime.family = "mobile"`, `runtime.framework = "flutter"`)
- API: `/app/v3/api/orders`, `/app/v3/api/recharges`, `/app/v3/api/withdrawals` (order app-api only; never `backend-api`)

## Architecture Rules

- Root `lib/` stays thin: `main.dart` entry, `bootstrap/` (dart-define environment + provider composition), `shell/` (MaterialApp + bottom tabs). No screens or business logic here.
- All screens/widgets live in `packages/sdkwork_order_flutter_mobile_orders/`; transport, environment, session, and domain services live in `packages/sdkwork_order_flutter_mobile_core/`.
- Dart packages use lower snake case with the `flutter_mobile` segment (`sdkwork_order_flutter_mobile_*`), per `FLUTTER_APP_MOBILE_ARCHITECTURE_SPEC.md` section 3.
- UI -> service -> injected transport flow is mandatory. Widgets never construct clients, read dart-define, or touch `dart:io`. The transport seam is a single file: `packages/sdkwork_order_flutter_mobile_core/lib/src/transport/order_transport.dart`.
- Presentation state uses `setState` + `ChangeNotifier` only (documented in `docs/decisions.md`); no riverpod/bloc/provider packages.
- No pub.dev network libraries (dio/http); the seam uses `dart:io HttpClient` until the generated Dart SDK family for order lands (see `sdks/README.md` and `docs/decisions.md`).
- Wire contract (`API_SPEC.md` §4.5/§14/§15): success `{code: 0, data, traceId}` with `data.item` / `data.items + pageInfo`; errors are HTTP 4xx/5xx `application/problem+json`; int64 ids stay strings.
- Money amounts are minor-unit integer strings end to end (`"6990"` = ¥69.90); formatting lives in core `logic/money.dart`.
- Write commands (payments, cancellations, recharge orders, coupon redemptions, refund requests, withdrawal requests) send the `Idempotency-Key` header with a fresh UUID scoped to one user attempt.
- Lists are server-paginated (`page`/`page_size`, default 20, max 200); no full-download slicing in the UI.

## Environments

- `env/sdkwork.<deploymentProfile>.<environment>.json` (standalone|cloud × development|test|staging|demo|production, ten files) feeds `--dart-define-from-file`.
- Keys: `SDKWORK_ENVIRONMENT`, `SDKWORK_DEPLOYMENT_PROFILE`, `SDKWORK_PROFILE_ID`, `SDKWORK_RUNTIME_TARGET` (`mobile`), `SDKWORK_ORDER_APP_API_BASE_URL`.

## Verification

```bash
flutter pub get
flutter analyze   # 0 issues across root + packages/
flutter test      # root widget tests
# package-scoped gates:
(cd packages/sdkwork_order_flutter_mobile_core && flutter analyze && flutter test)
(cd packages/sdkwork_order_flutter_mobile_orders && flutter analyze && flutter test)
```

## Boundaries

- Do not edit anything outside this app root.
- Do not hand-edit platform output (`android/`, `ios/`); regenerate with `flutter create --platforms android,ios .` if needed.
- Do not add secrets to `env/`, `config/`, or the manifest; bootstrap-injected dev tokens stay in gitignored local files only.
