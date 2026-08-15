import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "./i18n-test-utils";

import { SdkworkOrderTradeCenterAdminApp } from "../src/trade-admin-app";


const CAPABILITIES = { canManageOrders: true, canReviewTrade: true };

function createServiceStub() {
  return {
    getWorkbenchSummary: vi.fn(async () => ({
      pendingAfterSales: 0,
      pendingRefunds: 0,
      pendingShipments: 0,
      pendingWithdrawals: 0,
      pendingCancellations: 0,
      recentOrders: [],
    })),
  };
}

describe("SdkworkOrderTradeCenterAdminApp", () => {
  it("renders the workbench for the default/overview section", async () => {
    render(
      <SdkworkOrderTradeCenterAdminApp
        capabilities={CAPABILITIES}
        services={{ trade: createServiceStub() as never }}
      />,
    );
    expect(await screen.findByText("交易中心工作台")).toBeInTheDocument();
  });

  it("renders the orders screen for the orders section", async () => {
    const service = {
      getOrderEvents: vi.fn(async () => []),
      listOrders: vi.fn(async () => ({ items: [], page: 1, pageSize: 20, totalItems: 0, totalPages: 1 })),
    };
    render(
      <SdkworkOrderTradeCenterAdminApp
        capabilities={{ canManageOrders: false, canReviewTrade: false }}
        sectionId="orders"
        services={{ orders: service as never }}
      />,
    );
    expect(await screen.findByText("暂无订单")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "取消" })).not.toBeInTheDocument();
  });

  it("renders the cancellations screen for the cancellations section", async () => {
    const service = {
      listCancellations: vi.fn(async () => ({
        items: [],
        page: 1,
        pageSize: 20,
        totalItems: 0,
        totalPages: 1,
      })),
    };
    render(
      <SdkworkOrderTradeCenterAdminApp
        capabilities={CAPABILITIES}
        sectionId="cancellations"
        services={{ trade: service as never }}
      />,
    );
    expect(await screen.findByText("暂无数据")).toBeInTheDocument();
  });

  it("falls back to the workbench for unknown sections", async () => {
    render(
      <SdkworkOrderTradeCenterAdminApp
        capabilities={CAPABILITIES}
        sectionId="not-a-section"
        services={{ trade: createServiceStub() as never }}
      />,
    );
    expect(await screen.findByText("交易中心工作台")).toBeInTheDocument();
  });
});
