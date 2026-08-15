import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "./i18n-test-utils";

import { SdkworkOrderTokenBankPlansPage } from "../src/pages/TokenBankPlansPage";

function createServiceStub() {
  return {
    createTokenBankPlan: vi.fn(async () => ({ planCode: "TB-2" })),
    listTokenBankPlans: vi.fn(async () => ({
      items: [
        {
          planCode: "TB-1",
          displayName: "Token Bank 100",
          planPeriod: "monthly",
          grantAmount: "1000.00",
          bonusAmount: "100.00",
          priceAmount: "99.00",
          currencyCode: "CNY",
          renewalPolicy: "auto",
          status: "active",
        },
      ],
      page: 1,
      pageSize: 20,
      totalItems: 1,
      totalPages: 1,
    })),
    retireTokenBankPlan: vi.fn(async () => undefined),
    updateTokenBankPlan: vi.fn(async () => ({ planCode: "TB-1" })),
  };
}

describe("SdkworkOrderTokenBankPlansPage", () => {
  it("renders the plan list with localized period and status", async () => {
    render(<SdkworkOrderTokenBankPlansPage canManage={false} service={createServiceStub() as never} />);

    await waitFor(() => expect(screen.getByText("TB-1")).toBeInTheDocument());
    expect(screen.getByText("Token Bank 100")).toBeInTheDocument();
    expect(screen.getByText("月付")).toBeInTheDocument();
    // The status label appears both in the filter select and the row badge.
    expect(screen.getAllByText("启用中").length).toBeGreaterThan(0);
  });

  it("creates a plan through the add dialog", async () => {
    const service = createServiceStub();
    render(<SdkworkOrderTokenBankPlansPage canManage service={service as never} />);

    await waitFor(() => expect(screen.getByText("TB-1")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: /新增套餐/u }));
    await waitFor(() => expect(screen.getByRole("heading", { name: "新增套餐" })).toBeInTheDocument());

    fireEvent.change(screen.getByLabelText("套餐编码"), { target: { value: "TB-2" } });
    fireEvent.change(screen.getByLabelText("名称"), { target: { value: "Token Bank 200" } });
    fireEvent.change(screen.getByLabelText("到账金额"), { target: { value: "2000.00" } });
    fireEvent.change(screen.getByLabelText("售价"), { target: { value: "188.00" } });
    fireEvent.click(screen.getByRole("button", { name: "确认" }));

    await waitFor(() => {
      expect(service.createTokenBankPlan).toHaveBeenCalledWith(
        expect.objectContaining({ planCode: "TB-2", displayName: "Token Bank 200", grantAmount: "2000.00", priceAmount: "188.00", planPeriod: "monthly" }),
      );
    });
  });

  it("retires a plan after confirmation", async () => {
    const service = createServiceStub();
    render(<SdkworkOrderTokenBankPlansPage canManage service={service as never} />);

    await waitFor(() => expect(screen.getByText("TB-1")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: /^停用/u }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "停用" }));
    await waitFor(() => expect(service.retireTokenBankPlan).toHaveBeenCalledWith("TB-1"));
  });
});
