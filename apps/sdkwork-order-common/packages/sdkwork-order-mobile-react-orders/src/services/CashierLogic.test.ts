import { describe, expect, it } from "vitest";

import {
  CASHIER_TTL_SECONDS,
  computeCashierRemainingSeconds,
  formatCashierCountdown,
  resolveCashierPhaseFromPaymentStatus,
  resolveCashierWireMethod,
} from "./CashierLogic";

const paidStatus = { paid: true, status: "paid", statusName: "Paid" };
const pendingStatus = {
  paid: false,
  status: "pending_payment",
  statusName: "Pending payment",
};

describe("resolveCashierPhaseFromPaymentStatus", () => {
  it("resolves paid from an explicitly paid response", () => {
    expect(resolveCashierPhaseFromPaymentStatus(paidStatus, "pending_payment")).toBe("paid");
  });

  it("resolves paid when the order already moved to a paid state", () => {
    for (const status of ["paid", "fulfilled", "completed", "shipped", "delivered"]) {
      expect(resolveCashierPhaseFromPaymentStatus(pendingStatus, status)).toBe("paid");
    }
  });

  it("resolves refunding/refunded orders as paid", () => {
    expect(resolveCashierPhaseFromPaymentStatus(pendingStatus, "refunding")).toBe("paid");
    expect(resolveCashierPhaseFromPaymentStatus(pendingStatus, "refunded")).toBe("paid");
  });

  it("resolves cancelled and expired orders", () => {
    expect(resolveCashierPhaseFromPaymentStatus(pendingStatus, "cancelled")).toBe("cancelled");
    expect(resolveCashierPhaseFromPaymentStatus(pendingStatus, "closed")).toBe("cancelled");
    expect(resolveCashierPhaseFromPaymentStatus(pendingStatus, "expired")).toBe("expired");
    expect(resolveCashierPhaseFromPaymentStatus(pendingStatus, "timeout")).toBe("expired");
  });

  it("keeps pending while the order waits for payment", () => {
    expect(resolveCashierPhaseFromPaymentStatus(pendingStatus, "pending_payment")).toBe("pending");
    expect(resolveCashierPhaseFromPaymentStatus(pendingStatus, "draft")).toBe("pending");
  });
});

describe("computeCashierRemainingSeconds", () => {
  const now = Date.parse("2026-08-01T10:00:00Z");

  it("caps the countdown at the backend 15-minute cashier TTL", () => {
    expect(computeCashierRemainingSeconds(undefined, now, now)).toBe(CASHIER_TTL_SECONDS);
    expect(computeCashierRemainingSeconds(undefined, now, now + 60000)).toBe(CASHIER_TTL_SECONDS - 60);
  });

  it("uses the earlier of order expiry and cashier TTL", () => {
    const orderExpiresSoon = "2026-08-01T10:10:00Z";
    expect(computeCashierRemainingSeconds(orderExpiresSoon, now, now)).toBe(600);
  });

  it("floors at zero once the deadline passed", () => {
    expect(computeCashierRemainingSeconds("2026-08-01T09:59:00Z", now - 20000, now)).toBe(0);
  });
});

describe("formatCashierCountdown", () => {
  it("formats mm:ss", () => {
    expect(formatCashierCountdown(899)).toBe("14:59");
    expect(formatCashierCountdown(0)).toBe("00:00");
    expect(formatCashierCountdown(61)).toBe("01:01");
  });
});

describe("resolveCashierWireMethod", () => {
  it("keeps the selected method in a plain browser", () => {
    expect(resolveCashierWireMethod("browser", "wechat_pay", false)).toBe("wechat_pay");
    expect(resolveCashierWireMethod("browser", "alipay", false)).toBe("alipay");
    expect(resolveCashierWireMethod("browser", "balance", false)).toBe("balance");
  });

  it("uses provider H5 trades on a plain mobile browser", () => {
    expect(resolveCashierWireMethod("browser", "wechat_pay", false, true)).toBe("wechat_h5");
    expect(resolveCashierWireMethod("browser", "alipay", false, true)).toBe("alipay_wap");
    expect(resolveCashierWireMethod("browser", "balance", false, true)).toBe("balance");
    // Desktop keeps the native QR trade: scanning is the completion path there.
    expect(resolveCashierWireMethod("browser", "wechat_pay", false, false)).toBe("wechat_pay");
  });

  it("uses wechat_jsapi inside WeChat once the payer openid is known", () => {
    expect(resolveCashierWireMethod("wechat", "wechat_pay", true)).toBe("wechat_jsapi");
    expect(resolveCashierWireMethod("wechat", "wechat_pay", false)).toBe("wechat_pay");
  });

  it("uses alipay_wap inside the Alipay app", () => {
    expect(resolveCashierWireMethod("alipay", "alipay", false)).toBe("alipay_wap");
    expect(resolveCashierWireMethod("alipay", "alipay", true)).toBe("alipay_wap");
  });

  it("passes non-app methods through untouched", () => {
    expect(resolveCashierWireMethod("wechat", "balance", true)).toBe("balance");
    expect(resolveCashierWireMethod("alipay", "balance", true)).toBe("balance");
  });
});
