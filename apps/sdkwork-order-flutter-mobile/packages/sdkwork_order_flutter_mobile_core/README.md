# sdkwork_order_flutter_mobile_core

Core runtime package for the SDKWork Order Flutter mobile root
(`FLUTTER_APP_MOBILE_ARCHITECTURE_SPEC.md` §3 `core` role). Owns:

- `src/environment/` — typed dart-define environment (`SDKWORK_*` keys).
- `src/session/` — global token-manager equivalent (`AppSession`).
- `src/transport/` — the SINGLE order app-api transport seam
  (`dart:io HttpClient`, envelope unwrap, ProblemDetail mapping, bearer,
  `Idempotency-Key`). Swap point for the generated Dart SDK family.
- `src/services/` — order / recharge / withdrawal / refund domain services.
- `src/logic/` — pure, unit-tested logic: cashier poll backoff + phase
  resolution + countdown, minor-unit money formatting, envelope helpers,
  idempotency keys.
- `src/models/` — typed read models mapped defensively from loose wire
  payloads (replaced by generated SDK DTOs when the family lands).

Must not own screens or widgets (`APP_FLUTTER_UI_SPEC.md` §2).
