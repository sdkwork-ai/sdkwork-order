# Account Value Order Spec

Status: active  
Owner: SDKWork maintainers  
Capability: `commerce.order`  
Updated: 2026-11-14

Authority: `sdkwork-specs/API_SPEC.md`, `sdkwork-specs/DATABASE_SPEC.md`, `sdkwork-account/specs/COMMERCE_BOUNDARY_SPEC.md`, `sdkwork-payment/specs/PAYMENT_EXECUTOR_SPEC.md`

## 1. Purpose

This spec defines the order-owned business layer for account value movement.

Recharge, coupon redemption, refund, and withdrawal orchestration belong to `sdkwork-order`. Every value-changing user action that needs commercial evidence must create or reference a `commerce_order` or order-owned request record before money, coupon, payout, or account ledger effects happen.

`sdkwork-payment` executes provider payment and refund channels today, and owns any future provider payout executor contract. `sdkwork-account` is the ledger truth source for balances, holds, settlement, reversal, and immutable journal entries.

## 2. Naming Model

Use `Token Bank` for the AI-era intermediate value capability and `token_bank` for the account asset code. Do not introduce `compute_credit`, `token`, or other parallel asset names for the same purpose. `Compute Credits` is the English *display* name of the `token_bank` asset (its Chinese display name is `算力额度`); it is a translation, not a fourth asset.

| Term | Canonical code | Display (en-US / zh-CN) | Meaning |
| --- | --- | --- | --- |
| Token Bank | `token_bank` | `Compute Credits` / `算力额度` | Intermediate account asset used by LLM, image, video, Agent, workflow, plugin, and model-service consumption. Ledger unit: micro-credits (1 credit = 1e6 micro). |
| Points | `points` | `Membership Points` / `会员积分` | Traditional loyalty or marketing points. Ledger unit: integer points. |
| Cash balance | `cash` | `Cash balance` / `现金余额` | Withdrawable stored value when a product requires cash-account withdrawal. Ledger unit: currency minor units. |
| Membership entitlement | `subscription` | `Subscription` / `会员订阅` | **Not an account asset.** Fulfilled by the membership service; it must never be written to `acct_ledger_entry`. |

Only `cash`, `points`, and `token_bank` exist as ledger assets (`CommerceAccountAssetType`). `subscription` appears in order/coupon payloads solely as a fulfillment target routed to the membership capability.

### 2.1 Canonical subject → asset → ledger mapping

| Subject | Target asset | Fulfillment ledger `business_type` | Compensation `business_type` | Dispatcher |
| --- | --- | --- | --- | --- |
| `points_recharge` | `points` | `points_recharge` | `points_cloudback` (legacy spelling, see §13) | `AccountPointsCreditPort` |
| `token_bank_recharge` | `token_bank` | `token_bank_purchase_credit` | `token_bank_reversal` | Account value ledger port |
| `token_bank_plan_purchase` | `token_bank` | `token_bank_grant` | `token_bank_reversal` | Account value ledger port |
| `token_bank_plan_renewal` | `token_bank` | `token_bank_grant` | `token_bank_reversal` | Account value ledger port |
| `account_recharge_package` | package target asset (`points` / `token_bank` / `cash`) | by asset: `points_recharge` / `token_bank_purchase_credit` / `cash_adjustment` | by asset: `points_cloudback` / `token_bank_reversal` / `cash_adjustment` | Account value ledger port |
| `coupon_recharge` | coupon benefit asset | `token_bank_grant` (Token Bank), `points_earn` (Points), `cash_adjustment` (Cash), membership service (Subscription) | by asset | Account value ledger port / membership port |
| `refund_request` | reversed asset | `token_bank_hold_release` / `points_cloudback` / `cash_adjustment` / `manual_adjustment` | same table | Refund notify port |
| `cash_withdrawal` | `cash` | payout path (`cash_adjustment`) | `cash_adjustment` | Payout executor port |
| `membership` | `subscription` | membership service (no ledger row) | — | Membership notification port |
| `product`, `virtual_goods`, `app_template` | — | `product` / `external` / `app_template` | — | Fulfillment/shipment handlers |

`points_recharge` and `token_bank_recharge` are **different assets and different settlement chains**. An order that credits Token Bank must never carry `points_recharge`, and a `points_recharge` order must never be fulfilled from the Token Bank ledger.

### 2.2 Display-label contract

Every client surface resolves order subjects and asset codes through a catalog that emits these labels; it must never render the raw wire code.

