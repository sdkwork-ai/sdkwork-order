import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router";

import { OrderService, type Order } from "../services/OrderService";
import { OrderCenter } from "./OrderCenter";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function buildOrder(overrides: Partial<Order> = {}): Order {
  return {
    id: "order-1",
    orderSn: "ORDER-1",
    status: "pending_payment",
    statusText: "待付款",
    subject: "token_bank_recharge",
    totalAmount: "1000",
    currencyCode: "CNY",
    quantity: 1,
    createdAt: "2026-10-01T00:00:00Z",
    items: [
      {
        id: "item-1",
        title: "Token Bank 100",
        quantity: 1,
        unitPrice: "1000",
        totalAmount: "1000",
      },
    ],
    ...overrides,
  };
}

function renderCenter() {
  return render(
    <MemoryRouter initialEntries={["/orders"]}>
      <Routes>
        <Route path="/orders" element={<OrderCenter />} />
        <Route path="/orders/:orderId" element={<div>detail-stub</div>} />
        <Route path="/orders/:orderId/cashier" element={<div>cashier-stub</div>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("OrderCenter", () => {
  it("renders listed orders and opens the detail route from a card", async () => {
    vi.spyOn(OrderService, "getOrders").mockResolvedValue([buildOrder()]);
    renderCenter();

    await waitFor(() =>
      expect(screen.getByText("Token Bank 100")).toBeInTheDocument(),
    );
    expect(screen.getAllByText("待付款").length).toBeGreaterThan(0);

    fireEvent.click(screen.getByText("Token Bank 100"));
    expect(screen.getByText("detail-stub")).toBeInTheDocument();
  });

  it("offers cancel and pay for pending payment orders and pays through the cashier route", async () => {
    vi.spyOn(OrderService, "getOrders").mockResolvedValue([buildOrder()]);
    const cancelSpy = vi.spyOn(OrderService, "cancelOrder").mockResolvedValue(undefined);
    renderCenter();

    await waitFor(() =>
      expect(screen.getByRole("button", { name: "付款" })).toBeInTheDocument(),
    );
    expect(screen.getByRole("button", { name: "取消订单" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "取消订单" }));
    await waitFor(() => expect(cancelSpy).toHaveBeenCalledWith("order-1"));

    fireEvent.click(screen.getByRole("button", { name: "付款" }));
    expect(screen.getByText("cashier-stub")).toBeInTheDocument();
  });

  it("shows the empty state when the status tab has no orders", async () => {
    vi.spyOn(OrderService, "getOrders").mockResolvedValue([]);
    renderCenter();

    await waitFor(() =>
      expect(screen.getByText("暂无相关订单")).toBeInTheDocument(),
    );
  });
});
