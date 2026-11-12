import { afterEach, describe, expect, it, vi } from "vitest";
import {
  applySdkworkOrderDiscount,
  configureSdkworkOrderAppServiceProvider,
  configureSdkworkOrderSessionTokenProvider,
  type SdkworkOrderAppService,
} from "../src/index.ts";

function configureDiscountProvider(create: ReturnType<typeof vi.fn>) {
  configureSdkworkOrderSessionTokenProvider(() => ({ accessToken: "test-token" }));
  configureSdkworkOrderAppServiceProvider(
    () =>
      ({
        orders: { discount: { create } },
      }) as unknown as SdkworkOrderAppService,
  );
}

afterEach(() => {
  configureSdkworkOrderAppServiceProvider(null);
  configureSdkworkOrderSessionTokenProvider(null);
});

describe("applySdkworkOrderDiscount", () => {
  it("calls the order discount API and normalizes the minor-unit amounts", async () => {
    const create = vi.fn().mockResolvedValue({
      item: {
        orderId: "order-1",
        originalAmount: "10000",
        discountAmount: "3000",
        payableAmount: "7000",
        currencyCode: "CNY",
      },
    });
    configureDiscountProvider(create);

    const result = await applySdkworkOrderDiscount({
      orderId: " order-1 ",
      userCouponId: "coupon-1",
      idempotencyKey: "discount-key",
    });

    expect(create).toHaveBeenCalledWith(
      "order-1",
      { userCouponId: "coupon-1" },
      { idempotencyKey: "discount-key" },
    );
    expect(result).toEqual({
      orderId: "order-1",
      originalAmount: "10000",
      discountAmount: "3000",
      payableAmount: "7000",
      currencyCode: "CNY",
    });
  });

  it("rejects a missing coupon before calling the API", async () => {
    const create = vi.fn();
    configureDiscountProvider(create);
    await expect(
      applySdkworkOrderDiscount({ orderId: "order-1", userCouponId: "  " }),
    ).rejects.toThrow("Order id and user coupon id are required.");
    expect(create).not.toHaveBeenCalled();
  });
});
