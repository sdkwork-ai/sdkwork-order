import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import "./i18n-test-utils";
import { SdkworkOrderTradeWorkbenchPage } from "../src/pages/TradeWorkbenchPage";


function createServiceStub() {
  return {
    getWorkbenchSummary: vi.fn(async () => ({
      pendingAfterSales: 3,
      pendingRefunds: 7,
      pendingShipments: 5,
      pendingWithdrawals: 2,
      pendingCancellations: 1,
      recentOrders: [
        {
          orderId: "order-1",
          orderSn: "ORDER-1",
          status: "paid",
          statusName: "Paid",
          subject: "Token Bank 100",
          totalAmount: "99.00",
          quantity: "1",
          createdAt: "2026-07-18T00:00:00.000Z",
        },
      ],
    })),
  };
}

describe("SdkworkOrderTradeWorkbenchPage", () => {
  it("renders pending counts and recent orders from the summary", async () => {
    render(<SdkworkOrderTradeWorkbenchPage service={createServiceStub() as never} />);

    await waitFor(() => expect(screen.getByText("3")).toBeInTheDocument());
    expect(screen.getByText("7")).toBeInTheDocument();
    expect(screen.getByText("5")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();

    expect(screen.getByText("待审核售后")).toBeInTheDocument();
    expect(screen.getByText("待审核退款")).toBeInTheDocument();
    expect(screen.getByText("待审核提现")).toBeInTheDocument();
    expect(screen.getByText("待发货")).toBeInTheDocument();
    expect(screen.getByText("待审核取消")).toBeInTheDocument();

    await waitFor(() => expect(screen.getByText("Token Bank 100")).toBeInTheDocument());
    expect(screen.getByText("ORDER-1")).toBeInTheDocument();
    expect(screen.getByText("99.00")).toBeInTheDocument();
  });

  it("localizes machine order subjects instead of rendering raw database codes", async () => {
    const service = {
      getWorkbenchSummary: vi.fn(async () => ({
        pendingAfterSales: 0,
        pendingRefunds: 0,
        pendingShipments: 0,
        pendingWithdrawals: 0,
        pendingCancellations: 0,
        recentOrders: [
          {
            orderId: "order-points",
            orderSn: "ORDER-POINTS",
            status: "pending_payment",
            statusName: "Pending payment",
            subject: "points_recharge",
            totalAmount: "500.00",
            quantity: "1",
            createdAt: "2026-07-18T00:00:00.000Z",
          },
          {
            orderId: "order-token-bank",
            orderSn: "ORDER-TOKEN-BANK",
            status: "paid",
            statusName: "Paid",
            subject: "token_bank_recharge",
            totalAmount: "500.00",
            quantity: "1",
            createdAt: "2026-07-18T00:01:00.000Z",
          },
        ],
      })),
    };

    render(<SdkworkOrderTradeWorkbenchPage service={service as never} />);

    await waitFor(() => expect(screen.getByText("会员积分充值")).toBeInTheDocument());
    expect(screen.getByText("Token Bank 充值")).toBeInTheDocument();
    expect(screen.queryByText("points_recharge")).toBeNull();
    expect(screen.queryByText("token_bank_recharge")).toBeNull();
  });

  it("shows quick entry links for every trade module", async () => {
    render(<SdkworkOrderTradeWorkbenchPage service={createServiceStub() as never} />);

    await waitFor(() => expect(screen.getByText("3")).toBeInTheDocument());
    expect(screen.getByRole("link", { name: /全部订单/u })).toHaveAttribute("href", "/admin/trade/orders");
    expect(screen.getByRole("link", { name: /售后审核/u })).toHaveAttribute("href", "/admin/trade/after-sales");
    expect(screen.getByRole("link", { name: /发货管理/u })).toHaveAttribute("href", "/admin/trade/shipments");
    expect(screen.getByRole("link", { name: /退款单审核/u })).toHaveAttribute("href", "/admin/trade/refunds");
    expect(screen.getByRole("link", { name: /提现单审核/u })).toHaveAttribute("href", "/admin/trade/withdrawals");
    expect(screen.getByRole("link", { name: /取消审核/u })).toHaveAttribute("href", "/admin/trade/cancellations");
    expect(screen.getByRole("link", { name: /查看全部/u })).toHaveAttribute("href", "/admin/trade/orders");
  });

  it("surfaces a load error when the summary request fails", async () => {
    const service = {
      getWorkbenchSummary: vi.fn(async () => { throw new Error("network"); }),
    };
    render(<SdkworkOrderTradeWorkbenchPage service={service as never} />);

    await waitFor(() => expect(screen.getByText(/工作台加载失败/u)).toBeInTheDocument());
    expect(screen.getAllByText("0")).toHaveLength(5);
  });
});