| Code | en-US | zh-CN |
| --- | --- | --- |
| `product` | Product order | 商品订单 |
| `virtual_goods` | Virtual goods order | 虚拟商品订单 |
| `membership` | Membership subscription | 会员订阅 |
| `points_recharge` | Membership Points recharge | 会员积分充值 |
| `token_bank_recharge` | Token Bank recharge | Token Bank 充值 |
| `token_bank_plan_purchase` | Token Bank plan purchase | Token Bank 套餐购买 |
| `token_bank_plan_renewal` | Token Bank plan renewal | Token Bank 套餐续订 |
| `account_recharge_package` | Account value package recharge | 账户价值包充值 |
| `coupon_recharge` | Coupon recharge | 兑换码充值 |
| `refund_request` | Refund request | 退款申请 |
| `cash_withdrawal` | Cash withdrawal | 提现申请 |

Asset display names follow §2 table. A host may append product-specific subjects, but it must not rename `points` as Token Bank or the reverse.

## 3. Account Value Order Subjects

| Subject | Owner | Target ledger asset | Provider channel | Required commercial evidence |
| --- | --- | --- | --- | --- |
| `points_recharge` | Order | `points` | Payment collection | `commerce_order` |
| `token_bank_recharge` | Order | `token_bank` | Payment collection | `commerce_order` |
| `token_bank_plan_purchase` | Order | `token_bank` grant or allowance | Payment collection | `commerce_order` + immutable plan snapshot |
| `token_bank_plan_renewal` | Order | `token_bank` renewal grant or allowance | Payment collection | `commerce_order` + immutable plan snapshot |
| `account_recharge_package` | Order | package target asset | Payment collection | `commerce_order` + immutable package snapshot |
| `coupon_recharge` | Order | coupon target asset | zero amount or mixed payment | `commerce_order` + coupon redemption evidence |
| `refund_request` | Order | reversal of original value order | Provider refund | `commerce_order_refund_request` |
| `cash_withdrawal` | Order | `cash` | Provider payout | `commerce_order_withdrawal_request` |

`product`, `virtual_goods`, `membership`, and other commerce subjects remain order subjects, but they are not account value order subjects unless they directly create, reverse, hold, or settle an account ledger asset.

## 4. Ownership Boundaries

| Capability | Owns | Must not own |
| --- | --- | --- |
| `sdkwork-order` | order records, account value packages, Token Bank plans, coupon redemption orders, refund request state, withdrawal request state, idempotency scopes, saga orchestration | provider SDK calls, direct account SQL writes |
| `sdkwork-payment` | payment intent, payment attempt, provider refund execution, future provider payout execution, provider channel config, provider webhook event persistence | recharge routes, refund business approval, withdrawal business approval, account ledger side effects |
| `sdkwork-account` | accounts, balances, ledger journal, holds, settlement, release, reversal, idempotent ledger commands | order creation, package or plan catalog, provider payment/refund/payout calls |

Allowed dependency direction:

```text
sdkwork-order -> sdkwork-payment
sdkwork-order -> sdkwork-account
sdkwork-payment -X-> sdkwork-account
sdkwork-payment -X-> sdkwork-order service crates
sdkwork-account -X-> sdkwork-order
sdkwork-account -X-> sdkwork-payment
```

Payment may validate an existing `orderId` against order data through an approved read-only contract, but payment must not create `commerce_order`, call account ledger APIs, import account crates, or write account tables.

## 5. State Machines

### Paid recharge and package orders

```text
draft/requested
  -> pending_payment
  -> paid
  -> ledger_processing
  -> fulfilled
  -> closed
```

Failure states:

- `payment_failed`: payment provider attempt failed before ledger impact.
- `ledger_failed`: payment succeeded but account command failed; operator replay uses the order settlement route.
- `cancelled`: user or operator cancelled before successful payment.

### Coupon recharge orders

```text
requested
  -> coupon_validated
  -> pending_payment       # only for mixed-payment coupon recharge
  -> ledger_processing
  -> fulfilled
```

Zero-amount coupon redemption skips provider payment and still writes an order record so the user, operator, and ledger can trace the value grant.

### Refund requests

```text
requested
  -> account_reversal_held
  -> provider_refund_processing
  -> refunded
```

Failure states:

- `rejected`: business approval failed.
- `account_hold_failed`: refundable ledger balance cannot be held or reversed.
- `provider_refund_processing`: payment provider submission or reconciliation is pending or ambiguous; order retains the account hold, queries through the original provider account, and reuses the original Payment refund identity.
- `provider_refund_failed`: payment provider refund failed deterministically; only then may order release or compensate the account hold.

### Cash withdrawal requests

```text
requested
  -> account_cash_held
  -> provider_payout_processing
  -> paid_out
```

