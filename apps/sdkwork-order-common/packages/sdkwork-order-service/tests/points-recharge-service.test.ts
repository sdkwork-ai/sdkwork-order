import { describe, expect, it, vi } from "vitest";
import {
  createSdkworkPointsRechargeService,
  createSdkworkCouponRechargeService,
  configureSdkworkOrderSessionTokenProvider,
  type SdkworkOrderAppService,
} from "../src/index.ts";

function createAppService(overrides: {
  packages?: unknown;
  create?: unknown;
  retrieve?: unknown;
} = {}): { appService: SdkworkOrderAppService; create: ReturnType<typeof vi.fn> } {
  const create = vi.fn().mockResolvedValue(overrides.create ?? {
    item: {
      amount: "90",
      cashierUrl: "http://127.0.0.1:3901/cashier/recharge-order-900",
      expiresAt: "2026-07-27T04:30:00Z",
      orderId: "order-900",
      paymentProduct: "mobile_cashier_h5",
      points: 900,
      qrCodePayload: "weixin://pay/order-900",
      status: "pending",
    },
  });
  return {
    appService: {
      checkout: {} as SdkworkOrderAppService["checkout"],
      memberships: {} as SdkworkOrderAppService["memberships"],
      orders: {
        couponRedemptions: { create },
      } as unknown as SdkworkOrderAppService["orders"],
      shipments: {} as SdkworkOrderAppService["shipments"],
      recharges: {
        plans: { list: vi.fn() },
        packages: {
          list: vi.fn().mockResolvedValue(overrides.packages ?? {
            items: [
              { id: "recharge-500", priceAmount: "50", currencyCode: "CNY", points: 500 },
              { id: "recharge-900", priceAmount: "90", currencyCode: "CNY", points: 900 },
            ],
          }),
        },
        orders: {
          create,
          retrieve: vi.fn().mockResolvedValue(overrides.retrieve ?? {
            item: { orderId: "order-1", points: 500, status: "paid" },
          }),
          list: vi.fn(),
          cancel: vi.fn(),
        },
        settings: { retrieve: vi.fn() },
      },
      withdrawals: {} as SdkworkOrderAppService["withdrawals"],
    },
    create,
  };
}

describe("createSdkworkPointsRechargeService", () => {
  it("resolves the selected package and creates the canonical points recharge order", async () => {
    const { appService, create } = createAppService();
    const service = createSdkworkPointsRechargeService({ appService });

    await expect(service.createOrder({ packageId: "recharge-900" })).resolves.toEqual({
      amountCny: 90,
      cashierUrl: "http://127.0.0.1:3901/cashier/recharge-order-900",
      expiresAt: "2026-07-27T04:30:00Z",
      orderId: "order-900",
      orderNo: undefined,
      points: 900,
      qrCode: "http://127.0.0.1:3901/cashier/recharge-order-900",
      status: "pending",
    });
    expect(create).toHaveBeenCalledWith(
      {
        amount: 90,
        currencyCode: "CNY",
        packageId: "recharge-900",
        paymentMethod: "wechat_pay",
        paymentProduct: "mobile_cashier_h5",
        source: "membership-token-plan",
        subject: "points_recharge",
        targetAsset: "points",
      },
      expect.objectContaining({
        idempotencyKey: expect.any(String),
      }),
    );
  });

  it("rejects an unavailable package before creating an order", async () => {
    const { appService, create } = createAppService();
    const service = createSdkworkPointsRechargeService({ appService });

    await expect(service.createOrder({ packageId: "missing" })).rejects.toThrow(
      "selected recharge package is unavailable",
    );
    expect(create).not.toHaveBeenCalled();
  });

  it("maps paid checkout status to completed", async () => {
    const { appService } = createAppService();
    const service = createSdkworkPointsRechargeService({ appService });

    await expect(service.getOrderStatus("order-1")).resolves.toEqual(
      expect.objectContaining({ orderId: "order-1", status: "completed" }),
    );
  });
});

