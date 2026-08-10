import { afterEach, describe, expect, it, vi } from "vitest";

import type { SdkworkAppClient } from "@sdkwork/order-app-sdk";

import {
  configureOrderMobileRuntime,
  formatAmountCny,
  OrderCapabilityUnavailableError,
  OrderService,
  paymentMethodsForEnvironment,
  resetOrderMobileRuntime,
  toOrderListStatusWire,
  toOrderStatusWire,
} from "./OrderService";

function createMockClient() {
  return {
    orderOrders: {
      orders: {
        list: vi.fn(),
        retrieve: vi.fn(),
        statistics: { retrieve: vi.fn() },
        payments: { create: vi.fn() },
        paymentSuccess: { retrieve: vi.fn() },
        cancellations: { create: vi.fn() },
        couponRedemptions: { create: vi.fn() },
      },
    },
    orderCheckout: {
      checkout: {
        sessions: {
          create: vi.fn(),
          retrieve: vi.fn(),
          quotes: { create: vi.fn() },
          orders: { create: vi.fn() },
        },
      },
    },
  };
}

type MockClient = ReturnType<typeof createMockClient>;

function configureMockClient(client: MockClient) {
  configureOrderMobileRuntime({ client: client as unknown as SdkworkAppClient });
}

afterEach(() => {
  resetOrderMobileRuntime();
});

describe("fail-closed runtime", () => {
  it("rejects every operation when the Order SDK runtime is not composed", async () => {
    for (const operation of [
      () => OrderService.getOrders(),
      () => OrderService.getOrderById("order-id"),
      () => OrderService.getOrderStatistics(),
      () => OrderService.payOrder("order-id", "wechat_pay"),
      () => OrderService.getPaymentStatus("order-id"),
      () => OrderService.cancelOrder("order-id"),
      () => OrderService.redeemVoucher("voucher-code"),
      () =>
        OrderService.createOrder({
          items: [{ quantity: 1, skuId: "sku-1" }],
          shippingAddress: {
            receiverName: "Zhang",
            receiverPhone: "13800000000",
            countryCode: "CN",
            province: "Zhejiang",
            city: "Hangzhou",
            detailAddress: "Street 1",
          },
        }),
    ]) {
      await expect(operation()).rejects.toBeInstanceOf(OrderCapabilityUnavailableError);
    }
  });
});

describe("order tabs and list", () => {
  it("returns the canonical backend-aligned tabs", async () => {
    const client = createMockClient();
    configureMockClient(client);
    const tabs = await OrderService.getOrderTabs();
    expect(tabs.map((tab) => tab.id)).toEqual([
      "all",
      "pending_payment",
      "paid",
      "fulfilled",
      "completed",
      "cancelled",
    ]);
  });

  it("maps list items and passes the tab status filter", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.list.mockResolvedValue({
      items: [
        {
          orderId: "order-1",
          orderSn: "SW202608010001",
          status: "pending_payment",
          statusName: "Pending payment",
          subject: "积分充值",
          totalAmount: "6990",
          paidAmount: null,
          discountAmount: "0",
          quantity: 1,
          createdAt: "2026-08-01T10:00:00Z",
          expireTime: "2026-08-01T10:30:00Z",
          paymentMethod: "wechat_pay",
          items: [
            { id: "item-1", productName: "积分包", quantity: 1, unitPrice: "6990", totalAmount: "6990" },
          ],
        },
      ],
      pageInfo: { page: 1, pageSize: 50, totalItems: 1 },
    });

    const orders = await OrderService.getOrders("pending_payment");
    expect(client.orderOrders.orders.list).toHaveBeenCalledWith({
      status: "pending_payment",
      page: 1,
      pageSize: 50,
    });
    expect(orders).toHaveLength(1);
    expect(orders[0]).toMatchObject({
      id: "order-1",
      orderSn: "SW202608010001",
      status: "pending_payment",
      statusText: "Pending payment",
      subject: "积分充值",
      totalAmount: "6990",
      quantity: 1,
    });
    expect(orders[0].items[0]).toMatchObject({ title: "积分包", unitPrice: "6990" });
  });

  it("omits the status filter for the all tab", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.list.mockResolvedValue({ items: [], pageInfo: {} });
    await OrderService.getOrders("all");
    expect(client.orderOrders.orders.list).toHaveBeenCalledWith({
      status: undefined,
      page: 1,
      pageSize: 50,
    });
  });
});

