import { describe, expect, it, vi } from "vitest";

import "./i18n-test-utils";
import { createTradeAdminService, TRADE_PENDING_STATUS } from "../src/trade-admin-service";

function createClientStub() {
  const listPage = (items: unknown[], totalItems = items.length) => ({
    items,
    pageInfo: {
      mode: "offset",
      page: 1,
      pageSize: 1,
      totalItems: String(totalItems),
      totalPages: totalItems > 0 ? 1 : 0,
      hasMore: false,
    },
  });

  const afterSalesManagement = {
    list: vi.fn(async (params?: { status?: string; page?: string; pageSize?: string; afterSalesType?: string; orderId?: string }) => {
      if (params?.status === TRADE_PENDING_STATUS.afterSales) {
        return listPage([], 3);
      }
      return listPage([
        { afterSalesRequestId: "as-1", afterSalesNo: "AS-2026-1", orderId: "order-1", afterSalesType: "refund", reasonCode: "quality", requestedAmount: "99.00", currencyCode: "CNY", status: "submitted" },
      ]);
    }),
    retrieve: vi.fn(async () => ({
      afterSalesRequestId: "as-1", afterSalesNo: "AS-2026-1", orderId: "order-1", afterSalesType: "refund",
      reasonCode: "quality", requestedAmount: "99.00", currencyCode: "CNY", status: "submitted",
    })),
  };
  const afterSalesReviews = { create: vi.fn(async () => ({ afterSalesRequestId: "as-1" })) };
  const shipmentsList = vi.fn(async (params?: { status?: string; page?: string; pageSize?: string }) => {
    if (params?.status === TRADE_PENDING_STATUS.shipments) {
      return listPage([], 5);
    }
    return listPage([
      { shipmentId: "sh-1", shipmentNo: "SH-2026-1", fulfillmentId: "f-1", carrierCode: "sf", trackingNo: "SF123", status: "created" },
    ]);
  });
  const shipmentsRetrieve = vi.fn(async () => ({
    shipmentId: "sh-1", shipmentNo: "SH-2026-1", fulfillmentId: "f-1", carrierCode: "sf", trackingNo: "SF123", status: "created",
  }));
  const packagesApi = {
    management: {
      list: vi.fn(async () => listPage([{ packageId: "pkg-1", shipmentId: "sh-1", packageNo: "PKG-1", packageType: "standard", trackingNo: "SF123", status: "created" }])),
    },
    create: vi.fn(async () => ({ packageId: "pkg-2", shipmentId: "sh-1", packageNo: "PKG-2", packageType: "standard", trackingNo: "SF456", status: "created" })),
    update: vi.fn(async () => ({ packageId: "pkg-1", shipmentId: "sh-1", packageNo: "PKG-1", packageType: "standard", trackingNo: "SF999", status: "shipped" })),
  };
  const refundRequests = {
    list: vi.fn(async (params?: { status?: string; page?: number; pageSize?: number }) => {
      if (params?.status === TRADE_PENDING_STATUS.refunds) {
        return { items: [], pageInfo: { mode: "offset", page: 1, pageSize: 1, totalItems: "7", totalPages: 1 } };
      }
      return { items: [{ accountValueRequestId: "rf-1", requestNo: "RF-2026-1", subject: "Token refund", targetAsset: "token_bank", amount: "50.00", currencyCode: "CNY", status: "pending" }], pageInfo: { mode: "offset", page: 1, pageSize: 20, totalItems: "1", totalPages: 1 } };
    }),
    approve: vi.fn(async () => ({ accepted: true })),
    reject: vi.fn(async () => ({ accepted: true })),
    retry: vi.fn(async () => ({ accepted: true })),
  };
  const withdrawalRequests = {
    list: vi.fn(async (params?: { status?: string; page?: number; pageSize?: number }) => {
      if (params?.status === TRADE_PENDING_STATUS.withdrawals) {
        return { items: [], pageInfo: { mode: "offset", page: 1, pageSize: 1, totalItems: "2", totalPages: 1 } };
      }
      return { items: [], pageInfo: { mode: "offset", page: 1, pageSize: 20, totalItems: "0", totalPages: 0 } };
    }),
    approve: vi.fn(async () => ({ accepted: true })),
    reject: vi.fn(async () => ({ accepted: true })),
    retry: vi.fn(async () => ({ accepted: true })),
  };
  const ordersAdmin = {
    list: vi.fn(async () => listPage([
      { orderId: "order-1", orderSn: "ORDER-1", status: "paid", statusName: "Paid", subject: "Token Bank 100", totalAmount: "99.00", quantity: "1", createdAt: "2026-07-18T00:00:00.000Z" },
    ])),
  };

  return {
    afterSales: { management: afterSalesManagement, reviews: afterSalesReviews },
    shipments: { list: shipmentsList, retrieve: shipmentsRetrieve, packages: packagesApi },
    backend: { refundRequests, withdrawalRequests },
    orders: { admin: ordersAdmin },
  };
}