describe("createSdkworkCouponRechargeService", () => {
  it("sends only the coupon code to the dedicated redemption API", async () => {
    const { appService, create } = createAppService({
      create: {
        item: {
          benefit: {
            kind: "token_bank_credit",
            targetAsset: "token_bank",
            grantAmount: "50",
          },
          orderId: "order-coupon-1",
          orderNo: "CP1001",
          status: "fulfilled",
          targetAsset: "token_bank",
        },
      },
    });
    configureSdkworkOrderSessionTokenProvider(() => ({ accessToken: "session-token" }));
    const service = createSdkworkCouponRechargeService({ appService });

    await expect(service.redeem("  WELCOME  ")).resolves.toEqual({
      benefitKind: "token_bank_credit",
      grantAmount: 50,
      orderId: "order-coupon-1",
      orderNo: "CP1001",
      replayed: false,
      status: "completed",
      targetAsset: "token_bank",
    });
    expect(create).toHaveBeenCalledWith(
      {
        couponCode: "WELCOME",
      },
      expect.objectContaining({
        idempotencyKey: expect.any(String),
      }),
    );
    expect(Object.keys(create.mock.calls[0]?.[0] as object)).toEqual(["couponCode"]);
    configureSdkworkOrderSessionTokenProvider(null);
  });

  it("maps a points credit redemption", async () => {
    const { appService } = createAppService({
      create: {
        item: {
          benefit: {
            kind: "points_credit",
            grantPoints: "1000",
          },
          orderId: "order-coupon-3",
          orderNo: "CP1003",
          status: "fulfilled",
          replayed: false,
        },
      },
    });
    configureSdkworkOrderSessionTokenProvider(() => ({ accessToken: "session-token" }));
    const service = createSdkworkCouponRechargeService({ appService });

    await expect(service.redeem("POINTS-1000")).resolves.toEqual({
      benefitKind: "points_credit",
      grantPoints: 1000,
      orderId: "order-coupon-3",
      orderNo: "CP1003",
      replayed: false,
      status: "completed",
    });
    configureSdkworkOrderSessionTokenProvider(null);
  });

  it("maps a cash credit redemption in minor units", async () => {
    const { appService } = createAppService({
      create: {
        item: {
          benefit: {
            kind: "cash_credit",
            grantAmount: "10050",
          },
          orderId: "order-coupon-4",
          orderNo: "CP1004",
          status: "fulfilled",
          replayed: true,
        },
      },
    });
    configureSdkworkOrderSessionTokenProvider(() => ({ accessToken: "session-token" }));
    const service = createSdkworkCouponRechargeService({ appService });

    await expect(service.redeem("CASH-100")).resolves.toEqual({
      benefitKind: "cash_credit",
      grantAmount: 10050,
      orderId: "order-coupon-4",
      orderNo: "CP1004",
      replayed: true,
      status: "completed",
    });
    configureSdkworkOrderSessionTokenProvider(null);
  });

  it("maps a quota-limited subscription redemption", async () => {
    const { appService } = createAppService({
      create: {
        item: {
          benefit: {
            kind: "subscription",
            productId: "seed-product-membership",
            skuId: "sku-standard-monthly",
            packageId: "1002",
            period: "month",
            durationDays: "30",
            dailyQuota: "1000",
            totalQuota: "30000",
            subscriptionId: "subscription-coupon-1",
            startsAt: "2026-07-26 00:00:00",
            expiresAt: "2026-08-25 00:00:00",
          },
          orderId: "order-coupon-2",
          orderNo: "CP1002",
          status: "active",
          replayed: false,
        },
      },
    });
    configureSdkworkOrderSessionTokenProvider(() => ({ accessToken: "session-token" }));
    const service = createSdkworkCouponRechargeService({ appService });

    await expect(service.redeem("SUB-MONTH")).resolves.toEqual({
      benefitKind: "subscription",
      dailyQuota: 1000,
      durationDays: 30,
      expiresAt: "2026-08-25 00:00:00",
      orderId: "order-coupon-2",
      orderNo: "CP1002",
      packageId: "1002",
      period: "month",
      productId: "seed-product-membership",
      replayed: false,
      skuId: "sku-standard-monthly",
      startsAt: "2026-07-26 00:00:00",
      status: "pending",
      subscriptionId: "subscription-coupon-1",
      totalQuota: 30000,
    });
    configureSdkworkOrderSessionTokenProvider(null);
  });
});
