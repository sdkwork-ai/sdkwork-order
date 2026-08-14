import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SdkworkOrderAdminOrdersPage } from "../src/pages/AdminOrdersPage";
describe("order admin capabilities", () => {
  it("keeps detail access but hides cancel and close for read-only operators", async () => {
    const service = {
      cancelOrder: vi.fn(),
      closeOrder: vi.fn(),
      getOrder: vi.fn(),
      getOrderEvents: vi.fn().mockResolvedValue([]),
      getOrderShipments: vi.fn().mockResolvedValue([]),
      listOrders: vi.fn().mockResolvedValue({
        items: [{
          orderId: "order-1",
          orderNo: "ORDER-1",
          subject: "Commercial order",
          status: "pending_payment",
          statusName: "Pending payment",
          totalAmount: "99.00",
          currencyCode: "CNY",
          createdAt: "2026-07-17T00:00:00.000Z",
        }],
        page: 1,
        pageSize: 20,
        totalItems: 1,
        totalPages: 1,
      }),
    };

    render(
      <SdkworkOrderAdminOrdersPage
        capabilities={{ canManageOrders: false }}
        service={service as never}
      />,
    );

    await waitFor(() => expect(screen.getByText("Commercial order")).toBeInTheDocument());
    expect(screen.getByRole("button", { name: /详情/u })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "取消" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "关闭" })).not.toBeInTheDocument();
  });

  it("confirms order mutations before calling the backend service", async () => {
    const service = {
      cancelOrder: vi.fn().mockResolvedValue(undefined),
      closeOrder: vi.fn().mockResolvedValue(undefined),
      getOrder: vi.fn(),
      getOrderEvents: vi.fn().mockResolvedValue([]),
      getOrderShipments: vi.fn().mockResolvedValue([]),
      listOrders: vi.fn().mockResolvedValue({
        items: [{
          orderId: "order-2",
          orderSn: "ORDER-2",
          subject: "Pending order",
          status: "pending_payment",
          statusName: "Pending payment",
          totalAmount: "68.00",
          quantity: "1",
          createdAt: "2026-07-18T00:00:00.000Z",
        }],
        page: 1,
        pageSize: 20,
        totalItems: 1,
        totalPages: 1,
      }),
    };

    render(
      <SdkworkOrderAdminOrdersPage
        capabilities={{ canManageOrders: true }}
        service={service as never}
      />,
    );

    await waitFor(() => expect(screen.getByText("Pending order")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: "取消" }));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(service.cancelOrder).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "确认取消" }));
    await waitFor(() => expect(service.cancelOrder).toHaveBeenCalledWith("order-2"));
  });
});
