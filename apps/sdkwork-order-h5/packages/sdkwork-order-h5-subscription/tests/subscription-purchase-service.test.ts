import { describe, expect, it, vi } from "vitest";

import { createSubscriptionPurchaseService } from "../src/services/SubscriptionPurchaseService";
import type { SdkworkOrderAppService } from "@sdkwork/order-service";

function appServiceStub(overrides: Record<string, unknown> = {}) {
  return {
    recharges: {
      orders: {
        create: vi.fn().mockResolvedValue({
          code: 0,
          data: {
            item: {
              id: "recharge-order-1",
              orderNo: "TB-0001",
              status: "pending",
              cashierUrl: "https://cashier.example/pay",
            },
          },
          traceId: "t1",
        }),
        retrieve: vi.fn().mockResolvedValue({
          code: 0,
          data: { item: { id: "recharge-order-1", status: "succeeded" } },
          traceId: "t2",
        }),
      },
    },
    orders: {
      couponRedemptions: {
        create: vi.fn().mockResolvedValue({
          code: 0,
          data: {
            item: {
              id: "coupon-order-1",
              status: "succeeded",
              benefit: { kind: "token_bank", grantAmount: "100" },
            },
          },
          traceId: "t3",
        }),
      },
      paymentSuccess: {
        retrieve: vi.fn().mockResolvedValue({
          code: 0,
          data: {
            item: { id: "sub-order-1", status: "completed", amount: 199, durationDays: 30 },
          },
          traceId: "t4",
        }),
      },
    },
    memberships: {
      orders: {
        create: vi.fn().mockResolvedValue({
          code: 0,
          data: { item: { id: "sub-order-1", status: "pending" } },
          traceId: "t5",
        }),
      },
    },
    ...overrides,
  } as unknown as SdkworkOrderAppService;
}

describe("subscription purchase service", () => {
  it("unwraps the SdkWork envelope for coupon redemption and maps benefits", async () => {
    const appService = appServiceStub();
    const service = createSubscriptionPurchaseService({ appService });

    const result = await service.redeemCoupon(" SAVE-100 ");

    expect(appService.orders.couponRedemptions.create).toHaveBeenCalledTimes(1);
    const [body, params] = vi.mocked(appService.orders.couponRedemptions.create).mock.calls[0];
    expect(body).toEqual({ couponCode: "SAVE-100" });
    expect(params).toHaveProperty("idempotencyKey");
    expect(result).toMatchObject({
      orderId: "coupon-order-1",
      status: "completed",
      benefitKind: "token_bank",
      grantAmount: "100",
    });
  });

  it("rejects an empty coupon code before any network call", async () => {
    const appService = appServiceStub();
    const service = createSubscriptionPurchaseService({ appService });

    await expect(service.redeemCoupon("   ")).rejects.toThrow("A coupon code is required.");
    expect(appService.orders.couponRedemptions.create).not.toHaveBeenCalled();
  });

  it("creates a Token Bank recharge order for a known plan and unwraps the cashier payload", async () => {
    const appService = appServiceStub();
    const catalog = {
      listTokenBankPlans: vi.fn().mockResolvedValue([
        {
          planCode: "tb-100",
          priceAmount: "10000",
          currencyCode: "CNY",
          grantAmount: "100000000",
        },
      ]),
      listMembershipPackages: vi.fn(),
      listMembershipPackageGroups: vi.fn(),
    };
    const service = createSubscriptionPurchaseService({
      appService,
      catalog: catalog as unknown as import("../src/services/SubscriptionCatalogPort").SubscriptionCatalogPort,
    });

    const payment = await service.createRechargeOrder("tb-100", "wechat_pay");

    const [body] = vi.mocked(appService.recharges.orders.create).mock.calls[0];
    expect(body).toMatchObject({
      subject: "points_recharge",
      targetAsset: "points",
      packageId: "tb-100",
      paymentProduct: "mobile_cashier_h5",
    });
    expect(payment.orderId).toBe("recharge-order-1");
    expect(payment.cashierUrl).toBe("https://cashier.example/pay");
    expect(payment.status).toBe("pending");
  });

  it("maps subscription checkout status through the envelope to completed", async () => {
    const appService = appServiceStub();
    const service = createSubscriptionPurchaseService({ appService });

    const payment = await service.getSubscriptionStatus("sub-order-1");

    expect(appService.orders.paymentSuccess.retrieve).toHaveBeenCalledWith("sub-order-1");
    expect(payment.orderId).toBe("sub-order-1");
    expect(payment.status).toBe("completed");
    expect(payment.amountCny).toBe(199);
    expect(payment.durationDays).toBe(30);
  });
});
