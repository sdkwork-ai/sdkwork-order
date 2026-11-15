# Payment Callback Notification Standard（支付回调通知标准）

Normative standard for **every** provider notification this workspace owns: the
payment-success callback, the payment-failure/closure callback, the refund
callback, and any future financial-event family. It is the single authority for

- which URL a payment channel is told to call back on,
- how a delivery is authenticated, scoped to a tenant/organization, made
  idempotent, and acknowledged,
- how a confirmed success is dispatched to business fulfillment (virtual goods,
  physical goods, membership, recharge, app template, external capability),
- how refund notifications mirror the payment-success flow,
- how every application startup integrates the same framework.

`../sdkwork-specs/API_SPEC.md` (HTTP envelope, problem details),
`../sdkwork-specs/IAM_SPEC.md` + `../sdkwork-specs/IAM_LOGIN_INTEGRATION_SPEC.md`
(tenant/access-token isolation), `../sdkwork-specs/WEB_BACKEND_SPEC.md`
(route auth modes) and `../sdkwork-specs/RUST_CODE_SPEC.md` remain authoritative
for their own surfaces. This document narrows them for the notification flow.

Machine-readable companion: [`commerce-payment-webhook.spec.json`](./commerce-payment-webhook.spec.json).

---

## 1. Scope

| In scope | Owner |
| --- | --- |
| PSP payment success / failure / closure notification intake | `sdkwork-routes-order-app-api` (`payment_webhook_framework`) |
| PSP refund notification intake | same framework, own URL + family policy |
| Provider signature verification + normalization | `sdkwork-order-service` ports, implemented by `sdkwork-order-integration-payment` over `sdkwork-payment-providers` |
| Idempotent event persistence + payment/refund state application | `sdkwork-payment-repository-sqlx` |
| Business fulfillment dispatch | `sdkwork-order-service` `PaymentNotifyHandlerRegistry` |
| Order/refund state transitions | `sdkwork-order-repository-sqlx` |
| Notify URL construction | `sdkwork-payment-providers` `notify_url` module |

Out of scope: buyer-facing cashier redirects (`return_url`), admin
reconciliation, and the payment center's own refund/checkout surfaces — they
*must* consume this standard's URL contract but do not own the intake.

---

## 2. Canonical URL contract

```text
POST {notify-base}/app/v3/api/orders/payments/webhooks/{providerCode}   payment family
POST {notify-base}/app/v3/api/orders/refunds/webhooks/{providerCode}    refund family
```

Rules:

1. **One path per family.** `sdkwork-payment-providers::notify_url` owns both
   templates (`ORDER_PAYMENT_WEBHOOK_PATH`, `ORDER_REFUND_WEBHOOK_PATH`) and the
   builders. No other crate may hand-assemble a notify path.
2. **`{providerCode}` is the normalized provider code** (`trim → lowercase →
   '-'→'_'`). `WeChat-Pay` and `wechat_pay` are one channel with one path.
3. **No query string, no fragment, no trailing `//`.** The base is normalized
   once (`trim_end_matches('/')`); paths are appended as-is.
4. **`{notify-base}` resolution order** (identical for both families):
   1. the tenant's active default row in `commerce_payment_notify_domain`
      (exact `organization_id` row, else the platform `'0'` row);
   2. the deployment `ORDER_PAYMENT_WEBHOOK_BASE_URL`;
   3. otherwise **no callback** — checkout and refund submission MUST log a
      warning; settlement then depends on the compensation sweep, never on a
      silent assumption that the PSP was told where to call.
5. **HTTPS outside local development.** `commerce_payment_notify_domain`
   rejects non-`https` protocols in every environment except
   `development|dev|local|test|testing`; HTTP is allowed only for loopback
   development.
6. **Channel capability decides whether a refund URL is sent.** See §6.

`notify-base` is shared by both families so a tenant configures **one** origin,
not two.

---

## 3. The one pipeline

Every intake URL runs the same stages, in this order. No stage may be skipped,
duplicated, or reordered per URL:

```text
intake      bounded body read (512 KiB) BEFORE any signature work
  ↓
verify      channel signature/decryption over the raw bytes
  ↓
normalize   provider payload → PaymentNotifyEvent (event id, type, out_trade_no, status, payload)
  ↓
identify    stable provider-scoped event id (provider id, else sha256 fingerprint fallback)
  ↓
classify    WebhookEventFamily from the verified event type
  ↓
policy      does this intake URL serve that family?
  ↓
ingest      tenant-scoped, idempotent event persistence + attempt/refund state application
  ↓
dispatch    business fulfillment by subject kind (payment) / refund post-processing
  ↓
ack         standardized HTTP response
```

### 3.1 Intake