Failure states:

- `rejected`: business approval failed.
- `account_hold_failed`: withdrawable cash cannot be held.
- `provider_payout_failed`: payout provider failed; order releases the account hold.

## 6. Token Bank Plan Rules

Token Bank plan order records must snapshot all commercial facts that affect entitlement or ledger credit:

- `plan_code`
- billing period: `monthly`, `quarterly`, `yearly`, `continuous_monthly`, `continuous_yearly`
- grant asset: `token_bank`
- grant quantity
- bonus quantity
- price currency
- price amount
- renewal policy
- provider channel constraints
- effective start and end timestamps

Continuous plans create a distinct `token_bank_plan_purchase` order for the first cycle and `token_bank_plan_renewal` orders for renewals. A renewal must never mutate the original order amount or original plan snapshot.

## 7. Flow Contracts

### Paid Token Bank recharge

```text
1. recharges.orders.create subject=token_bank_recharge
2. orders.payments.create creates or reuses a payment intent through sdkwork-payment
3. PSP callback enters order-owned webhook route
4. order settles payment state
5. order calls sdkwork-account to credit asset_code=token_bank
6. order marks fulfillment complete
```

### Account recharge package

```text
1. user selects a package
2. order snapshots package facts into commerce_order_item.sku_snapshot_json
3. payment is executed through sdkwork-payment
4. account ledger is credited through sdkwork-account after payment success
```

### Coupon recharge

```text
1. order validates coupon through the promotion or coupon port
2. order creates subject=coupon_recharge with coupon evidence
3. zero-amount orders skip provider payment; mixed-payment orders call sdkwork-payment
4. order calls sdkwork-account for the target asset credit
```

### Refund request

```text
1. user or operator creates a refund request referencing the original order
2. order validates refundable amount and ledger impact
3. order asks sdkwork-account to hold or reverse the granted account value
4. order asks sdkwork-payment to execute provider refund using the original Payment Attempt provider/account/transaction snapshot
5. submitted or ambiguous provider state keeps the account hold while the same refund identity remains retryable
6. provider-confirmed success commits account reversal; deterministic failure releases the account hold
```

### Cash withdrawal

```text
1. user creates withdrawal request for asset_code=cash
2. order asks sdkwork-account to hold withdrawable cash
3. order asks the configured `PaymentPayoutExecutorPort` to execute provider payout
4. success settles and debits the account hold; failure releases the hold
5. current default runtime is fail-closed until `sdkwork-payment` exposes a concrete provider payout executor
```

## 8. Idempotency

All write commands marked `x-sdkwork-idempotent` must accept `Idempotency-Key`. The owning service computes and stores the canonical request fingerprint; app and backend clients must not send request-hash or fingerprint headers.

Canonical idempotency scopes:

| Flow | Scope |
| --- | --- |
| Token Bank recharge fulfillment | `token-bank-recharge:fulfill:{orderId}` |
| Token Bank plan purchase grant | `token-bank-plan:purchase:{orderId}` |
| Token Bank plan renewal grant | `token-bank-plan:renewal:{orderId}` |
| Account package fulfillment | `account-package:fulfill:{orderId}` |
| Coupon recharge fulfillment | `coupon-recharge:fulfill:{orderId}` |
| Refund account hold | `refund-request:account-hold:{refundRequestId}` |
| Refund provider execution | `refund-request:payment-refund:{refundRequestId}` |
| Withdrawal account hold | `withdrawal:account-hold:{withdrawalRequestId}` |
| Withdrawal provider execution | `withdrawal:payment-payout:{withdrawalRequestId}` |

Request hashes must include tenant, organization, owner, subject, target asset, amount, currency, original order reference when present, coupon code when present, package or plan code when present, and client request number.

## 9. API And SDK Rules

- App users create account value orders through order app-api resources such as `recharges.*`, `orders.refundRequests.*`, and `withdrawals.requests.*`.
- Backend operators manage account value packages, Token Bank plans, refund review, and withdrawal review through order backend-api resources.
- Frontend and service consumers must use `@sdkwork/order-app-sdk` or `@sdkwork/order-backend-sdk`; raw HTTP and generated transport package imports are forbidden.
- SDKWork-owned HTTP contracts must use `SdkWorkApiResponse` for success and `ProblemDetail` for errors.
- Lists must use SDKWork pagination (`data.items` and `data.pageInfo`) and store-level SQL pagination.

## 10. Database Rules

Greenfield order-owned account value tables:

