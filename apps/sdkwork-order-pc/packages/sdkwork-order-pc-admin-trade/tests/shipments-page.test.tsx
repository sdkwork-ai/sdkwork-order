import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import "./i18n-test-utils";
import { SdkworkOrderShipmentsPage } from "../src/pages/ShipmentsPage";


function createServiceStub() {
  return {
    createShipmentPackage: vi.fn(async () => ({
      packageId: "pkg-2",
      shipmentId: "sh-1",
      packageNo: "PKG-2",
      packageType: "standard",
      trackingNo: "SF456",
      status: "created",
    })),
    getShipment: vi.fn(async () => ({
      shipmentId: "sh-1",
      shipmentNo: "SH-2026-1",
      fulfillmentId: "f-1",
      carrierCode: "sf",
      trackingNo: "SF123",
      status: "created",
    })),
    listShipmentPackages: vi.fn(async () => ({
      items: [
        {
          packageId: "pkg-1",
          shipmentId: "sh-1",
          packageNo: "PKG-1",
          packageType: "standard",
          trackingNo: "SF123",
          status: "created",
        },
      ],
      page: 1,
      pageSize: 20,
      totalItems: 1,
      totalPages: 1,
    })),
    listShipments: vi.fn(async () => ({
      items: [
        {
          shipmentId: "sh-1",
          shipmentNo: "SH-2026-1",
          fulfillmentId: "f-1",
          carrierCode: "sf",
          trackingNo: "SF123",
          status: "created",
        },
      ],
      page: 1,
      pageSize: 20,
      totalItems: 1,
      totalPages: 1,
    })),
    updateShipmentPackage: vi.fn(async () => ({
      packageId: "pkg-1",
      shipmentId: "sh-1",
      packageNo: "PKG-1",
      packageType: "standard",
      trackingNo: "SF999",
      status: "shipped",
    })),
  };
}

describe("SdkworkOrderShipmentsPage", () => {
  it("renders the shipment list and opens the detail drawer with packages", async () => {
    const service = createServiceStub();
    render(<SdkworkOrderShipmentsPage canManage service={service as never} />);

    await waitFor(() => expect(screen.getByText("SH-2026-1")).toBeInTheDocument());
    expect(screen.getByText("sf")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /详情/u }));
    await waitFor(() => expect(service.getShipment).toHaveBeenCalledWith("sh-1"));
    await waitFor(() => expect(screen.getByText("发货单详情")).toBeInTheDocument());
    await waitFor(() => expect(screen.getByText("PKG-1")).toBeInTheDocument());
  });

  it("creates a package with type and tracking number from the drawer", async () => {
    const service = createServiceStub();
    render(<SdkworkOrderShipmentsPage canManage service={service as never} />);

    await waitFor(() => expect(screen.getByText("SH-2026-1")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: /详情/u }));
    await waitFor(() => expect(screen.getByText("发货单详情")).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: /新增包裹/u }));
    await waitFor(() => expect(screen.getByRole("heading", { name: "新增包裹" })).toBeInTheDocument());

    fireEvent.change(screen.getByLabelText("包裹类型"), { target: { value: "express" } });
    fireEvent.change(screen.getByLabelText("包裹编号"), { target: { value: "PKG-2" } });
    fireEvent.change(screen.getByLabelText(/运单号/u), { target: { value: "SF456" } });
    fireEvent.click(screen.getByRole("button", { name: "确认" }));

    await waitFor(() => {
      expect(service.createShipmentPackage).toHaveBeenCalledWith(
        "sh-1",
        expect.objectContaining({ packageType: "express", packageNo: "PKG-2", trackingNo: "SF456" }),
      );
    });
  });
});
