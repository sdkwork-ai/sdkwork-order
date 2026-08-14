import {
  ClipboardCheck,
  LayoutDashboard,
  ShoppingCart,
  Truck,
  Undo2,
  WalletCards,
  type LucideIcon,
} from "lucide-react";

/**
 * Trading center admin contribution metadata.
 *
 * Route records, menu records, and permission hints for a backend-admin
 * domain package stay in the owning package (`sdkwork-order`); embedding
 * hosts only compose them by spreading these records into their module
 * registry, permission hints, and route table (same pattern as the IAM and
 * RTC contributions consumed by `sdkwork-cloudrouter`).
 */

export const TRADE_ADMIN_DEFAULT_PATH = "/admin/trade/overview";

export interface SdkworkOrderAdminTradeModuleDef {
  id: "tradeCenter";
  nameKey: string;
  icon: LucideIcon;
  defaultPath: string;
  pathPrefixes: string[];
}

export const TRADE_ADMIN_MODULE_DEF: SdkworkOrderAdminTradeModuleDef = {
  id: "tradeCenter",
  nameKey: "admin.header.tradeCenter",
  icon: ShoppingCart,
  defaultPath: TRADE_ADMIN_DEFAULT_PATH,
  pathPrefixes: ["/admin/trade"],
};

export interface SdkworkOrderAdminTradeMenuItem {
  path: string;
  labelKey: string;
  icon: LucideIcon;
  iconColor?: string;
}

export interface SdkworkOrderAdminTradeMenuGroup {
  groupKey: string;
  items: SdkworkOrderAdminTradeMenuItem[];
}

export interface SdkworkOrderAdminTradeMenu {
  moduleId: "tradeCenter";
  items?: SdkworkOrderAdminTradeMenuItem[];
  groups: SdkworkOrderAdminTradeMenuGroup[];
}

export const TRADE_ADMIN_MENU: SdkworkOrderAdminTradeMenu = {
  moduleId: "tradeCenter",
  items: [
    { path: "/admin/trade/overview", labelKey: "admin.menu.trade.workbench", icon: LayoutDashboard },
  ],
  groups: [
    {
      groupKey: "admin.menu.trade.orderManagement",
      items: [
        { path: "/admin/trade/orders", labelKey: "admin.menu.trade.orders", icon: ShoppingCart, iconColor: "text-lobster-400" },
        { path: "/admin/trade/after-sales", labelKey: "admin.menu.trade.afterSales", icon: ClipboardCheck, iconColor: "text-amber-500" },
      ],
    },
    {
      groupKey: "admin.menu.trade.fulfillment",
      items: [
        { path: "/admin/trade/shipments", labelKey: "admin.menu.trade.shipments", icon: Truck, iconColor: "text-cyan-500" },
      ],
    },
    {
      groupKey: "admin.menu.trade.funds",
      items: [
        { path: "/admin/trade/refunds", labelKey: "admin.menu.trade.refunds", icon: Undo2, iconColor: "text-emerald-500" },
        { path: "/admin/trade/withdrawals", labelKey: "admin.menu.trade.withdrawals", icon: WalletCards, iconColor: "text-violet-500" },
      ],
    },
  ],
};

/** Trading center screen ids, also used as the `:sectionId?` route param. */
export const TRADE_ADMIN_SECTIONS = [
  "overview",
  "orders",
  "after-sales",
  "shipments",
  "refunds",
  "withdrawals",
] as const;

export type SdkworkOrderAdminTradeSection = (typeof TRADE_ADMIN_SECTIONS)[number];

export interface SdkworkOrderAdminTradeRouteRecord {
  path: string;
  requiredPermission: string;
  redirectTo?: string;
}

export const TRADE_ADMIN_ROUTE_RECORDS: readonly SdkworkOrderAdminTradeRouteRecord[] = [
  { path: "trade", requiredPermission: "cloudrouter.admin.access", redirectTo: TRADE_ADMIN_DEFAULT_PATH },
  { path: "trade/overview", requiredPermission: "cloudrouter.admin.access" },
  { path: "trade/orders", requiredPermission: "cloudrouter.admin.access" },
  { path: "trade/after-sales", requiredPermission: "cloudrouter.admin.access" },
  { path: "trade/shipments", requiredPermission: "cloudrouter.admin.access" },
  { path: "trade/refunds", requiredPermission: "cloudrouter.admin.access" },
  { path: "trade/withdrawals", requiredPermission: "cloudrouter.admin.access" },
];

export interface SdkworkOrderAdminTradePermissionHint {
  pathPrefix: string;
  requiredPermission: string;
}

export const TRADE_ADMIN_PERMISSION_HINTS: readonly SdkworkOrderAdminTradePermissionHint[] = [
  { pathPrefix: "/admin/trade", requiredPermission: "cloudrouter.admin.access" },
  { pathPrefix: "/admin/trade/overview", requiredPermission: "cloudrouter.admin.access" },
  { pathPrefix: "/admin/trade/orders", requiredPermission: "cloudrouter.admin.access" },
  { pathPrefix: "/admin/trade/after-sales", requiredPermission: "cloudrouter.admin.access" },
  { pathPrefix: "/admin/trade/shipments", requiredPermission: "cloudrouter.admin.access" },
  { pathPrefix: "/admin/trade/refunds", requiredPermission: "cloudrouter.admin.access" },
  { pathPrefix: "/admin/trade/withdrawals", requiredPermission: "cloudrouter.admin.access" },
];
