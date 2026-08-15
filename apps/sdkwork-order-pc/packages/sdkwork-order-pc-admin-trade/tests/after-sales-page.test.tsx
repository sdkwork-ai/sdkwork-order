import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "./i18n-test-utils";

import { SdkworkOrderAfterSalesPage } from "../src/pages/AfterSalesPage";


function createServiceStub() {
  return {
    getAfterSales: vi.fn(async () => ({
      afterSalesRequestId: "as-1",
      afterSalesNo: "AS-2026-1",
      orderId: "order-1",
      afterSalesType: "refund",
      reasonCode: "quality",
      requestedAmount: "99.00",
      currencyCode: "CNY",
      status: "submitted",
    })),
    listAfterSales: vi.fn(async () => ({
      items: [
        {
          afterSalesRequestId: "as-1",
          afterSalesNo: "AS-2026-1",
          orderId: "order-1",
          afterSalesType: "refund",
          reasonCode: "quality",
          requestedAmount: "99.00",
          currencyCode: "CNY",
          status: "submitted",
        },
      ],
      page: 1,
      pageSize: 20,
      totalItems: 1,
      totalPages: 1,
    })),
    reviewAfterSales: vi.fn(async () => ({ afterSalesRequestId: "as-1" })),
  };
}

describe("SdkworkOrderAfterSalesPage", () => {
  it("renders the after-sales list with request rows", async () => {
    render(<SdkworkOrderAfterSalesPage canManage={false} service={createServiceStub() as never} />);

    await waitFor(() => expect(screen.getByText("AS-2026-1")).toBeInTheDocument());
    expect(screen.getByText("退款")).toBeInTheDocument();
    expect(screen.getByText(/99\.00/)).toBeInTheDocument();
  });

  it("hides the review action for read-only operators", async () => {
    render(<SdkworkOrderAfterSalesPage canManage={false} service={createServiceStub() as never} />);

    await waitFor(() => expect(screen.getByText("AS-2026-1")).toBeInTheDocument());
    expect(screen.queryByRole("button", { name: /审核/u })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /详情/u })).toBeInTheDocument();
  });

  it("shows the review action and submits an approval with amount and comment", async () => {
    const service = createServiceStub();
    render(<SdkworkOrderAfterSalesPage canManage service={service as never} />);

    await waitFor(() => expect(screen.getByText("AS-2026-1")).toBeInTheDocument());
    const reviewButton = screen.getByRole("button", { name: /审核/u });
    expect(reviewButton).toBeInTheDocument();

    fireEvent.click(reviewButton);
    await waitFor(() => expect(screen.getByRole("heading", { name: "通过" })).toBeInTheDocument());

    fireEvent.change(screen.getByLabelText("核准金额"), { target: { value: "50.00" } });
    fireEvent.change(screen.getByLabelText("审核备注"), { target: { value: "partial refund" } });
    fireEvent.click(screen.getByRole("button", { name: "确认" }));

    await waitFor(() => {
      expect(service.reviewAfterSales).toHaveBeenCalledWith(
        "as-1",
        expect.objectContaining({ action: "approve", approvedAmount: "50.00", reviewComment: "partial refund" }),
        expect.any(String),
      );
    });
  });

  it("opens the detail drawer when a row is selected", async () => {
    const service = createServiceStub();
    render(<SdkworkOrderAfterSalesPage canManage={false} service={service as never} />);

    await waitFor(() => expect(screen.getByText("AS-2026-1")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: /详情/u }));

    await waitFor(() => expect(service.getAfterSales).toHaveBeenCalledWith("as-1"));
    await waitFor(() => expect(screen.getByText("售后单详情")).toBeInTheDocument());
  });
});
