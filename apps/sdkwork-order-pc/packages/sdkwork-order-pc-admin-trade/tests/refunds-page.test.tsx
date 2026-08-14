import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "./i18n-test-utils";

import { SdkworkOrderRefundsPage } from "../src/pages/RefundsPage";


function createServiceStub() {
  return {
    listRefundRequests: vi.fn(async () => ({
      items: [
        {
          accountValueRequestId: "rf-1",
          requestNo: "RF-2026-1",
          subject: "Token refund",
          targetAsset: "token_bank",
          amount: "50.00",
          currencyCode: "CNY",
          status: "pending",
          createdAt: "2026-07-18T00:00:00.000Z",
        },
      ],
      page: 1,
      pageSize: 20,
      totalItems: 1,
      totalPages: 1,
    })),
    reviewRefundRequest: vi.fn(async () => undefined),
  };
}

describe("SdkworkOrderRefundsPage", () => {
  it("renders refund requests with amount and status", async () => {
    render(<SdkworkOrderRefundsPage canManage={false} service={createServiceStub() as never} />);

    await waitFor(() => expect(screen.getByText("RF-2026-1")).toBeInTheDocument());
    expect(screen.getByText("Token refund")).toBeInTheDocument();
    expect(screen.getByText(/50\.00/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /审核/u })).not.toBeInTheDocument();
  });

  it("submits an approval with reason code and comment", async () => {
    const service = createServiceStub();
    render(<SdkworkOrderRefundsPage canManage service={service as never} />);

    await waitFor(() => expect(screen.getByText("RF-2026-1")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: /审核/u }));
    await waitFor(() => expect(screen.getByText("审核退款单")).toBeInTheDocument());

    fireEvent.change(screen.getByLabelText("原因编码"), { target: { value: "approve-manual" } });
    fireEvent.click(screen.getByRole("button", { name: "确认" }));

    await waitFor(() => {
      expect(service.reviewRefundRequest).toHaveBeenCalledWith(
        "rf-1",
        "approve",
        expect.objectContaining({ reasonCode: "approve-manual" }),
      );
    });
  });
});
