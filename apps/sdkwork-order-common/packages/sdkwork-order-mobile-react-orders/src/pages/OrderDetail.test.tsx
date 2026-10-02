import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router";

import { OrderService, type Order } from "../services/OrderService";
import { OrderDetail } from "./OrderDetail";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const detailOrder: Order = {
  id: "order-9",
  orderSn: "ORDER-9",
  status: "paid",
  statusText: "已付款",
  subject: "points_recharge",
  totalAmount: "5000",
  currencyCode: "CNY",
  quantity: 1,
  createdAt: "2026-10-02T00:00:00Z",
  items: [
    {
      id: "item-9",
      title: "Points 50",
      quantity: 1,
      unitPrice: "5000",
      totalAmount: "5000",
    },
  ],
};

function renderDetail(orderId: string) {
  return render(
    <MemoryRouter initialEntries={[`/orders/${orderId}`]}>
      <Routes>
        <Route path="/orders/:orderId" element={<OrderDetail />} />
        <Route path="/orders/:orderId/cashier" element={<div>cashier-stub</div>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("OrderDetail", () => {
  it("renders order info and items for an existing order", async () => {
    vi.spyOn(OrderService, "getOrderById").mockResolvedValue(detailOrder);
    renderDetail("order-9");

    await waitFor(() =>
      expect(screen.getByText("Points 50")).toBeInTheDocument(),
    );
    expect(screen.getByText("ORDER-9")).toBeInTheDocument();
    expect(screen.getByText("订单信息")).toBeInTheDocument();
  });

  it("shows the typed not-found state for a missing order id", async () => {
    vi.spyOn(OrderService, "getOrderById").mockResolvedValue(null);
    renderDetail("missing");

    await waitFor(() =>
      expect(screen.getByText("订单不存在或已不可见")).toBeInTheDocument(),
    );
  });
});
