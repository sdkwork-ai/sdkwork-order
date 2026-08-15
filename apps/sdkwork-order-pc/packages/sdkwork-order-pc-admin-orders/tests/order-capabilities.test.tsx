import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "./i18n-test-utils";
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
    await waitFor(() => expect(service.cancelOrder).toHaveBeenCalledWith("order-2", expect.objectContaining({ reason: undefined, cancelType: undefined }), expect.any(String)));
  });

  it("submits the selected quick reason when cancelling an order", async () => {
    const service = {
      cancelOrder: vi.fn().mockResolvedValue(undefined),
      closeOrder: vi.fn().mockResolvedValue(undefined),
      getOrder: vi.fn(),
      getOrderEvents: vi.fn().mockResolvedValue([]),
      getOrderShipments: vi.fn().mockResolvedValue([]),
      listOrders: vi.fn().mockResolvedValue({
        items: [{
          orderId: "order-3",
          orderSn: "ORDER-3",
          subject: "Timeout order",
          status: "pending_payment",
          statusName: "Pending payment",
          totalAmount: "10.00",
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

    await waitFor(() => expect(screen.getByText("Timeout order")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: "超时未支付" }));
    fireEvent.click(screen.getByRole("button", { name: "确认取消" }));

    await waitFor(() => {
      expect(service.cancelOrder).toHaveBeenCalledWith(
        "order-3",
        expect.objectContaining({ reason: "timeout", cancelType: "timeout" }),
        expect.any(String),
      );
    });
  });

  it("creates a partial refund with the order paid amount and an idempotency key", async () => {
    const service = {
      cancelOrder: vi.fn(),
      closeOrder: vi.fn(),
      createRefundRequest: vi.fn().mockResolvedValue(undefined),
      getOrder: vi.fn().mockResolvedValue({
        orderId: "order-4",
        orderSn: "ORDER-4",
        status: "paid",
        statusName: "Paid",
        subject: "Refundable order",
        totalAmount: "99.00",
        paidAmount: "99.00",
        quantity: "1",
        createdAt: "2026-07-18T00:00:00.000Z",
        items: [],
      }),
      getOrderEvents: vi.fn().mockResolvedValue([]),
      getOrderShipments: vi.fn().mockResolvedValue([]),
      listOrders: vi.fn().mockResolvedValue({
        items: [{
          orderId: "order-4",
          orderSn: "ORDER-4",
          subject: "Refundable order",
          status: "paid",
          statusName: "Paid",
          totalAmount: "99.00",
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

    await waitFor(() => expect(screen.getByText("Refundable order")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: /详情/u }));
    await waitFor(() => expect(service.getOrder).toHaveBeenCalledWith("order-4"));
    await waitFor(() => expect(screen.getByText("订单详情")).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: "退款" }));
    await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

    fireEvent.change(screen.getByLabelText("退款金额"), { target: { value: "50.00" } });
    fireEvent.click(screen.getByRole("button", { name: "用户申请" }));
    fireEvent.click(screen.getByRole("button", { name: "创建退款" }));

    await waitFor(() => {
      expect(service.createRefundRequest).toHaveBeenCalledWith(
        "order-4",
        expect.objectContaining({ amount: "50.00", reasonCode: "user_request" }),
        expect.any(String),
      );
    });
  });

  it("gives the order number input priority over the keyword input", async () => {
    const service = {
      listOrders: vi.fn().mockResolvedValue({
        items: [],
        page: 1,
        pageSize: 20,
        totalItems: 0,
        totalPages: 1,
      }),
    };

    render(
      <SdkworkOrderAdminOrdersPage
        capabilities={{ canManageOrders: false }}
        service={service as never}
      />,
    );

    await waitFor(() => expect(service.listOrders).toHaveBeenCalled());
    fireEvent.change(screen.getByLabelText("订单号"), { target: { value: "ORDER-9" } });
    fireEvent.change(screen.getByLabelText("搜索"), { target: { value: "Token" } });
    fireEvent.click(screen.getByRole("button", { name: /查询/u }));

    await waitFor(() => {
      expect(service.listOrders).toHaveBeenLastCalledWith(
        expect.objectContaining({ q: "ORDER-9" }),
      );
    });
  });
});