- The body limit is `WEBHOOK_BODY_MAX_BYTES` (512 KiB), applied as an axum
  `DefaultBodyLimit` on the router so oversized forged payloads are rejected
  with `413` **before** signature work or persistence.
- The intake is transport-neutral: headers + raw body only. The framework never
  reads a business payload field to make a routing decision before the
  signature is verified.

### 3.2 Verify / normalize

- Verification uses the raw request bytes, never a re-serialized body.
- The channel adapter is resolved from the tenant's
  `commerce_payment_provider_account` binding first, then the deployment
  credentials. A failed resolution is a provider error, not a validation error.
- Normalization produces exactly the order-domain projection
  (`PaymentNotifyEvent`); the service crate never links a provider adapter.

### 3.3 Identity

- The provider event id is used when present.
- Otherwise a stable fallback is derived:
  `{provider_code}:{out_trade_no}:{event_type}:{sha256(payload)[..16]}`.
  Two distinct events for one trade stay distinct; a redelivery of the same
  event collapses onto the same identity.

### 3.4 Classify (open but negative-safe)

| Family | Detected by | Dispatch |
| --- | --- | --- |
| `Refund` | event type contains `refund` | refund pipeline |
| `Payment` | anything else, **including no event type** | payment pipeline |
| `Unsupported` | event type names a foreign financial family (`payout`, `dispute`, `chargeback`, `balance`, `transfer`, `settlement`) | audit + `200` ack, **no payment effect** |

`Refund` wins over every other marker. `Unsupported` exists so a future or
foreign family can never be applied to a payment attempt as if it were a
payment status.

### 3.5 Policy

| Intake | `Payment` | `Refund` | `Unsupported` |
| --- | --- | --- | --- |
| payments URL (`All`) | dispatch | dispatch | audit + `200` ack |
| refunds URL (`RefundOnly`) | audit + `4xx` | dispatch | audit + `4xx` |

A `4xx` is the correct answer for a misrouted delivery: the PSP retries, the
operator sees the configuration error, and the audit row records why.

### 3.6 Ingest (idempotency + tenant scope)

- The event is persisted with `ON CONFLICT (tenant_id, event_id) DO NOTHING`;
  a duplicate delivery is reported as `replayed` and re-applies the same stored
  status.
- Payment/refund state application is transactional and terminal-safe:
  `FOR UPDATE` on the attempt, conditional status update, `applied = false`
  when a stale status conflicts with a terminal state.
- Amount/currency integrity: a recognized notified amount that disagrees with
  the stored attempt MUST NOT settle the payment; the event is stored as
  `failed` with the reason and acknowledged (no retry loop), preserving the
  payload for forensics and manual replay.

### 3.7 Dispatch (payment success)

The pipeline confirms the payment, advances the order, then dispatches
fulfillment through `PaymentNotifyHandlerRegistry` keyed by the order subject
kind. The matrix is closed — a paid order never leaves settlement without
exactly one fulfillment decision:

| Subject kind | Business type | Port | Terminal outcome |
| --- | --- | --- | --- |
| `product` / `physical` / `physical_shipment` | `product` | `PhysicalGoodsFulfillmentPort` | `awaiting_shipment` |
| `virtual_goods` / `virtual` / `virtual_delivery` | `virtual_goods` | `VirtualGoodsFulfillmentPort` | `fulfilled` |
| `membership` / `membership_activation` | `membership` | `MembershipPurchaseFulfillmentPort` | `fulfilled` |
| `app_template` / `deploy_app_template` | `app_template` | `AppTemplatePurchaseFulfillmentPort` | `fulfilled` |
| `points_recharge` / `points_credit` | `points_recharge` | recharge store + `AccountPointsCreditPort` | `fulfilled` |
| `token_bank_recharge` / `token_bank_plan_purchase` / `token_bank_plan_renewal` / `account_recharge_package` | same names | account-value store + `AccountValueLedgerPort` | `fulfilled` |
| `coupon_recharge` | `coupon_recharge` | coupon redemption + ledger | `fulfilled` |
| `coupon_package` / other machine subject | `external` | — | `awaiting_external_fulfillment` (deferred) |
| unknown/display text | `unknown` | — | `awaiting_subject_resolution` (deferred) |

