# sdks/ — Dart SDK Family (Pending)

This app root has **no generated Dart SDK family yet**. The order authority's
generated SDKs are TypeScript only (`../../sdks/` in this repository):

- `sdkwork-order-app-sdk` (TypeScript, composed facade)
- `sdkwork-order-backend-sdk` (TypeScript)

Per `docs/decisions.md` (mall precedent), the app speaks the order app-api
contract through the single handwritten transport seam:

```text
packages/sdkwork_order_flutter_mobile_core/lib/src/transport/order_transport.dart
```

The seam owns: `{code, data, traceId}` envelope unwrap, ProblemDetail error
mapping, bearer auth, `Idempotency-Key` headers, and bounded timeouts over
`dart:io HttpClient`. Services depend on the `OrderApiTransport` interface;
when `flutter create`-grade Dart codegen for
`sdkwork_order_app_sdk` lands in `sdks/`, each service swaps the transport
for the generated client and this file is deleted. Screens never change.

Generation, once available, goes through the canonical generator
(`sdkwork-sdk-generator`), never by hand.
