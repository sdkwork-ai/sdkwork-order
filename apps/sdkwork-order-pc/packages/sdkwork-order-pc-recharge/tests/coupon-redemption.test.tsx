import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SdkworkThemeProvider } from "@sdkwork/ui-pc-react/theme";
import type { SdkworkCouponRedemptionService } from "@sdkwork/order-service";
import {
  SdkworkCouponRedemptionDialog,
  SdkworkCouponRedemptionInline,
} from "../src";

function renderWithTheme(node: ReactNode) {
  return render(<SdkworkThemeProvider defaultTheme="light">{node}</SdkworkThemeProvider>);
}

afterEach(cleanup);

describe("SDKWork coupon redemption surfaces", () => {
  it("trims the code and renders a Token Bank credit", async () => {
    const redeem = vi.fn().mockResolvedValue({
      benefitKind: "token_bank_credit" as const,
      grantAmount: 500,
      orderId: "order-token-bank",
      orderNo: "CP1001",
      replayed: false,
      status: "completed" as const,
      targetAsset: "token_bank" as const,
    });

    renderWithTheme(<SdkworkCouponRedemptionInline service={{ redeem }} />);

    fireEvent.change(screen.getByLabelText("Coupon code"), {
      target: { value: "  TOKEN-500  " },
    });
    fireEvent.click(screen.getByRole("button", { name: "Redeem" }));

    await waitFor(() => expect(redeem).toHaveBeenCalledWith("TOKEN-500", expect.any(String)));
    expect(await screen.findByText("Token Bank credited")).toBeInTheDocument();
    expect(screen.getByText("500")).toBeInTheDocument();
  });

  it("retries the same code with the same idempotency key", async () => {
    const redeem = vi
      .fn()
      .mockRejectedValueOnce(new Error("HTTP 503: Service Unavailable"))
      .mockResolvedValueOnce({
        benefitKind: "token_bank_credit" as const,
        grantAmount: 500,
        orderId: "order-token-bank",
        orderNo: "CP1001",
        replayed: true,
        status: "completed" as const,
        targetAsset: "token_bank" as const,
      });

    renderWithTheme(<SdkworkCouponRedemptionInline initialCode="TOKEN-500" service={{ redeem }} />);

    fireEvent.click(screen.getByRole("button", { name: "Redeem" }));
    expect(await screen.findByText("HTTP 503: Service Unavailable")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Redeem" }));
    expect(await screen.findByText("Token Bank credited")).toBeInTheDocument();

    const keys = redeem.mock.calls.map((call) => call[1]);
    expect(keys).toHaveLength(2);
    expect(keys[0]).toEqual(expect.any(String));
    expect(keys[1]).toBe(keys[0]);
  });

  it("renders subscription quotas and invokes completion", async () => {
    type Result = Awaited<ReturnType<SdkworkCouponRedemptionService["redeem"]>>;
    let resolveRedemption!: (value: Result) => void;
    const pending = new Promise<Result>((resolve) => {
      resolveRedemption = resolve;
    });
    const service: SdkworkCouponRedemptionService = {
      redeem: vi.fn().mockReturnValue(pending),
    };
    const onCompleted = vi.fn();

    renderWithTheme(
      <SdkworkCouponRedemptionInline
        initialCode="SUB-MONTH"
        onCompleted={onCompleted}
        service={service}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Redeem" }));
    expect(screen.getByLabelText("Coupon code")).toBeDisabled();
    expect(screen.getByRole("button", { name: "Redeeming..." })).toBeDisabled();

    const result: Result = {
      benefitKind: "subscription",
      dailyQuota: 1000,
      durationDays: 30,
      expiresAt: "2026-08-25T00:00:00Z",
      orderId: "order-subscription",
      orderNo: "CP1002",
      packageId: "1002",
      period: "month",
      productId: "seed-product-membership",
      replayed: false,
      skuId: "sku-standard-monthly",
      startsAt: "2026-07-26T00:00:00Z",
      status: "completed",
      subscriptionId: "subscription-1002",
      totalQuota: 30000,
    };
    resolveRedemption(result);

    expect(await screen.findByText("Subscription activated")).toBeInTheDocument();
    expect(screen.getByText("1,000")).toBeInTheDocument();
    expect(screen.getByText("30,000")).toBeInTheDocument();
    await waitFor(() => expect(onCompleted).toHaveBeenCalledWith(result));
  });

  it("renders a points credit redemption", async () => {
    const redeem = vi.fn().mockResolvedValue({
      benefitKind: "points_credit" as const,
      grantPoints: 1000,
      orderId: "order-points",
      replayed: false,
      status: "completed" as const,
    });

    renderWithTheme(
      <SdkworkCouponRedemptionInline
        copy={{ pointsCredited: "Points credited" }}
        initialCode="POINTS-1000"
        service={{ redeem }}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Redeem" }));

    expect(await screen.findByText("Points credited")).toBeInTheDocument();
    expect(screen.getByText("1,000")).toBeInTheDocument();
  });

  it("renders a cash credit redemption in yuan", async () => {
    const redeem = vi.fn().mockResolvedValue({
      benefitKind: "cash_credit" as const,
      grantAmount: 10050,
      orderId: "order-cash",
      replayed: false,
      status: "completed" as const,
    });

    renderWithTheme(
      <SdkworkCouponRedemptionInline
        copy={{ cashCredited: "Cash balance credited" }}
        initialCode="CASH-100"
        service={{ redeem }}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Redeem" }));

    expect(await screen.findByText("Cash balance credited")).toBeInTheDocument();
    expect(screen.getByText("100.50")).toBeInTheDocument();
  });

  it("renders service errors and clears them when the code changes", async () => {
    const service: SdkworkCouponRedemptionService = {
      redeem: vi.fn().mockRejectedValue(new Error("Coupon has expired")),
    };

    renderWithTheme(
      <SdkworkCouponRedemptionInline initialCode="EXPIRED" service={service} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Redeem" }));
    expect(await screen.findByText("Coupon has expired")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Coupon code"), {
      target: { value: "NEW-CODE" },
    });
    expect(screen.queryByText("Coupon has expired")).not.toBeInTheDocument();
  });

  it("resets result and code after the dialog is closed and reopened", async () => {
    const service: SdkworkCouponRedemptionService = {
      redeem: vi.fn().mockResolvedValue({
        benefitKind: "token_bank_credit",
        grantAmount: 50,
        orderId: "order-reset",
        orderNo: "CP1003",
        replayed: false,
        status: "completed",
        targetAsset: "token_bank",
      }),
    };
    const onClose = vi.fn();
    const dialog = (isOpen: boolean) => (
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkCouponRedemptionDialog
          initialCode="WELCOME"
          isOpen={isOpen}
          onClose={onClose}
          service={service}
        />
      </SdkworkThemeProvider>
    );
    const view = render(dialog(true));

    expect(screen.getAllByRole("button", { name: "Close" })).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "Redeem" }));
    expect(await screen.findByText("Token Bank credited")).toBeInTheDocument();

    view.rerender(dialog(false));
    view.rerender(dialog(true));

    await waitFor(() => {
      expect(screen.queryByText("Token Bank credited")).not.toBeInTheDocument();
      expect(screen.getByLabelText("Coupon code")).toHaveValue("WELCOME");
    });
  });
});