describe("createTradeAdminService", () => {
  it("lists after-sales requests with extended filters", async () => {
    const client = createClientStub();
    const service = createTradeAdminService(client as never);
    const page = await service.listAfterSales({ page: 1, pageSize: 20, status: "approved", afterSalesType: "refund", orderId: "order-1" });

    expect(client.afterSales.management.list).toHaveBeenCalledWith({
      page: "1",
      pageSize: "20",
      status: "approved",
      afterSalesType: "refund",
      orderId: "order-1",
    });
    expect(page.items).toHaveLength(1);
    expect(page.totalItems).toBe(1);
  });

  it("reviews after-sales requests with approved amount and comment", async () => {
    const client = createClientStub();
    const service = createTradeAdminService(client as never);
    await service.reviewAfterSales("as-1", { action: "approve", approvedAmount: "50.00", reviewComment: "partial refund" });

    expect(client.afterSales.reviews.create).toHaveBeenCalledWith(
      "as-1",
      { reviewAction: "approve", approvedAmount: "50.00", reviewComment: "partial refund" },
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
    );
  });

  it("manages shipment packages", async () => {
    const client = createClientStub();
    const service = createTradeAdminService(client as never);

    const created = await service.createShipmentPackage("sh-1", { packageType: "standard", trackingNo: "SF456", status: "created" });
    expect(created.packageId).toBe("pkg-2");
    expect(client.shipments.packages.create).toHaveBeenCalledWith(
      "sh-1",
      { packageType: "standard", trackingNo: "SF456", status: "created" },
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
    );

    await service.updateShipmentPackage("sh-1", "pkg-1", { trackingNo: "SF999", status: "shipped" });
    expect(client.shipments.packages.update).toHaveBeenCalledWith(
      "sh-1",
      "pkg-1",
      { trackingNo: "SF999", status: "shipped" },
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
    );

    const packages = await service.listShipmentPackages("sh-1");
    expect(packages.items).toHaveLength(1);
    expect(client.shipments.packages.management.list).toHaveBeenCalledWith("sh-1", { page: "1", pageSize: "200" });
  });

  it("reviews refund requests with a reason and comment", async () => {
    const client = createClientStub();
    const service = createTradeAdminService(client as never);
    await service.reviewRefundRequest("rf-1", "reject", { reasonCode: "insufficient-funds", reviewComment: "rejected by ops" });

    expect(client.backend.refundRequests.reject).toHaveBeenCalledWith(
      "rf-1",
      expect.objectContaining({ idempotencyKey: expect.any(String) }),
      { reasonCode: "insufficient-funds", reviewComment: "rejected by ops" },
    );
  });

  it("aggregates workbench pending counts and recent orders", async () => {
    const client = createClientStub();
    const service = createTradeAdminService(client as never);
    const summary = await service.getWorkbenchSummary();

    expect(summary.pendingAfterSales).toBe(3);
    expect(summary.pendingRefunds).toBe(7);
    expect(summary.pendingWithdrawals).toBe(2);
    expect(summary.pendingShipments).toBe(5);
    expect(summary.recentOrders).toHaveLength(1);
    expect(summary.recentOrders[0]?.orderSn).toBe("ORDER-1");
    expect(client.afterSales.management.list).toHaveBeenCalledWith({
      page: "1",
      pageSize: "1",
      status: TRADE_PENDING_STATUS.afterSales,
    });
  });

  it("degrades pending counts to zero when a list request fails", async () => {
    const client = createClientStub();
    client.afterSales.management.list.mockRejectedValueOnce(new Error("network"));
    const service = createTradeAdminService(client as never);
    const summary = await service.getWorkbenchSummary();

    expect(summary.pendingAfterSales).toBe(0);
    expect(summary.pendingRefunds).toBe(7);
    expect(summary.recentOrders).toHaveLength(1);
  });
});
