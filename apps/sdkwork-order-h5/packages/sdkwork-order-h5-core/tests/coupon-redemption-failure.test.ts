import { describe, expect, it } from "vitest";

import {
  COUPON_BENEFIT_DELIVERY_FAILED_I18N_KEY,
  classifyCouponRedemptionFailure,
} from "../src/sdk/couponRedemptionFailure";

function generatedSdkError(overrides: Record<string, unknown> = {}) {
  return {
    name: "ServiceUnavailableError",
    message: "HTTP 503: Service Unavailable",
    code: "SERVICE_UNAVAILABLE",
    httpStatus: 503,
    traceId: "trace-503",
    problem: {
      status: 503,
      code: 50301,
      traceId: "trace-503",
      i18nKey: COUPON_BENEFIT_DELIVERY_FAILED_I18N_KEY,
      detail: "An internal error occurred",
    },
    ...overrides,
  };
}

describe("classifyCouponRedemptionFailure", () => {
  it("classifies the 503 benefit-delivery failure as retryable", () => {
    const failure = classifyCouponRedemptionFailure(generatedSdkError());

    expect(failure.kind).toBe("benefit_delivery_failed");
    expect(failure.retryable).toBe(true);
    expect(failure.traceId).toBe("trace-503");
    expect(failure.code).toBe(50301);
    expect(failure.i18nKey).toBe(COUPON_BENEFIT_DELIVERY_FAILED_I18N_KEY);
    expect(failure.status).toBe(503);
  });

  it("classifies the legacy generic 500/50001 fulfillment failure as retryable", () => {
    const failure = classifyCouponRedemptionFailure({
      httpStatus: 500,
      traceId: "trace-legacy",
      problem: { status: 500, code: 50001, traceId: "trace-legacy" },
    });

    expect(failure.kind).toBe("benefit_delivery_failed");
    expect(failure.retryable).toBe(true);
    expect(failure.traceId).toBe("trace-legacy");
  });

  it("classifies a 503 without the i18n metadata as retryable", () => {
    expect(classifyCouponRedemptionFailure({ httpStatus: 503 }).kind).toBe(
      "benefit_delivery_failed",
    );
  });

  it("does not claim the coupon was consumed for a gateway 5xx", () => {
    const failure = classifyCouponRedemptionFailure({ httpStatus: 504 });

    expect(failure.kind).toBe("request_failed");
    expect(failure.retryable).toBe(true);
  });

  it("classifies 400/404/422 as an invalid or expired code", () => {
    for (const status of [400, 404, 422]) {
      const failure = classifyCouponRedemptionFailure({ httpStatus: status });
      expect(failure.kind).toBe("invalid_or_expired");
      expect(failure.retryable).toBe(false);
    }
  });

  it("classifies 409 as an already-used coupon", () => {
    const failure = classifyCouponRedemptionFailure({
      httpStatus: 409,
      problem: { status: 409, code: 40901, detail: "coupon redemption preview was rejected" },
    });

    expect(failure.kind).toBe("already_used");
    expect(failure.retryable).toBe(false);
    expect(failure.code).toBe(40901);
  });

  it("falls back to a retryable request failure without a problem document", () => {
    for (const cause of [new TypeError("Failed to fetch"), { code: "TIMEOUT" }, undefined]) {
      const failure = classifyCouponRedemptionFailure(cause);
      expect(failure.kind).toBe("request_failed");
      expect(failure.retryable).toBe(true);
    }
  });

  it("never mistypes the SDK error-name code as a platform code", () => {
    const failure = classifyCouponRedemptionFailure(
      generatedSdkError({ problem: { status: 503 } }),
    );

    expect(failure.code).toBeUndefined();
  });
});
