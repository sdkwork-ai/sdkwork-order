import { describe, expect, it, vi } from "vitest";
import {
  createSdkworkOrderController,
  type CreateSdkworkOrderControllerOptions,
  type SdkworkOrderMessagesOverrides,
} from "../src";
import type { SdkworkOrderPagination } from "../src/order-service";

const emptyPagination: SdkworkOrderPagination = {
  hasMore: false,
  page: 1,
  pageSize: 20,
  total: 0,
  totalPages: 0,
};

describe("sdkwork-order-pc-order controller", () => {
  it("bootstraps, filters orders, opens details, and refreshes after cancellation", async () => {
    const allDashboard = {
      orders: [
        {
          createdAt: "2026-04-03T09:00:00.000Z",
          id: "ORDER-3",
          paidAmountCny: 0,
          status: "pending-payment" as const,
          statusLabel: "Pending payment",
          subject: "Pro Monthly",
          totalAmountCny: 199,
        },
        {
          createdAt: "2026-04-02T08:00:00.000Z",
          id: "ORDER-2",
          paidAmountCny: 699,
          status: "paid" as const,
          statusLabel: "Paid",
          subject: "Pro Annual",
          totalAmountCny: 699,
        },
      ],
      pagination: { ...emptyPagination, total: 2, totalPages: 1 },
      statistics: {
        completed: 8,
        pendingPayment: 1,
        pendingReceipt: 0,
        pendingShipment: 0,
        totalAmountCny: 2999,
        totalOrders: 9,
      },
    };
    const pendingDashboard = {
      orders: [allDashboard.orders[0]],
      pagination: { ...emptyPagination, total: 1, totalPages: 1 },
      statistics: allDashboard.statistics,
    };
    const cancelledDashboard = {
      orders: [],
      pagination: { ...emptyPagination, total: 0, totalPages: 0 },
      statistics: {
        ...allDashboard.statistics,
        pendingPayment: 0,
      },
    };
    const detail = {
      createdAt: "2026-04-03T09:00:00.000Z",
      id: "ORDER-3",
      items: [],
      status: "pending-payment" as const,
      statusLabel: "Pending payment",
      subject: "Pro Monthly",
      timeline: [],
      totalAmountCny: 199,
    };
    const service = {
      cancelOrder: vi.fn().mockResolvedValue({
        cancelled: true,
        orderId: "ORDER-3",
      }),
      getDashboard: vi
        .fn()
        // bootstrap (filter=all)
        .mockResolvedValueOnce(allDashboard)
        // setFilter("pending-payment") reload
        .mockResolvedValueOnce(pendingDashboard)
        // cancelOrder reload (still filtering pending-payment)
        .mockResolvedValueOnce(cancelledDashboard),
      getEmptyDashboard: vi.fn().mockReturnValue({
        orders: [],
        pagination: emptyPagination,
        statistics: {
          completed: 0,
          pendingPayment: 0,
          pendingReceipt: 0,
          pendingShipment: 0,
          totalAmountCny: 0,
          totalOrders: 0,
        },
      }),
      getOrderDetail: vi.fn().mockResolvedValue(detail),
      payOrder: vi.fn(),
    };

    const controller = createSdkworkOrderController({
      service,
    });

    await controller.bootstrap();
    expect(controller.getState().visibleOrders).toHaveLength(2);

    await controller.setFilter("pending-payment");
    expect(controller.getState().visibleOrders).toHaveLength(1);

    await controller.openDetail("ORDER-3");
    expect(controller.getState().detail?.id).toBe("ORDER-3");

    await controller.cancelOrder({
      cancelReason: "Changed package",
      orderId: "ORDER-3",
    });

    expect(service.cancelOrder).toHaveBeenCalledWith({
      cancelReason: "Changed package",
      orderId: "ORDER-3",
    });
    expect(controller.getState().dashboard.statistics.pendingPayment).toBe(0);
  });

  it("uses controller copy overrides when mutations fail without an Error instance", async () => {
    const controller = createSdkworkOrderController({
      messages: {
        controller: {
          cancelFailed: "Cancel fallback from overrides",
        },
      } satisfies SdkworkOrderMessagesOverrides,
      service: {
        cancelOrder: vi.fn().mockRejectedValue("bad"),
        getDashboard: vi.fn().mockResolvedValue({
          orders: [],
          pagination: emptyPagination,
          statistics: {
            completed: 0,
            pendingPayment: 0,
            pendingReceipt: 0,
            pendingShipment: 0,
            totalAmountCny: 0,
            totalOrders: 0,
          },
        }),
        getEmptyDashboard: vi.fn().mockReturnValue({
          orders: [],
          pagination: emptyPagination,
          statistics: {
            completed: 0,
            pendingPayment: 0,
            pendingReceipt: 0,
            pendingShipment: 0,
            totalAmountCny: 0,
            totalOrders: 0,
          },
        }),
        getOrderDetail: vi.fn(),
        payOrder: vi.fn(),
      },
    } satisfies CreateSdkworkOrderControllerOptions);

    // Controller owns the error surface: a rejected service call surfaces as
    // a partial result with `success: false` instead of an unhandled
    // rejection. UI reads `state.lastError` for the message.
    const result = await controller.cancelOrder({
      orderId: "ORDER-3",
    });
    expect(result).toMatchObject({
      cancelled: true,
      orderId: "ORDER-3",
      success: false,
    });
    expect(result.message).toContain("Cancel fallback from overrides");
    expect(controller.getState().lastError).toBe("Cancel fallback from overrides");
  });

  it("forwards scene and tag facets to the dashboard query", async () => {
    const emptyStatistics = {
      completed: 0,
      pendingPayment: 0,
      pendingReceipt: 0,
      pendingShipment: 0,
      totalAmountCny: 0,
      totalOrders: 0,
    };
    const getDashboard = vi.fn().mockResolvedValue({
      orders: [],
      pagination: emptyPagination,
      statistics: emptyStatistics,
    });
    const controller = createSdkworkOrderController({
      service: {
        cancelOrder: vi.fn(),
        getDashboard,
        getEmptyDashboard: vi.fn().mockReturnValue({
          orders: [],
          pagination: emptyPagination,
          statistics: emptyStatistics,
        }),
        getOrderDetail: vi.fn(),
        payOrder: vi.fn(),
      },
    });

    await controller.setFacets({ scene: "h5", tags: ["vip", "risk"], tagMatch: "all" });

    expect(getDashboard).toHaveBeenLastCalledWith(expect.objectContaining({
      page: 1,
      scene: "h5",
      tags: ["vip", "risk"],
      tagMatch: "all",
    }));
  });

  it("merges facet updates instead of dropping untouched dimensions", async () => {
    const emptyStatistics = {
      completed: 0,
      pendingPayment: 0,
      pendingReceipt: 0,
      pendingShipment: 0,
      totalAmountCny: 0,
      totalOrders: 0,
    };
    const getDashboard = vi.fn().mockResolvedValue({
      orders: [],
      pagination: emptyPagination,
      statistics: emptyStatistics,
    });
    const controller = createSdkworkOrderController({
      service: {
        cancelOrder: vi.fn(),
        getDashboard,
        getEmptyDashboard: vi.fn().mockReturnValue({
          orders: [],
          pagination: emptyPagination,
          statistics: emptyStatistics,
        }),
        getOrderDetail: vi.fn(),
        payOrder: vi.fn(),
      },
    });

    await controller.setFacets({ scene: "h5", tags: ["vip"] });
    // A later scene-only change must keep the tag selection.
    await controller.setFacets({ scene: "app" });

    expect(getDashboard).toHaveBeenLastCalledWith(expect.objectContaining({
      scene: "app",
      tags: ["vip"],
    }));
  });

  it("omits facet keys when nothing is selected", async () => {
    const emptyStatistics = {
      completed: 0,
      pendingPayment: 0,
      pendingReceipt: 0,
      pendingShipment: 0,
      totalAmountCny: 0,
      totalOrders: 0,
    };
    const getDashboard = vi.fn().mockResolvedValue({
      orders: [],
      pagination: emptyPagination,
      statistics: emptyStatistics,
    });
    const controller = createSdkworkOrderController({
      service: {
        cancelOrder: vi.fn(),
        getDashboard,
        getEmptyDashboard: vi.fn().mockReturnValue({
          orders: [],
          pagination: emptyPagination,
          statistics: emptyStatistics,
        }),
        getOrderDetail: vi.fn(),
        payOrder: vi.fn(),
      },
    });

    await controller.bootstrap();

    const query = getDashboard.mock.calls[0]?.[0] as Record<string, unknown>;
    expect(query).not.toHaveProperty("scene");
    expect(query).not.toHaveProperty("tags");
  });
});
