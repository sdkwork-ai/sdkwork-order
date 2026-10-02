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
        receipts: { create: vi.fn() },
        refundRequests: { list: vi.fn(), create: vi.fn() },
      },
    },
    orderFulfillments: { fulfillments: { list: vi.fn() } },
    orderShipments: {
      shipments: {
        retrieve: vi.fn(),
        trackingEvents: { list: vi.fn() },
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
      () => OrderService.confirmReceipt("order-id"),
      () => OrderService.listRefundRequests(),
      () =>
        OrderService.createRefundRequest({
          orderId: "order-id",
          targetAsset: "cash",
          amount: "10.00",
        }),
      () => OrderService.getOrderShipment("order-id"),
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

describe("receipt confirmation", () => {
  it("confirms receipt through the idempotent receipts port with an empty command body", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.receipts.create.mockResolvedValue({
      accepted: true,
      resourceId: "order-1",
    });
    await OrderService.confirmReceipt("order-1");
    expect(client.orderOrders.orders.receipts.create).toHaveBeenCalledWith(
      "order-1",
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
      // Empty command body keeps the request well-formed (40002 avoidance).
      {},
    );
  });

  it("propagates backend rejections (e.g. already confirmed)", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.receipts.create.mockRejectedValue(
      new Error("order is not fulfilled"),
    );
    await expect(OrderService.confirmReceipt("order-1")).rejects.toThrow(
      "order is not fulfilled",
    );
  });
});

describe("refund requests", () => {
  it("lists refund requests and reads pageInfo.hasMore", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.refundRequests.list.mockResolvedValue({
      items: [
        {
          accountValueRequestId: "req-1",
          requestNo: "RR20261001001",
          originalOrderId: "order-1",
          subject: "退款申请",
          targetAsset: "cash",
          amount: "1050",
          currencyCode: "CNY",
          status: "requested",
          createdAt: "2026-10-01T10:00:00Z",
          updatedAt: "2026-10-01T10:00:00Z",
        },
      ],
      pageInfo: { mode: "offset", page: 1, pageSize: 20, hasMore: true },
    });

    const result = await OrderService.listRefundRequests({ status: "requested" });
    expect(client.orderOrders.orders.refundRequests.list).toHaveBeenCalledWith({
      status: "requested",
      page: 1,
      pageSize: 20,
    });
    expect(result.hasMore).toBe(true);
    expect(result.items[0]).toMatchObject({
      requestId: "req-1",
      requestNo: "RR20261001001",
      originalOrderId: "order-1",
      targetAsset: "cash",
      amount: "1050",
      status: "requested",
    });
  });

  it("normalizes cash amounts from major units to minor-unit integers", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.refundRequests.create.mockResolvedValue({
      accountValueRequestId: "req-2",
      requestNo: "RR2",
      originalOrderId: "order-2",
      targetAsset: "cash",
      amount: "1050",
      currencyCode: "CNY",
      status: "requested",
      createdAt: "2026-10-01T10:00:00Z",
      updatedAt: "2026-10-01T10:00:00Z",
    });

    const view = await OrderService.createRefundRequest({
      orderId: "order-2",
      targetAsset: "cash",
      amount: "10.5",
      reasonDetail: " 商品与描述不符 ",
    });
    expect(client.orderOrders.orders.refundRequests.create).toHaveBeenCalledWith(
      {
        originalOrderId: "order-2",
        targetAsset: "cash",
        amount: "1050",
        currencyCode: "CNY",
        reasonCode: undefined,
        reasonDetail: "商品与描述不符",
      },
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
    );
    expect(view.requestId).toBe("req-2");
    expect(view.amount).toBe("1050");
  });

  it("passes positive-integer asset amounts through verbatim for points", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderOrders.orders.refundRequests.create.mockResolvedValue({
      accountValueRequestId: "req-3",
      targetAsset: "points",
      amount: "150",
      currencyCode: "CNY",
      status: "requested",
    });
    await OrderService.createRefundRequest({
      orderId: "order-3",
      targetAsset: "points",
      amount: " 150 ",
    });
    expect(client.orderOrders.orders.refundRequests.create).toHaveBeenCalledWith(
      expect.objectContaining({ amount: "150", targetAsset: "points" }),
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
    );
  });

  it("rejects invalid amounts before calling the API", async () => {
    const client = createMockClient();
    configureMockClient(client);
    await expect(
      OrderService.createRefundRequest({
        orderId: "order-1",
        targetAsset: "cash",
        amount: "abc",
      }),
    ).rejects.toThrow("请输入有效的退款金额");
    await expect(
      OrderService.createRefundRequest({
        orderId: "order-1",
        targetAsset: "points",
        amount: "1.5",
      }),
    ).rejects.toThrow("请输入有效的退款额度");
    await expect(
      OrderService.createRefundRequest({
        orderId: "order-1",
        targetAsset: "token_bank",
        amount: "0",
      }),
    ).rejects.toThrow("请输入有效的退款额度");
    expect(client.orderOrders.orders.refundRequests.create).not.toHaveBeenCalled();
  });

  it("rejects an empty original order id before calling the API", async () => {
    const client = createMockClient();
    configureMockClient(client);
    await expect(
      OrderService.createRefundRequest({
        orderId: " ",
        targetAsset: "cash",
        amount: "10.00",
      }),
    ).rejects.toThrow("original order id");
    expect(client.orderOrders.orders.refundRequests.create).not.toHaveBeenCalled();
  });
});

