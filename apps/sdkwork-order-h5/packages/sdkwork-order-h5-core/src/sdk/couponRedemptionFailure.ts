/**
 * Typed classification of `POST /app/v3/api/orders/coupon_redemptions`
 * failures.
 *
 * The coupon is consumed before the benefit is delivered, so the three
 * customer-visible failure families carry different meaning and different
 * recovery:
 *
 * - `benefit_delivery_failed`: the code was accepted and consumed, but
 *   granting the benefit failed (HTTP 503 with a stable `503xx` code and the
 *   `orders.coupon_redemptions.benefit_delivery_failed` i18n key; older
 *   deployments surface the same state as a generic `500`/`50001`). Retrying
 *   the identical request with the same idempotency key is safe and does not
 *   consume a second coupon.
 * - `invalid_or_expired`: a 4xx validation/not-found answer; the code itself
 *   is wrong, unknown, or expired, so the customer has to check the code.
 * - `already_used`: a 409 answer; the coupon (or the per-user redemption
 *   limit for it) is already consumed and retrying will not help.
 * - `request_failed`: no server problem document at all (offline, timeout,
 *   auth gate), so nothing is known about the coupon yet.
 *
 * Classification is duck-typed against the generated-SDK error shape
 * (`httpStatus`, `traceId`, `problem.{status,code,i18nKey,traceId}`) so this
 * module stays free of SDK runtime imports and is directly unit-testable.
 */

/** Stable server i18n key for "coupon consumed, benefit not delivered". */
export const COUPON_BENEFIT_DELIVERY_FAILED_I18N_KEY =
  "orders.coupon_redemptions.benefit_delivery_failed";

export type CouponRedemptionFailureKind =
  | "benefit_delivery_failed"
  | "invalid_or_expired"
  | "already_used"
  | "request_failed";

export interface CouponRedemptionFailure {
  /** Customer-facing failure family. */
  kind: CouponRedemptionFailureKind;
  /** Whether re-submitting the same request (same idempotency key) can succeed. */
  retryable: boolean;
  /** Server trace id, when the problem document carried one. */
  traceId?: string;
  /** SDKWork problem `code` (numeric `503xx`/`4xxxx` on the wire). */
  code?: number | string;
  /** Server presentation key, when present. */
  i18nKey?: string;
  /** HTTP status, when the failure came back from the server. */
  status?: number;
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return undefined;
  }
  return value as Record<string, unknown>;
}

function asFiniteNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

/** Reads the platform problem code without mistaking an SDK error name for one. */
function readProblemCode(value: unknown): number | string | undefined {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === "string" && /^\d{3,6}$/u.test(value)) {
    return value;
  }
  return undefined;
}

function numericCode(code: number | string | undefined): number | undefined {
  if (typeof code === "number") {
    return code;
  }
  if (typeof code === "string") {
    const parsed = Number(code);
    return Number.isFinite(parsed) ? parsed : undefined;
  }
  return undefined;
}

/**
 * Classifies a thrown coupon-redemption failure.
 *
 * @param cause the rejection reason from the redemption service (generated
 *   SDK error, plain `{ status, code }` object, or anything else).
 */
export function classifyCouponRedemptionFailure(cause: unknown): CouponRedemptionFailure {
  const record = asRecord(cause);
  const problem = asRecord(record?.["problem"]);
  const i18nKey = typeof problem?.["i18nKey"] === "string" ? problem["i18nKey"] : undefined;
  const code = readProblemCode(problem?.["code"]) ?? readProblemCode(record?.["code"]);
  const status =
    asFiniteNumber(problem?.["status"]) ??
    asFiniteNumber(record?.["httpStatus"]) ??
    asFiniteNumber(record?.["status"]);
  const traceId =
    (typeof record?.["traceId"] === "string" ? record["traceId"] : undefined) ??
    (typeof problem?.["traceId"] === "string" ? problem["traceId"] : undefined);

  const failure = (kind: CouponRedemptionFailureKind, retryable: boolean): CouponRedemptionFailure => {
    const classified: CouponRedemptionFailure = { kind, retryable };
    if (traceId) {
      classified.traceId = traceId;
    }
    if (code !== undefined) {
      classified.code = code;
    }
    if (i18nKey) {
      classified.i18nKey = i18nKey;
    }
    if (status !== undefined) {
      classified.status = status;
    }
    return classified;
  };

  // The stable server marker always wins: it names the exact post-consumption state.
  if (i18nKey === COUPON_BENEFIT_DELIVERY_FAILED_I18N_KEY) {
    return failure("benefit_delivery_failed", true);
  }
  const numeric = numericCode(code);
  if (numeric !== undefined && numeric >= 50300 && numeric <= 50399) {
    return failure("benefit_delivery_failed", true);
  }
  if (status === 409) {
    return failure("already_used", false);
  }
  if (status === 400 || status === 404 || status === 422) {
    return failure("invalid_or_expired", false);
  }
  // 503 is the contract for the post-consumption delivery failure; 500 is the
  // legacy generic answer for the same state. Other 5xx answers (gateway
  // errors) carry no evidence that the coupon was consumed.
  if (status === 503 || status === 500) {
    return failure("benefit_delivery_failed", true);
  }
  if (status !== undefined && status >= 500) {
    return failure("request_failed", true);
  }
  // Offline, timeout, auth gate, or an unrecognized 4xx: nothing is known about
  // the coupon, so the safe advice is to retry the same idempotent request.
  return failure("request_failed", true);
}