| Table | Purpose |
| --- | --- |
| `commerce_account_value_package` | Recharge package catalog for points, Token Bank, or other account assets |
| `commerce_token_bank_plan` | Token Bank one-time and continuous plan catalog |
| `commerce_order_refund_request` | Refund request workflow and provider refund execution reference |
| `commerce_order_withdrawal_request` | Cash withdrawal workflow, account hold reference, and provider payout execution reference when payout execution is available |

Immutable commercial evidence must be copied into order rows or order item snapshots. Account ledger rows must store ledger facts, not package or plan catalog truth.

## 11. Security And Operations

- Refund and withdrawal approval routes require backend IAM permissions and audit events.
- Provider refund and future payout retries must be idempotent and tied to order-owned request ids.
- Ledger commands must be idempotent, tenant-scoped, and organization-scoped.
- Failed ledger, refund, or fail-closed payout steps must be recoverable through backend replay routes.
- Sensitive provider payloads must stay in payment-owned tables; order stores only execution references and business status.

## 12. Forbidden

- direct account SQL writes from order
- payment-owned recharge routes
- payment-to-account ledger writes
- naked token account naming
- order-owned provider SDK calls
- account-owned recharge, refund, or withdrawal orchestration
- payment-created `commerce_order` records
- mutable plan or package evidence after order creation
- naming the `points` asset "Token Bank" (or the reverse) in any client copy, key, or default
- crediting `token_bank` through `points_recharge` (or the reverse) "for compatibility"

## 13. Known Contract Debt (tracked, not yet migrated)

These are real inconsistencies accepted only until their migration lands. Do not add new code that depends on them.

| # | Debt | Impact | Migration requirement |
| --- | --- | --- | --- |
| D1 | `CommerceLedgerBusinessType::POINTS_CLAWBACK` is spelled `points_cloudback` | Every points reversal persists the misspelled code; `points_clawback` rows do not exist | Rename the constant **and** backfill `acct_ledger_entry.business_type` in one migration; update any report filter |
| D2 | `recharges.orders.create` defaults an omitted `subject` to `points_recharge` | A Token Bank purchase that omits `subject` becomes a points order and credits the points ledger | Make `subject` required for the recharge create command, or default to an explicit `account_recharge_package` |
| D3 | Admin refund create defaults `targetAsset` to `token_bank` | A refund on a points/cash order can target an asset the order never granted | Require the refund asset explicitly, or derive it from the original order's fulfillment ledger row |
| D4 | `AccountValueAssetCode::Subscription` is parseable but has no account counterpart | `targetAsset=subscription` survives DTO validation and fails at the last hop, after the order/refund row exists | Reject `subscription` at the account-value command boundary and route subscription orders only through the membership port |
| D5 | Subscription fulfillment maps to `token_bank_grant` and compensation to `token_bank_reversal` | Subscription orders appear in Token Bank ledger reports with no Token Bank asset movement | Move subscription to the membership port and remove the ledger mapping |
| D6 | The generated app/backend SDK unions expose only `points`/`token_bank`/`cash` while the Rust enum also accepts `subscription` | Client types and server acceptance disagree | Align the OpenAPI enum with D4 resolution |
| D6 | The generated app/backend SDK unions expose only `points`/`token_bank`/`cash` while the Rust enum also accepts `subscription` | Client types and server acceptance disagree | Align the OpenAPI enum with D4 resolution |

### 13.1 Resolved (2026-11-14)

**D7 (Token Bank sales booked as points)** — resolved.

The account-value recharge path now supports the `mobile_cashier_h5` product and server-side pricing:

- `create_account_recharge_order` returns an order-bound cashier deep-link (`cashierUrl` + `qrCodePayload`, `nextAction = cashier`); `submit_recharge` returns it and starts **no** provider payment when the product is `mobile_cashier_h5` (`sdkwork-order-repository-sqlx::postgres_account_value`, `sdkwork-routes-order-app-api::recharge_router`).
- `resolve_account_value_grant_amount` prices `token_bank_recharge` from the exchange rule + package bonus and plan subjects from the immutable Token Bank plan snapshot (`grant_amount + bonus_amount`); for plans it also overrides the payable amount/currency with the catalog values. Client-supplied `grantAmount` is rejected for these subjects.
- Clients switched: `createSdkworkTokenBankRechargeService` (`token_bank_recharge` → `token_bank`), Cloud Router Token Plan modal + Playground, and the H5 Token Bank top-up (`token_bank_recharge` + `packageId`; the catalog those "plans" read is `recharges/packages`, a one-off top-up, so no plan period is involved).

Remaining follow-ups: `account_recharge_package` and `coupon_recharge` still use their existing grant/benefit paths, and historical orders keep their original subject (no data migration is performed).

