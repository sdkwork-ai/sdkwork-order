import { describe, expect, it, vi } from "vitest";
import { createOrderAdminService } from "../src/order-admin-service";

describe("createOrderAdminService", () => {
  it("lists orders with v3 envelope unwrapping", async () => {
    const client = {
      orders: {
        admin: {
          list: vi.fn().mockResolvedValue({
            code: 0,
            data: {
              items: [{ orderId: "o-1", subject: "Test", status: "pending_payment" }],
              pageInfo: { mode: "offset", page: 1, pageSize: 20, totalItems: 1, totalPages: 1 },
            },
            traceId: "trace-1",
          }),
          retrieve: vi.fn(),
          cancel: vi.fn(),
          close: vi.fn(),
        },
      },
    };

    const service = createOrderAdminService(client as never);
    const page = await service.listOrders({ page: 1, pageSize: 20 });

    expect(page.items).toHaveLength(1);
    expect(page.items[0]?.orderId).toBe("o-1");
    expect(page.totalItems).toBe(1);
  });

  it("passes the created-at range through to the orders list", async () => {
    const list = vi.fn().mockResolvedValue({
      code: 0,
      data: {
        items: [],
        pageInfo: { mode: "offset", page: 1, pageSize: 20, totalItems: 0, totalPages: 0 },
      },
      traceId: "trace-3",
    });
    const client = {
      orders: { admin: { list, retrieve: vi.fn(), cancel: vi.fn(), close: vi.fn() } },
    };

    const service = createOrderAdminService(client as never);
    await service.listOrders({
      page: 1,
      pageSize: 20,
      status: "paid",
      createdFrom: "2026-07-01T00:00:00",
      createdTo: "2026-07-31T23:59:59",
    });

    expect(list).toHaveBeenCalledWith({
      page: "1",
      pageSize: "20",
      status: "paid",
      q: undefined,
      createdFrom: "2026-07-01T00:00:00",
      createdTo: "2026-07-31T23:59:59",
    });
  });

  it("sends the standard idempotency parameter for admin cancel", async () => {
    const cancel = vi.fn().mockResolvedValue({ code: 0, data: { accepted: true } });
    const client = {
      orders: {
        admin: {
          list: vi.fn(),
          retrieve: vi.fn(),
          cancel,
          close: vi.fn(),
        },
      },
    };

    const service = createOrderAdminService(client as never);
    await service.cancelOrder("o-1", { reason: "operator cancel" });

    expect(cancel).toHaveBeenCalledTimes(1);
    const [orderId, params, body] = cancel.mock.calls[0] ?? [];
    expect(orderId).toBe("o-1");
    expect(body).toEqual({ reason: "operator cancel" });
    expect(params).toEqual({
      idempotencyKey: expect.any(String),
    });
  });

  it("lists shipments by order id for the fulfillment section", async () => {
    const shipmentsList = vi.fn().mockResolvedValue({
      code: 0,
      data: {
        items: [{ shipmentId: "sh-1", shipmentNo: "SH-1", fulfillmentId: "f-1", carrierCode: "sf", trackingNo: "SF1", status: "shipped" }],
        pageInfo: { mode: "offset", page: 1, pageSize: 20, totalItems: 1, totalPages: 1 },
      },
      traceId: "trace-2",
    });
    const client = {
      orders: { admin: { list: vi.fn(), retrieve: vi.fn(), events: { list: vi.fn() }, cancel: vi.fn(), close: vi.fn() } },
      shipments: { list: shipmentsList },
    };

    const service = createOrderAdminService(client as never);
    const shipments = await service.getOrderShipments("o-1");

    expect(shipments).toHaveLength(1);
    expect(shipments[0]?.shipmentNo).toBe("SH-1");
    expect(shipmentsList).toHaveBeenCalledWith({ page: "1", pageSize: "20", orderId: "o-1" });
  });

  it("confirms a provider payment with the request number and idempotency", async () => {
    const confirm = vi.fn().mockResolvedValue({ code: 0, data: { accepted: true } });
    const client = {
      orders: {
        admin: { list: vi.fn(), retrieve: vi.fn(), cancel: vi.fn(), close: vi.fn() },
        paymentConfirmations: { create: confirm },
      },
    };

    const service = createOrderAdminService(client as never);
    await service.confirmOrderPayment("o-1", " PAY-2026-001 ");

    expect(confirm).toHaveBeenCalledTimes(1);
    const [orderId, body, params] = confirm.mock.calls[0] ?? [];
    expect(orderId).toBe("o-1");
    expect(body).toEqual({ requestNo: "PAY-2026-001" });
    expect(params).toEqual({ idempotencyKey: expect.any(String) });
  });
});
