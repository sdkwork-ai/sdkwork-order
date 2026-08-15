import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "./i18n-test-utils";

import { SdkworkOrderAccountValuePackagesPage } from "../src/pages/AccountValuePackagesPage";

function createServiceStub() {
  return {
    createAccountValuePackage: vi.fn(async () => ({ packageId: "pkg-2" })),
    listAccountValuePackages: vi.fn(async () => ({
      items: [
        {
          packageId: "pkg-1",
          packageCode: "PKG-1",
          displayName: "Points 100",
          targetAsset: "points",
          grantAmount: "100.00",
          bonusAmount: "10.00",
          priceAmount: "99.00",
          currencyCode: "CNY",
          status: "active",
        },
      ],
      page: 1,
      pageSize: 20,
      totalItems: 1,
      totalPages: 1,
    })),
    retireAccountValuePackage: vi.fn(async () => undefined),
    updateAccountValuePackage: vi.fn(async () => ({ packageId: "pkg-1" })),
  };
}

describe("SdkworkOrderAccountValuePackagesPage", () => {
  it("renders the package list with localized asset and status", async () => {
    render(<SdkworkOrderAccountValuePackagesPage canManage={false} service={createServiceStub() as never} />);

    await waitFor(() => expect(screen.getByText("PKG-1")).toBeInTheDocument());
    expect(screen.getByText("Points 100")).toBeInTheDocument();
    // The asset label appears both in the filter select and the row cell.
    expect(screen.getAllByText("积分").length).toBeGreaterThan(0);
    expect(screen.getByText(/100\.00/)).toBeInTheDocument();
    // The status label appears both in the filter select and the row badge.
    expect(screen.getAllByText("启用中").length).toBeGreaterThan(0);
  });

  it("creates a package through the add dialog", async () => {
    const service = createServiceStub();
    render(<SdkworkOrderAccountValuePackagesPage canManage service={service as never} />);

    await waitFor(() => expect(screen.getByText("PKG-1")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: /新增价值包/u }));
    await waitFor(() => expect(screen.getByRole("heading", { name: "新增价值包" })).toBeInTheDocument());

    fireEvent.change(screen.getByLabelText("包编码"), { target: { value: "PKG-2" } });
    fireEvent.change(screen.getByLabelText("名称"), { target: { value: "Points 200" } });
    fireEvent.change(screen.getByLabelText("到账金额"), { target: { value: "200.00" } });
    fireEvent.change(screen.getByLabelText("售价"), { target: { value: "188.00" } });
    fireEvent.click(screen.getByRole("button", { name: "确认" }));

    await waitFor(() => {
      expect(service.createAccountValuePackage).toHaveBeenCalledWith(
        expect.objectContaining({ packageCode: "PKG-2", displayName: "Points 200", grantAmount: "200.00", priceAmount: "188.00", targetAsset: "points" }),
      );
    });
  });

  it("retires a package after confirmation", async () => {
    const service = createServiceStub();
    render(<SdkworkOrderAccountValuePackagesPage canManage service={service as never} />);

    await waitFor(() => expect(screen.getByText("PKG-1")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: /^停用/u }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "停用" }));
    await waitFor(() => expect(service.retireAccountValuePackage).toHaveBeenCalledWith("pkg-1"));
  });
});
