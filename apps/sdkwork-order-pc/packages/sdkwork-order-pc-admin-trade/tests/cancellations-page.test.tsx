import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "./i18n-test-utils";

import { SdkworkOrderCancellationsPage } from "../src/pages/CancellationsPage";

function createServiceStub() {
  return {
    listCancellations: vi.fn(async ({ status }: { status?: string } = {}) => ({
      items: [
        {
          id: "cancel-1",
          orderId: "order-1",
          status: "pending",
          reasonCode: "user-request",
          reasonMessage: "User changed mind",
          createdAt: "2026-07-18T00:00:00.000Z",
        },
      ].filter((item) => !status || item.status === status),
      page: 1,
      pageSize: 20,
      totalItems: 1,
      totalPages: 1,
    })),
  };
}

describe("SdkworkOrderCancellationsPage", () => {
  it("renders cancellation records with localized status and reason", async () => {
    render(<SdkworkOrderCancellationsPage service={createServiceStub() as never} />);

    await waitFor(() => expect(screen.getByText("cancel-1")).toBeInTheDocument());
    expect(screen.getByText("order-1")).toBeInTheDocument();
    expect(screen.getByText("user-request")).toBeInTheDocument();
    expect(screen.getByText("User changed mind")).toBeInTheDocument();
    // The status label appears both in the filter select and the row badge.
    expect(screen.getAllByText("待审核").length).toBeGreaterThan(0);
  });

  it("filters by cancellation status", async () => {
    const service = createServiceStub();
    render(<SdkworkOrderCancellationsPage service={service as never} />);

    await waitFor(() => expect(screen.getByText("cancel-1")).toBeInTheDocument());
    fireEvent.change(screen.getByLabelText("状态"), { target: { value: "approved" } });
    fireEvent.click(screen.getByRole("button", { name: /查询/u }));

    await waitFor(() => {
      expect(service.listCancellations).toHaveBeenLastCalledWith(
        expect.objectContaining({ status: "approved" }),
      );
    });
  });

  it("navigates to the first and last pages", async () => {
    const service = {
      listCancellations: vi.fn(async ({ page }: { page?: number } = {}) => ({
        items: [{
          id: `cancel-${page ?? 1}`,
          orderId: "order-1",
          status: "pending",
          reasonCode: "user-request",
          createdAt: "2026-07-18T00:00:00.000Z",
        }],
        page: page ?? 1,
        pageSize: 20,
        totalItems: 45,
        totalPages: 3,
      })),
    };
    render(<SdkworkOrderCancellationsPage service={service as never} />);

    await waitFor(() => expect(screen.getByText("cancel-1")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: "末页" }));
    await waitFor(() => {
      expect(service.listCancellations).toHaveBeenLastCalledWith(
        expect.objectContaining({ page: 3 }),
      );
    });

    fireEvent.click(screen.getByRole("button", { name: "首页" }));
    await waitFor(() => {
      expect(service.listCancellations).toHaveBeenLastCalledWith(
        expect.objectContaining({ page: 1 }),
      );
    });
  });

  it("jumps to a typed page number from the pagination input", async () => {
    const service = {
      listCancellations: vi.fn(async ({ page }: { page?: number } = {}) => ({
        items: [{
          id: `cancel-${page ?? 1}`,
          orderId: "order-1",
          status: "pending",
          reasonCode: "user-request",
          createdAt: "2026-07-18T00:00:00.000Z",
        }],
        page: page ?? 1,
        pageSize: 20,
        totalItems: 45,
        totalPages: 3,
      })),
    };
    render(<SdkworkOrderCancellationsPage service={service as never} />);

    await waitFor(() => expect(screen.getByText("cancel-1")).toBeInTheDocument());
    const pageInput = screen.getByLabelText("跳转页");
    fireEvent.change(pageInput, { target: { value: "3" } });
    fireEvent.keyDown(pageInput, { key: "Enter" });

    await waitFor(() => {
      expect(service.listCancellations).toHaveBeenLastCalledWith(
        expect.objectContaining({ page: 3 }),
      );
    });
  });

  it("applies a status deep link from the URL on mount", async () => {
    window.history.pushState({}, "", "/?status=approved");
    const service = createServiceStub();
    render(<SdkworkOrderCancellationsPage service={service as never} />);

    await waitFor(() => {
      expect(service.listCancellations).toHaveBeenLastCalledWith(
        expect.objectContaining({ status: "approved" }),
      );
    });
    window.history.pushState({}, "", "/");
  });
});
