/**
 * Canonical order-subject display contract.
 *
 * `commerce_order.subject` is a machine code (`points_recharge`,
 * `token_bank_recharge`, `product`, …). Rendering it directly leaks a database
 * identifier into operator and shopper UI ("points_recharge"), which is neither
 * localized nor meaningful. Every order surface resolves the subject through
 * this catalog instead, and layers its own `orders.subject.*` copy bundle (or a
 * host-provided override) on top.
 *
 * Unknown values are returned unchanged: a merchandising flow may store a real
 * plan title ("Token Bank 100", "黄金会员月卡") in the subject column, and the UI
 * must never mask it.
 */

/** Minimal `t` shape consumed by the resolver. */
export interface OrderSubjectTranslate {
  (key: string, fallback?: string): string;
}

/**
 * Canonical backend subject code → `orders.subject.*` resource key.
 *
 * Keys are normalized through {@link normalizeSdkworkOrderSubjectCode}, so
 * `Points Recharge`, `points-recharge`, and `points_recharge` share one entry.
 */
const ORDER_SUBJECT_LABEL_KEYS: Readonly<Record<string, string>> = {
  account_recharge_package: "orders.subject_account_recharge_package",
  cash_withdrawal: "orders.subject_cash_withdrawal",
  coupon_recharge: "orders.subject_coupon_recharge",
  membership: "orders.subject_membership",
  membership_subscription: "orders.subject_membership",
  points_recharge: "orders.subject_points_recharge",
  product: "orders.subject_product",
  refund_request: "orders.subject_refund_request",
  token_bank_plan_purchase: "orders.subject_token_bank_plan_purchase",
  token_bank_plan_renewal: "orders.subject_token_bank_plan_renewal",
  token_bank_recharge: "orders.subject_token_bank_recharge",
  virtual_goods: "orders.subject_virtual_goods",
  vip: "orders.subject_membership",
  vip_membership: "orders.subject_membership",
};

/** Every `orders.subject.*` resource key the catalog can emit. */
export const SDKWORK_ORDER_SUBJECT_LABEL_KEYS: readonly string[] = Array.from(
  new Set(Object.values(ORDER_SUBJECT_LABEL_KEYS)),
).sort();

/**
 * Canonical account-value asset code → `orders.target_asset_*` resource key.
 *
 * Refund and withdrawal requests carry the affected account asset
 * (`token_bank`, `points`, `cash`, `subscription`) and must render it through
 * this catalog instead of the raw wire code.
 */
const ORDER_TARGET_ASSET_LABEL_KEYS: Readonly<Record<string, string>> = {
  cash: "orders.target_asset_cash",
  points: "orders.target_asset_points",
  subscription: "orders.target_asset_subscription",
  token_bank: "orders.target_asset_token_bank",
};

/** Every `orders.target_asset_*` resource key the catalog can emit. */
export const SDKWORK_ORDER_TARGET_ASSET_LABEL_KEYS: readonly string[] = Object.values(
  ORDER_TARGET_ASSET_LABEL_KEYS,
).sort();

/**
 * Normalizes a backend subject for matching: trim, lowercase, and collapse
 * `_`/`-`/whitespace runs into one space. `Token Bank Recharge` and
 * `token_bank_recharge` therefore resolve to `token bank recharge`.
 */
export function normalizeSdkworkOrderSubjectCode(value: unknown): string {
  return String(value ?? "")
    .trim()
    .toLowerCase()
    .replace(/[\s_-]+/g, " ");
}

/**
 * Returns the `orders.subject.*` resource key for a known subject code, or
 * `null` when the value is a free-form title the UI must render verbatim.
 */
export function resolveSdkworkOrderSubjectLabelKey(value: unknown): string | null {
  const normalized = normalizeSdkworkOrderSubjectCode(value);
  if (!normalized) {
    return null;
  }
  return ORDER_SUBJECT_LABEL_KEYS[normalized.replaceAll(" ", "_")] ?? null;
}

/**
 * Renders a localized order-subject label.
 *
 * Falls back to the raw backend value when the subject is free-form, empty, or
 * when the caller's copy bundle ships no entry for the matched key — a missing
 * translation must never hide the real subject behind a key name.
 */
export function localizeSdkworkOrderSubject(
  value: unknown,
  t: OrderSubjectTranslate,
): string {
  const raw = String(value ?? "").trim();
  if (!raw) {
    return "";
  }
  const key = resolveSdkworkOrderSubjectLabelKey(raw);
  if (!key) {
    return raw;
  }
  return t(key, raw);
}

/**
 * Renders a localized target-asset label for account-value requests.
 *
 * Falls back to the raw wire code when the asset is unknown or the caller's
 * copy bundle ships no entry, matching {@link localizeSdkworkOrderSubject}.
 */
export function localizeSdkworkOrderTargetAsset(
  value: unknown,
  t: OrderSubjectTranslate,
): string {
  const raw = String(value ?? "").trim();
  if (!raw) {
    return "";
  }
  const key = ORDER_TARGET_ASSET_LABEL_KEYS[normalizeSdkworkOrderSubjectCode(raw).replaceAll(" ", "_")];
  if (!key) {
    return raw;
  }
  return t(key, raw);
}
