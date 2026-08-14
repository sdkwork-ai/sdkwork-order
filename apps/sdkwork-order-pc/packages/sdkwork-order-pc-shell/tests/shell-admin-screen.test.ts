import { describe, expect, it } from "vitest";

import { resolveAdminScreen } from "../src/index";

describe("resolveAdminScreen", () => {
  it("routes every trading center section to its own screen", () => {
    expect(resolveAdminScreen("/admin/trade/overview")).toBe("trade-workbench");
    expect(resolveAdminScreen("/admin/trade/orders")).toBe("trade-orders");
    expect(resolveAdminScreen("/admin/trade/after-sales")).toBe("trade-after-sales");
    expect(resolveAdminScreen("/admin/trade/shipments")).toBe("trade-shipments");
    expect(resolveAdminScreen("/admin/trade/refunds")).toBe("trade-refunds");
    expect(resolveAdminScreen("/admin/trade/withdrawals")).toBe("trade-withdrawals");
  });

  it("falls back to the order supervision screen for legacy and unknown admin paths", () => {
    expect(resolveAdminScreen("/admin/orders")).toBe("orders");
    expect(resolveAdminScreen("/admin/whatever")).toBe("orders");
  });
});
