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

describe("order centre facets", () => {
  function facetClient() {
    const list = vi.fn().mockResolvedValue({
      code: 0,
      data: {
        items: [],
        pageInfo: { mode: "offset", page: 1, pageSize: 20, totalItems: 0, totalPages: 0 },
      },
      traceId: "trace-f",
    });
    const facetList = vi.fn().mockResolvedValue({
      code: 0,
      data: {
        item: {
          scenes: [{ scene: "h5", count: "4" }],
          tags: [{ tag: "vip", count: "2" }],
          totalOrders: "6",
        },
      },
      traceId: "trace-f2",
    });
    const classificationUpdate = vi.fn().mockResolvedValue({
      code: 0,
      data: {
        item: { orderId: "o-1", scene: "h5", tags: ["vip"], updatedAt: "2026-07-20T00:00:00.000Z" },
      },
      traceId: "trace-f3",
    });
    const tagsCreate = vi.fn().mockResolvedValue({
      code: 0,
      data: { item: { orderId: "o-1", scene: "web", tags: ["vip", "risk"] } },
      traceId: "trace-f4",
    });
    return {
      list,
      facetList,
      classificationUpdate,
      tagsCreate,
      client: {
        orders: {
          admin: {
            list,
            retrieve: vi.fn(),
            cancel: vi.fn(),
            close: vi.fn(),
            facets: { list: facetList },
            classification: { update: classificationUpdate },
            tags: { create: tagsCreate },
          },
        },
      },
    };
  }

  it("forwards scene and tag facets to the list endpoint", async () => {
    const { client, list } = facetClient();
    const service = createOrderAdminService(client as never);

    await service.listOrders({ page: 1, pageSize: 20, scene: "h5", tags: ["vip", "risk"], tagMatch: "all" });

    expect(list).toHaveBeenCalledWith(expect.objectContaining({
      scene: "h5",
      tags: ["vip", "risk"],
      tagMatch: "all",
    }));
  });

  it("omits facet keys entirely when no facet is selected", async () => {
    const { client, list } = facetClient();
    const service = createOrderAdminService(client as never);

    await service.listOrders({ page: 1, pageSize: 20 });

    const sent = list.mock.calls[0]?.[0] as Record<string, unknown>;
    expect(sent).not.toHaveProperty("scene");
    expect(sent).not.toHaveProperty("tags");
    expect(sent).not.toHaveProperty("tagMatch");
  });

  it("defaults the tag match mode to any when tags are present", async () => {
    const { client, list } = facetClient();
    const service = createOrderAdminService(client as never);

    await service.listOrders({ page: 1, pageSize: 20, tags: ["vip"] });

    expect(list).toHaveBeenCalledWith(expect.objectContaining({ tags: ["vip"], tagMatch: "any" }));
  });

  it("reads facet buckets from the envelope", async () => {
    const { client } = facetClient();
    const service = createOrderAdminService(client as never);

    const facets = await service.getOrderFacets({ status: "paid" });

    expect(facets.scenes).toEqual([{ scene: "h5", count: "4" }]);
    expect(facets.tags).toEqual([{ tag: "vip", count: "2" }]);
    expect(facets.totalOrders).toBe("6");
  });

  it("replaces the classification through the idempotent PUT", async () => {
    const { client, classificationUpdate } = facetClient();
    const service = createOrderAdminService(client as never);

    const result = await service.replaceOrderClassification(
      "o-1",
      { tags: ["vip"], scene: "h5", remark: "  reviewed  " },
      "key-1",
    );

    expect(classificationUpdate).toHaveBeenCalledWith(
      "o-1",
      { tags: ["vip"], scene: "h5", remark: "reviewed" },
      { idempotencyKey: "key-1" },
    );
    expect(result.tags).toEqual(["vip"]);
  });

  it("never sends a scene on the add-tags endpoint", async () => {
    const { client, tagsCreate } = facetClient();
    const service = createOrderAdminService(client as never);

    await service.addOrderTags("o-1", { tags: ["vip"], scene: "h5" }, "key-2");

    const [, body] = tagsCreate.mock.calls[0] ?? [];
    expect(body).toEqual({ tags: ["vip"] });
  });

  it("clears the remark when it is only whitespace", async () => {
    const { client, classificationUpdate } = facetClient();
    const service = createOrderAdminService(client as never);

    await service.replaceOrderClassification("o-1", { tags: [], remark: "   " }, "key-3");

    const [, body] = classificationUpdate.mock.calls[0] ?? [];
    expect(body).toEqual({ tags: [] });
  });
});