describe("order detail and statistics", () => {
  it("maps the detail read model", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.retrieve.mockResolvedValue({
      orderId: "order-2",
      orderSn: "SW202608010002",
      status: "paid",
      statusName: "Paid",
      subject: "实物订单",
      totalAmount: "12000",
      paidAmount: "12000",
      discountAmount: "0",
      quantity: 2,
      createdAt: "2026-08-01T09:00:00Z",
      payTime: "2026-08-01T09:05:00Z",
      items: [
        { id: "i1", productName: "商品A", quantity: 2, unitPrice: "6000", totalAmount: "12000" },
      ],
      outTradeNo: "20260801090000001",
      transactionId: "txn-1",
    });

    const order = await OrderService.getOrderById("order-2");
    expect(order?.paidAmount).toBe("12000");
    expect(order?.payTime).toBe("2026-08-01T09:05:00Z");
    expect(order?.outTradeNo).toBe("20260801090000001");
  });

  it("returns null when the order does not exist", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.retrieve.mockResolvedValue(null);
    await expect(OrderService.getOrderById("missing")).resolves.toBeNull();
  });

  it("maps statistics counts", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.statistics.retrieve.mockResolvedValue({
      totalOrders: 10,
      pendingPayment: 2,
      pendingShipment: 3,
      pendingReceipt: 1,
      completed: 4,
      totalAmount: "100000",
    });
    await expect(OrderService.getOrderStatistics()).resolves.toEqual({
      totalOrders: 10,
      pendingPayment: 2,
      pendingShipment: 3,
      pendingReceipt: 1,
      completed: 4,
      totalAmount: "100000",
    });
  });
});

describe("cashier payment flow", () => {
  it("creates a payment session and maps payment params", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.payments.create.mockResolvedValue({
      amount: "6990",
      orderId: "order-1",
      outTradeNo: "202608010001",
      paymentId: "pay-1",
      paymentMethod: "wechat_pay",
      paymentParams: {
        cashierUrl: "https://im.sdkwork.com/cashier/order-1?scene=recharge&outTradeNo=202608010001",
        qrCodePayload: "https://im.sdkwork.com/cashier/order-1?scene=recharge&outTradeNo=202608010001",
        nextAction: "cashier",
        orderSn: "SW1",
        cashierScene: "recharge",
      },
    });

    const session = await OrderService.payOrder("order-1", "alipay");
    expect(client.orderOrders.orders.payments.create).toHaveBeenCalledWith(
      "order-1",
      { paymentMethod: "alipay" },
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
    );
    expect(session).toMatchObject({
      amount: "6990",
      orderId: "order-1",
      outTradeNo: "202608010001",
      paymentId: "pay-1",
      paymentMethod: "wechat_pay",
    });
    expect(session.paymentParams.qrCodePayload).toContain("scene=recharge");
  });

  it("rejects unsupported payment methods before calling the API", async () => {
    const client = createMockClient();
    configureMockClient(client);
    await expect(
      OrderService.payOrder("order-1", "cash" as "wechat_pay"),
    ).rejects.toThrow("Unsupported payment method");
    expect(client.orderOrders.orders.payments.create).not.toHaveBeenCalled();
  });

  it("passes the payer openid through for wechat_jsapi", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.payments.create.mockResolvedValue({
      amount: "6990",
      orderId: "order-1",
      outTradeNo: "202608010001",
      paymentId: "pay-1",
      paymentMethod: "wechat_jsapi",
      paymentParams: {
        jsapiPayload: JSON.stringify({ appId: "wxappid", timeStamp: "1" }),
        nextAction: "jsapi",
      },
    });

    const session = await OrderService.payOrder("order-1", "wechat_jsapi", {
      openid: "o_payer",
    });
    expect(client.orderOrders.orders.payments.create).toHaveBeenCalledWith(
      "order-1",
      { paymentMethod: "wechat_jsapi", openid: "o_payer" },
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
    );
    expect(session.paymentMethod).toBe("wechat_jsapi");
  });

  it("omits the openid field when not provided", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.payments.create.mockResolvedValue({
      amount: "6990",
      orderId: "order-1",
      outTradeNo: "202608010001",
      paymentId: "pay-1",
      paymentMethod: "alipay_wap",
      paymentParams: { payUrl: "https://cashier.alipay.com/example", nextAction: "redirect" },
    });
    await OrderService.payOrder("order-1", "alipay_wap");
    expect(client.orderOrders.orders.payments.create).toHaveBeenCalledWith(
      "order-1",
      { paymentMethod: "alipay_wap" },
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
    );
  });

  it("reads the payment success status", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.paymentSuccess.retrieve.mockResolvedValue({
      paid: true,
      status: "paid",
      statusName: "Paid",
    });
    await expect(OrderService.getPaymentStatus("order-1")).resolves.toEqual({
      paid: true,
      status: "paid",
      statusName: "Paid",
    });
  });

  it("cancels an order through the cancellations port", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.cancellations.create.mockResolvedValue({
      accepted: true,
      resourceId: "order-1",
    });
    await OrderService.cancelOrder("order-1");
    expect(client.orderOrders.orders.cancellations.create).toHaveBeenCalledWith(
      "order-1",
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
      // Empty command body keeps the request well-formed; the HTTP layer would
      // otherwise send an empty JSON body that the server rejects (40002).
      {},
    );
  });
});