Adding a family = add a port + a `PaymentNotifyHandler` + register it. **No
pipeline code changes.** Handlers MUST be idempotent on
`{family}:fulfill:{order_id}` (or the family's documented key).

A capability that is not installed degrades to a *deferred* status
(`awaiting_virtual_goods_fulfillment`, `awaiting_subject_resolution`, …) — it
MUST NOT fail the webhook, because the payment and order state are already
advanced and a `5xx` would make the PSP retry an event that can never succeed.

### 3.8 Ack

- Success: HTTP `200` with `SdkWorkApiResponse` `{ code: 0, data: { accepted, status, resourceId? } }`.
- Verified-but-unmatched / unsupported / terminal-conflict: `200`, `accepted` /
  `unmapped` / `unsupported_family` — never a retry loop.
- Invalid signature, malformed payload, oversize body, or misrouted family:
  `4xx` `application/problem+json` with a generic message. Detailed reasons are
  logged and persisted, never returned to the provider.
- Internal failure after a verified event: `5xx`, which is the provider's cue to
  redeliver the same event; ingest deduplicates the retry.

---

## 4. Tenant and organization isolation

PSP deliveries carry **no platform credential**. Isolation is therefore derived
from persisted facts, never from the request:

1. **Scope source of truth.** `out_trade_no` + `providerCode` resolve exactly one
   `commerce_payment_attempt`. Zero matches → the event is stored under the
   resolved/none scope with reason `payment_attempt_not_found` and acked; more
   than one match → `conflict`, never an arbitrary pick.
2. **Derived scope wins.** `tenant_id` and `organization_id` come from the
   matched attempt. A caller-supplied scope can never widen or retarget the
   settlement.
3. **Organization sentinel.** `organization_id = '0'` means platform scope; the
   matcher accepts `organization_id = '0'` when the expected organization is
   `NULL` and never otherwise.
4. **Untrusted before verification.** A rejected delivery (bad signature,
   normalization failure, misrouted family) is persisted under tenant `'0'`
   with a body fingerprint id, because its scope cannot be trusted yet.
5. **Cross-tenant writes are impossible by construction.** Every downstream
   statement carries `tenant_id` + `organization_id` predicates; there is no
   "system" escape hatch on the settlement path.
6. **Provider-account scoping.** When several active accounts exist for one
   channel, the account is resolved by tenant scope first; per-account
   verification is bounded (candidate cap) and a cap hit is logged loudly rather
   than silently attributing the payment to the wrong account.
7. **No identity projection headers.** Webhook handlers MUST NOT read
   `x-sdkwork-tenant-id`, `X-Tenant-Id`, `X-Platform`, or `X-User-Id`; the
   platform suppresses them on the wire (`I18N_SPEC` §4 / `API_SPEC` §10.2).

---

## 5. Access-token isolation (non-webhook success entry points)

The manual/backend success path (`POST /backend/v3/api/orders/{orderId}/payment_confirmations`)
is **not** anonymous. Standard:

1. The route requires `RouteAuth::BackendToken` + the `commerce.orders.fulfill`
   permission; the operator's tenant/organization come from the access token's
   request context (`IamAppContext` / `WebRequestContext`), never from the body.
2. The `{orderId}` is loaded **within** that scope; an order owned by another
   tenant/organization is `404`, not `403` (no existence oracle).
3. The idempotency key is server-derived from the token + method + route;
   a caller-chosen key cannot replay another order's settlement.
4. **Registry parity is mandatory.** The confirmation path MUST use the same
   `PaymentNotifyHandlerRegistry` instance as the webhook intake. Two entry
   points into one payment-success flow with two different registries is a
   defect: a business handler registered for the PSP callback would be silently
   bypassed by an operator re-confirmation. Deployments inject ONE registry and
   pass it to both mounts.
5. The webhook admission in `http_route_manifest` is `HttpRoute::public`; adding
   a token requirement to a PSP callback route is a contract violation.

---

## 6. Refund notification parity

Refunds mirror payment success: same verify/normalize/identify stages, the same
tenant scoping, the same idempotent ingest, an ack with the same vocabulary.

| Channel | Refund notify capability | Delivery |
| --- | --- | --- |
| WeChat Pay v3 | `dedicated_url` | `notify_url` is sent on refund creation → refunds URL |
| sandbox | `dedicated_url` | synthetic, mirrors WeChat |
| Stripe | `payment_url_family` | `charge.refunded` / `refund.*` arrive on the **payments** URL; family classification routes them to the refund pipeline |
| PayPal | `payment_url_family` | `PAYMENT.CAPTURE.REFUNDED` on the payments URL |
| Alipay | `unsupported` | Alipay sends **no** async refund notification; refund state is resolved by refund query / compensation |
| unknown channel | `unsupported` | never handed a callback URL |

Rules:

1. The refund URL is built from the **same** notify base as the payment URL
   (`resolve_refund_notify_url_postgres`).
2. Every refund submission path (`order` executor, payment app router, payment
   backend router, payment compensation re-submission) applies one rule
   (`refund_notify_url_for_channel`): forward the URL only for
   `dedicated_url` channels, never emit a blank value.
3. Refund status mapping: `succeeded → refunded`, `failed|canceled →
   refund_failed`, `processing → refunding`; terminal states are never
   overwritten by a late notification.
4. A refund event that cannot be matched to a refund row is acked and audited
   (forensics), never retried.

---

## 7. Application startup integration contract

There is exactly **one** way an application mounts the notification framework.
`build_order_app_business_router(host)` (aliased `gateway_mount`) is the single
composer; it mounts both intake URLs and passes `host`'s fulfillment ports.

| Application | Mount | Notes |
| --- | --- | --- |
| `sdkwork-api-order-standalone-gateway` | `assemble_api_router_with_runtime_from_env()` → `gateway_mount` | background workers owned by `OrderRuntimeWorkers` and stopped on shutdown |
| `sdkwork-api-cloud-gateway` (embedded) | `sdkwork_api_order_assembly::web_module_with_pool(pool)` → `gateway_mount` | same routes, same registries, shared pool |
| order backend api | `build_order_backend_business_router(host)` | confirmation route, registry parity per §5.4 |

Rules:

1. **No application may build the intake URLs itself.** A new surface merges the
   business router; it never re-declares the paths.
2. **Ports come from the host.** `physical_goods_fulfillment_port` and
   `virtual_goods_fulfillment_port` are wired together from the same commerce
   fulfillment family so no deployment configures one delivery family and
   forgets the other.
3. **Registries are injected once.** `default_payment_notify_handler_registry()`
   / `refund_notify_handler_registry_with(...)` are the defaults; deployments
   that add handlers pass their registry to the webhook mount **and** the
   confirmation mount.
4. **Compensation worker parity.** `spawn_payment_compensation_worker` re-enters
   `process_payment_notify_verified` / `process_refund_notify_verified` with a
   synthetic event and the same registries, so a lost webhook recovers without a
   second success.
5. Route manifests stay `HttpRoute::public` for both intake URLs; the manifest
   test asserts it.

---

## 8. Failure semantics summary

| Condition | HTTP | Retry expected | State effect |
| --- | --- | --- | --- |
| Oversize body | `413` | no | none |
| Bad signature / undecryptable | `401` | no | audit row (tenant `'0'`) |
| Malformed payload | `400` | no | audit row |
| Misrouted family on refund URL | `4xx` | provider-defined | audit row |
| Unknown channel | `4xx` | no | audit row |
| Unmatched `out_trade_no` | `200 accepted` | no | event stored, no effects |
| Unsupported family | `200 unsupported_family` | no | event stored, no effects |
| Terminal conflict (stale status) | `200 accepted` | no | no transition |
| Settlement/DB failure after verify | `5xx` | yes | ingest deduplicates the retry |

---

## 9. Verification checklist

Change any of the following → re-run the corresponding gate.

1. `cargo check --workspace --all-targets` in `sdkwork-order` and
   `sdkwork-payment`.
2. `cargo test --workspace` in `sdkwork-order` (full regression; DB-backed
   suites self-skip without a pool) and
   `cargo test --workspace --lib` in `sdkwork-payment` — classification, family
   policy, virtual-goods delivery/deferral, refund status mapping, notify URL
   builder, refund capability matrix.
3. `cargo test -p sdkwork-routes-order-app-api --test webhook_public_route_standard`
   — both intake URLs reachable with **no** platform credentials.
4. `cargo test -p sdkwork-routes-order-app-api --test webhook_router_standard`
   — body limit `413` before processing, refund URL family policy, registry seam.
5. `cargo test -p sdkwork-routes-order-backend-api --test payment_confirmation_registry_standard`
   — the operator confirmation mount takes the shared registry + virtual-goods
   port and never falls back to the registry-less settlement entry point.
6. `cargo test -p sdkwork-order-service --test payment_notify_orchestration_standard`
   — success/failure/recovery/terminal-conflict orchestration and virtual-goods
   deferral.
7. `cargo test -p sdkwork-order-repository-sqlx --test order_lifecycle_state_machine`
   — terminal guards (late refund fail, recovery success, failure vs paid).
8. `node ../sdkwork-specs/tools/check-api-operation-patterns.mjs --workspace .`
   and `.../check-api-response-envelope.mjs --workspace .` when routes or
   response envelopes change.
9. `node ../sdkwork-specs/tools/check-rust-crate-naming-standard.mjs --root .`
   and `.../check-rust-manifest-standard.mjs --root .` (required for any
   `Cargo.toml` change; cheap to run always).
10. `pnpm verify` (contract + docs gates) when specs or docs change.

Reviewer questions this standard answers:

- Which URL is a channel told to call, and who builds it?
- What happens to a delivery nobody can be authenticated as belonging to?
- Can a foreign event family ever write a payment status?
- Is an operator confirmation identical to a PSP callback?
- Which tenant/organization does the settlement write under, and where did that
  scope come from?
- Can a virtual-goods order be paid without a delivery decision?
- Is the refund callback reachable for this channel, and if not, who notices?
