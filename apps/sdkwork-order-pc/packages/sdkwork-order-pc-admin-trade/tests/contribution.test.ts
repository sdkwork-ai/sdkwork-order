import { describe, expect, it } from "vitest";

import "./i18n-test-utils";
import {
  TRADE_ADMIN_DEFAULT_PATH,
  TRADE_ADMIN_MENU,
  TRADE_ADMIN_MODULE_DEF,
  TRADE_ADMIN_PERMISSION_HINTS,
  TRADE_ADMIN_ROUTE_RECORDS,
  TRADE_ADMIN_SECTIONS,
} from "../src/contribution";
import { sdkworkOrderPcAdminTradeRoutes } from "../src/routes";

describe("trading center contribution metadata", () => {
  it("declares the module def pointing at the workbench default path", () => {
    expect(TRADE_ADMIN_MODULE_DEF.id).toBe("tradeCenter");
    expect(TRADE_ADMIN_MODULE_DEF.defaultPath).toBe(TRADE_ADMIN_DEFAULT_PATH);
    expect(TRADE_ADMIN_MODULE_DEF.pathPrefixes).toEqual(["/admin/trade"]);
    expect(TRADE_ADMIN_DEFAULT_PATH).toBe("/admin/trade/overview");
  });

  it("keeps every menu path inside the module prefix", () => {
    const paths = [
      ...(TRADE_ADMIN_MENU.items ?? []).map((item) => item.path),
      ...TRADE_ADMIN_MENU.groups.flatMap((group) => group.items.map((item) => item.path)),
    ];
    expect(paths).toHaveLength(9);
    for (const path of paths) {
      expect(path.startsWith("/admin/trade/")).toBe(true);
    }
  });

  it("declares route records for every trading center section plus the redirect", () => {
    expect(TRADE_ADMIN_ROUTE_RECORDS.map((record) => record.path)).toEqual([
      "trade",
      "trade/overview",
      "trade/orders",
      "trade/after-sales",
      "trade/shipments",
      "trade/refunds",
      "trade/withdrawals",
      "trade/cancellations",
      "trade/account-value-packages",
      "trade/token-bank-plans",
    ]);
    const redirect = TRADE_ADMIN_ROUTE_RECORDS.find((record) => record.path === "trade");
    expect(redirect?.redirectTo).toBe("/admin/trade/overview");
  });

  it("declares permission hints aligned with the route records", () => {
    const recordPaths = TRADE_ADMIN_ROUTE_RECORDS.map((record) => record.path);
    const hintPaths = TRADE_ADMIN_PERMISSION_HINTS.map((hint) =>
      hint.pathPrefix === "/admin/trade" ? "trade" : hint.pathPrefix.replace(/^\/admin\//, ""),
    );
    expect(hintPaths.sort()).toEqual(recordPaths.sort());
    for (const hint of TRADE_ADMIN_PERMISSION_HINTS) {
      expect(hint.requiredPermission).toBe("cloudrouter.admin.access");
    }
  });

  it("covers every declared section with a route record and a route contribution", () => {
    for (const section of TRADE_ADMIN_SECTIONS) {
      expect(
        TRADE_ADMIN_ROUTE_RECORDS.some((record) => record.path === `trade/${section}`),
        `missing route record for section ${section}`,
      ).toBe(true);
      expect(
        sdkworkOrderPcAdminTradeRoutes.some((route) => route.path === `/admin/trade/${section}`),
        `missing route contribution for section ${section}`,
      ).toBe(true);
    }
  });
});
