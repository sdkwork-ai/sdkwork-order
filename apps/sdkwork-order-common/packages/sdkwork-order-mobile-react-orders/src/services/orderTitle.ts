/**
 * Localized order titles for the mobile order UI.
 *
 * Order subjects and line-item titles come from the order backend. Some
 * canonical flows ship machine-readable identifiers or English labels
 * (e.g. the membership subject "Membership", the recharge subject
 * "points_recharge") that should render as localized plan titles instead.
 *
 * Known identifiers map onto `orders.subject_*` resource keys; unknown
 * values render unchanged so future backend titles keep working and no
 * backend subject is ever masked by the UI layer.
 */

/** Minimal `t` shape consumed by the helper. The second parameter is
 * intentionally untyped: react-i18next's `TFunction` narrows it to `TOptions`
 * in newer i18next type versions while older versions accept a plain default
 * string, and hosts (im-h5 / order-h5) resolve different type versions. */
export interface TranslateFunction {
  (key: string, defaultValue?: any): string;
}

/** Known backend subject identifiers → `orders.*` resource keys. */
const KNOWN_SUBJECT_KEYS: Readonly<Record<string, string>> = {
  membership: "orders.subject_membership",
  "membership subscription": "orders.subject_membership",
  vip: "orders.subject_membership",
  "vip membership": "orders.subject_membership",
  points_recharge: "orders.subject_points_recharge",
  "points recharge": "orders.subject_points_recharge",
  token_bank_recharge: "orders.subject_token_bank_recharge",
  "token bank recharge": "orders.subject_token_bank_recharge",
};

/** Normalizes a backend label for matching: trim, lowercase, `_` → space. */
export function normalizeOrderLabel(value: string): string {
  return value.trim().toLowerCase().replace(/_/g, " ").replace(/\s+/g, " ");
}

/**
 * Returns the localized title for an order subject or line-item title.
 * Falls back to the raw backend value when the label is unknown or the
 * host ships no resource for the matched key.
 */
export function localizeOrderTitle(label: string, t: TranslateFunction): string {
  if (!label) {
    return label;
  }
  const key = KNOWN_SUBJECT_KEYS[normalizeOrderLabel(label)];
  return key ? t(key, label) : label;
}