describe("order shipment assembly", () => {
  it("assembles fulfillment → shipment → tracking events", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderFulfillments.fulfillments.list.mockResolvedValue({
      items: [
        {
          fulfillmentId: "fulfillment-1",
          fulfillmentNo: "F20261001001",
          fulfillmentType: "physical_shipment",
          orderId: "order-1",
          status: "shipped",
        },
      ],
      pageInfo: { mode: "offset", page: 1, pageSize: 10 },
    });
    client.orderShipments.shipments.retrieve.mockResolvedValue({
      shipmentId: "shipment-1",
      shipmentNo: "S20261001001",
      fulfillmentId: "fulfillment-1",
      carrierCode: "sf",
      trackingNo: "SF123456789",
      status: "in_transit",
    });
    client.orderShipments.shipments.trackingEvents.list.mockResolvedValue({
      items: [
        {
          eventId: "event-1",
          shipmentId: "shipment-1",
          trackingEventNo: "TE-1",
          eventType: "pickup",
          eventTime: "2026-10-01T08:00:00Z",
          locationText: "杭州转运中心",
        },
        {
          eventId: "event-2",
          shipmentId: "shipment-1",
          trackingEventNo: "TE-2",
          eventType: "in_transit",
          eventStatus: "delivered",
          eventTime: "2026-10-02T09:30:00Z",
        },
      ],
      pageInfo: { mode: "offset", page: 1, pageSize: 50 },
    });

    const summary = await OrderService.getOrderShipment("order-1");
    expect(client.orderFulfillments.fulfillments.list).toHaveBeenCalledWith({
      orderId: "order-1",
      page: 1,
      pageSize: 10,
    });
    expect(client.orderShipments.shipments.retrieve).toHaveBeenCalledWith("fulfillment-1");
    expect(client.orderShipments.shipments.trackingEvents.list).toHaveBeenCalledWith("shipment-1", {
      page: 1,
      pageSize: 50,
    });
    expect(summary).toMatchObject({
      fulfillmentId: "fulfillment-1",
      fulfillmentNo: "F20261001001",
      shipmentId: "shipment-1",
      shipmentNo: "S20261001001",
      carrierCode: "sf",
      trackingNo: "SF123456789",
      status: "in_transit",
    });
    expect(summary?.events).toHaveLength(2);
    expect(summary?.events[0]).toMatchObject({ eventType: "pickup", locationText: "杭州转运中心" });
    expect(summary?.events[1].eventStatus).toBe("delivered");
  });

  it("returns null when the order has no fulfillment yet", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderFulfillments.fulfillments.list.mockResolvedValue({ items: [], pageInfo: {} });
    await expect(OrderService.getOrderShipment("order-1")).resolves.toBeNull();
    expect(client.orderShipments.shipments.retrieve).not.toHaveBeenCalled();
  });

  it("keeps the fulfillment header when the shipment read misses", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderFulfillments.fulfillments.list.mockResolvedValue({
      items: [
        {
          fulfillmentId: "fulfillment-2",
          fulfillmentNo: "F2",
          fulfillmentType: "physical_shipment",
          orderId: "order-2",
          status: "awaiting_shipment",
        },
      ],
      pageInfo: {},
    });
    client.orderShipments.shipments.retrieve.mockRejectedValue(new Error("shipment was not found"));

    const summary = await OrderService.getOrderShipment("order-2");
    expect(summary).toMatchObject({
      fulfillmentId: "fulfillment-2",
      fulfillmentNo: "F2",
      fulfillmentStatus: "awaiting_shipment",
      status: "awaiting_shipment",
      carrierCode: "",
    });
    expect(summary?.shipmentId).toBeUndefined();
    expect(summary?.events).toHaveLength(0);
    expect(client.orderShipments.shipments.trackingEvents.list).not.toHaveBeenCalled();
  });

  it("keeps the shipment header when the tracking events read fails", async () => {
    const client = createMockClient();
    configureMockClient(client);
    client.orderFulfillments.fulfillments.list.mockResolvedValue({
      items: [
        {
          fulfillmentId: "fulfillment-3",
          fulfillmentNo: "F3",
          fulfillmentType: "physical_shipment",
          orderId: "order-3",
          status: "shipped",
        },
      ],
      pageInfo: {},
    });
    client.orderShipments.shipments.retrieve.mockResolvedValue({
      shipmentId: "shipment-3",
      shipmentNo: "S3",
      fulfillmentId: "fulfillment-3",
      carrierCode: "yt",
      status: "in_transit",
    });
    client.orderShipments.shipments.trackingEvents.list.mockRejectedValue(new Error("events unavailable"));

    const summary = await OrderService.getOrderShipment("order-3");
    expect(summary).toMatchObject({
      shipmentId: "shipment-3",
      shipmentNo: "S3",
      carrierCode: "yt",
      status: "in_transit",
    });
    expect(summary?.events).toHaveLength(0);
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
