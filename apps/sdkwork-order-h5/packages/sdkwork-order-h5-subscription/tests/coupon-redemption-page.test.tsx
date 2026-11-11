import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CouponRedemptionPage } from "../src/pages/CouponRedemptionPage";
import type { SubscriptionPurchasePort } from "../src/services/SubscriptionPurchaseService";

/**
 * The page's copy lives in the order H5 i18n resources
 * (`sdkwork-order-h5-shell/src/i18n.ts`); the fallback argument passed by the
 * page is the zh-CN default this test asserts against.
 */
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: unknown) =>
      typeof fallback === "string" ? fallback : key,
  }),
}));

/**
 * The mobile page shell (header + router back affordance) is owned by
 * `@sdkwork/ui-mobile-react`; this suite asserts the coupon failure UX, so the
 * shell is stubbed to keep the page hermetic.
 */
vi.mock("@sdkwork/ui-mobile-react", () => ({
  PageLayout: ({ children, title }: { children: ReactNode; title?: string }) => (
    <section aria-label={title}>{children}</section>
  ),
}));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function createPort(overrides: Partial<SubscriptionPurchasePort> = {}): SubscriptionPurchasePort {
  return {
    listMembershipPackages: vi.fn().mockResolvedValue([]),
    listMembershipPackageGroups: vi.fn().mockResolvedValue([]),
    listTokenBankPlans: vi.fn().mockResolvedValue([]),
    createRechargeOrder: vi.fn(),
    getRechargeStatus: vi.fn(),
    createSubscriptionOrder: vi.fn(),
    getSubscriptionStatus: vi.fn(),
    redeemCoupon: vi.fn(),
    ...overrides,
  };
}

function renderPage(service: SubscriptionPurchasePort): void {
  render(<CouponRedemptionPage service={service} />);
}

async function submitCode(code: string): Promise<void> {
  fireEvent.change(screen.getByPlaceholderText("请输入兑换码"), { target: { value: code } });
  fireEvent.click(screen.getByRole("button", { name: "立即兑换" }));
}

const BENEFIT_DELIVERY_FAILED_COPY = "券已核销，但权益发放失败；请重试，重试不会重复扣券。";

function benefitDeliveryFailed(): Record<string, unknown> {
  return {
    name: "ServiceUnavailableError",
    message: "HTTP 503: Service Unavailable",
    httpStatus: 503,
    traceId: "trace-503",
    problem: {
      status: 503,
      code: 50301,
      traceId: "trace-503",
      i18nKey: "orders.coupon_redemptions.benefit_delivery_failed",
      detail: "An internal error occurred",
    },
  };
}

describe("CouponRedemptionPage failure experience", () => {
  it("shows the benefit-delivery message with support hint and trace id instead of the raw 500 body", async () => {
    const redeemCoupon = vi.fn().mockRejectedValue(benefitDeliveryFailed());
    renderPage(createPort({ redeemCoupon }));

    await submitCode("SAVE-100");

    expect(await screen.findByText(BENEFIT_DELIVERY_FAILED_COPY)).toBeInTheDocument();
    expect(screen.getByText("如多次失败，请联系客服并提供追踪号")).toBeInTheDocument();
    expect(screen.getByText("trace-503")).toBeInTheDocument();
    expect(screen.queryByText("An internal error occurred")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "重试" })).toBeInTheDocument();
  });

  it("retries the same intent with the same idempotency key", async () => {
    const redeemCoupon = vi
      .fn()
      .mockRejectedValueOnce(benefitDeliveryFailed())
      .mockResolvedValueOnce({
        orderId: "coupon-order-1",
        status: "completed",
        benefitKind: "token_bank_credit",
        grantAmount: "100",
      });
    renderPage(createPort({ redeemCoupon }));

    await submitCode("SAVE-100");
    fireEvent.click(await screen.findByRole("button", { name: "重试" }));

    expect(await screen.findByText("兑换成功")).toBeInTheDocument();

    expect(redeemCoupon).toHaveBeenCalledTimes(2);
    const [firstCode, firstKey] = redeemCoupon.mock.calls[0] as [string, string];
    const [secondCode, secondKey] = redeemCoupon.mock.calls[1] as [string, string];
    expect(firstCode).toBe("SAVE-100");
    expect(secondCode).toBe("SAVE-100");
    expect(firstKey).toEqual(expect.any(String));
    expect(secondKey).toBe(firstKey);
  });

  it("shows the already-used message for a 409 and does not offer a retry", async () => {
    const redeemCoupon = vi.fn().mockRejectedValue({
      httpStatus: 409,
      traceId: "trace-409",
      problem: { status: 409, code: 40901, traceId: "trace-409" },
    });
    renderPage(createPort({ redeemCoupon }));

    await submitCode("SAVE-100");

    expect(await screen.findByText("该优惠券已被使用。")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "重试" })).not.toBeInTheDocument();
    expect(screen.getByText("trace-409")).toBeInTheDocument();
  });

  it("shows the invalid-or-expired message for a 400 and starts a new intent when the code changes", async () => {
    const redeemCoupon = vi
      .fn()
      .mockRejectedValueOnce({ httpStatus: 400, problem: { status: 400, code: 40001 } })
      .mockResolvedValueOnce({
        orderId: "coupon-order-2",
        status: "completed",
        benefitKind: "token_bank_credit",
        grantAmount: "50",
      });
    renderPage(createPort({ redeemCoupon }));

    await submitCode("BAD-CODE");
    expect(await screen.findByText("兑换码无效或已过期，请检查兑换码。")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "重试" })).not.toBeInTheDocument();

    await submitCode("GOOD-CODE");
    await waitFor(() => expect(redeemCoupon).toHaveBeenCalledTimes(2));

    const [firstCode, firstKey] = redeemCoupon.mock.calls[0] as [string, string];
    const [secondCode, secondKey] = redeemCoupon.mock.calls[1] as [string, string];
    expect(firstCode).toBe("BAD-CODE");
    expect(secondCode).toBe("GOOD-CODE");
    expect(secondKey).not.toBe(firstKey);
  });

  it("copies the trace id through the surface clipboard action", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const redeemCoupon = vi.fn().mockRejectedValue(benefitDeliveryFailed());
    renderPage(createPort({ redeemCoupon }));

    await submitCode("SAVE-100");
    fireEvent.click(await screen.findByRole("button", { name: "复制" }));

    expect(writeText).toHaveBeenCalledWith("trace-503");
    expect(await screen.findByRole("button", { name: "已复制" })).toBeInTheDocument();
  });
});