describe("voucher redemption", () => {
  it("maps a completed redemption to success", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.couponRedemptions.create.mockResolvedValue({
      benefitKind: "token_bank_credit",
      grantAmount: 1000,
      orderId: "order-3",
      orderNo: "SW3",
      replayed: false,
      status: "completed",
      targetAsset: "token_bank",
    });
    await expect(OrderService.redeemVoucher("abc123")).resolves.toEqual({
      success: true,
      message: "核销成功",
      orderId: "order-3",
      orderNo: "SW3",
    });
    expect(client.orderOrders.orders.couponRedemptions.create).toHaveBeenCalledWith(
      { couponCode: "ABC123" },
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
    );
  });

  it("reports failure without throwing when redemption is rejected", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.couponRedemptions.create.mockRejectedValue(
      new Error("coupon code is invalid"),
    );
    const result = await OrderService.redeemVoucher("invalid");
    expect(result.success).toBe(false);
    expect(result.message).toContain("coupon code is invalid");
  });
});

describe("checkout session order creation", () => {
  it("runs the canonical three-step checkout flow", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderCheckout.checkout.sessions.create.mockResolvedValue({
      checkoutSessionId: "session-1",
      status: "open",
      currencyCode: "CNY",
      originalAmount: "12000",
      discountAmount: "0",
      payableAmount: "12000",
    });
    client.orderCheckout.checkout.sessions.quotes.create.mockResolvedValue({
      checkoutSessionId: "session-1",
      quoteId: "quote-1",
      currencyCode: "CNY",
      originalAmount: "12000",
      discountAmount: "0",
      payableAmount: "12000",
    });
    client.orderCheckout.checkout.sessions.orders.create.mockResolvedValue({
      orderId: "order-4",
      orderNo: "SW202608010004",
      orderSn: "SW4",
      status: "pending_payment",
      totalAmount: "12000",
    });
    client.orderOrders.orders.retrieve.mockResolvedValue({
      orderId: "order-4",
      orderSn: "SW4",
      status: "pending_payment",
      statusName: "Pending payment",
      subject: "实物订单",
      totalAmount: "12000",
      quantity: 2,
      createdAt: "2026-08-01T08:00:00Z",
      items: [],
    });

    const order = await OrderService.createOrder({
      currencyCode: "CNY",
      items: [{ quantity: 2, skuId: "sku-a" }],
      shippingAddress: {
        receiverName: "Zhang",
        receiverPhone: "13800000000",
        countryCode: "CN",
        province: "Zhejiang",
        city: "Hangzhou",
        detailAddress: "Street 1",
      },
    });

    expect(client.orderCheckout.checkout.sessions.create).toHaveBeenCalledWith(
      {
        currencyCode: "CNY",
        items: [{ quantity: "2", skuId: "sku-a" }],
        shippingAddress: expect.objectContaining({ receiverName: "Zhang" }),
      },
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
    );
    expect(client.orderCheckout.checkout.sessions.quotes.create).toHaveBeenCalledWith(
      "session-1",
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
    );
    expect(client.orderCheckout.checkout.sessions.orders.create).toHaveBeenCalledWith(
      "session-1",
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
    );
    expect(order.id).toBe("order-4");
  });
});

describe("wire mapping helpers", () => {
  it("maps legacy backend statuses onto the mobile union", () => {
    expect(toOrderStatusWire("unpaid")).toBe("pending_payment");
    expect(toOrderStatusWire("wait_pay")).toBe("pending_payment");
    expect(toOrderStatusWire("canceled")).toBe("cancelled");
    expect(toOrderStatusWire("closed")).toBe("cancelled");
    expect(toOrderStatusWire("timeout")).toBe("expired");
    expect(toOrderStatusWire("shipped")).toBe("fulfilled");
    expect(toOrderStatusWire("finished")).toBe("completed");
    expect(toOrderStatusWire("pending_payment")).toBe("pending_payment");
    expect(toOrderStatusWire("refunding")).toBe("refunding");
  });

  it("normalizes tab ids to backend status query values", () => {
    expect(toOrderListStatusWire("all")).toBeUndefined();
    expect(toOrderListStatusWire("pending_payment")).toBe("pending_payment");
    expect(toOrderListStatusWire("PENDING-PAYMENT")).toBe("pending_payment");
    expect(toOrderListStatusWire("")).toBeUndefined();
  });

  it("formats minor-unit amounts as CNY display strings", () => {
    expect(formatAmountCny("6990")).toBe("¥69.90");
    expect(formatAmountCny(12000)).toBe("¥120.00");
    expect(formatAmountCny("0")).toBe("¥0.00");
    expect(formatAmountCny(undefined)).toBe("--");
    expect(formatAmountCny("not-an-amount")).toBe("not-an-amount");
    expect(formatAmountCny("5000", "JPY")).toBe("¥5,000");
  });
});

describe("paymentMethodsForEnvironment", () => {
  it("narrows to alipay inside the Alipay app", () => {
    expect(paymentMethodsForEnvironment("alipay")).toEqual(["alipay"]);
  });

  it("narrows to wechat inside the WeChat app", () => {
    expect(paymentMethodsForEnvironment("wechat")).toEqual(["wechat_pay"]);
  });

  it("offers the full list in a browser", () => {
    expect(paymentMethodsForEnvironment("browser")).toEqual([
      "wechat_pay",
      "alipay",
      "balance",
    ]);
  });
});
