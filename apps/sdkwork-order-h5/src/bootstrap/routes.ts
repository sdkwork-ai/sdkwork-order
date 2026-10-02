import { ORDER_MOBILE_ROUTE_DEFINITIONS } from "@sdkwork/order-mobile-react-orders";

/**
 * Host route constants for the order H5 root.
 *
 * The order surfaces reuse the canonical order owner route ids/paths from
 * `ORDER_MOBILE_ROUTE_DEFINITIONS`; the value/withdraw entries reuse the
 * route ids already declared by the im-h5 host for the same capabilities so
 * route identity stays aligned across clients
 * (APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC §2).
 */
export const SDKWORK_ORDER_H5_ORDER_ROUTES = ORDER_MOBILE_ROUTE_DEFINITIONS;

export const SDKWORK_ORDER_H5_HOST_ROUTES = {
  valueHome: {
    id: "app.commerce.orders.value",
    path: "/value",
    screen: "value",
    titleKey: "orders.value_title",
  },
  valueVip: {
    id: "app.membership.vip.index",
    path: "/value/vip",
    screen: "vip",
    titleKey: "membership.vip_title",
  },
  valueTokenBank: {
    id: "app.membership.recharge.index",
    path: "/value/token-bank",
    screen: "token-bank",
    titleKey: "membership.recharge_title",
  },
  valueCoupon: {
    id: "app.membership.coupon.index",
    path: "/value/coupon",
    screen: "coupon",
    titleKey: "membership.coupon_title",
  },
  withdraw: {
    id: "app.commerce.orders.withdraw",
    path: "/withdraw",
    screen: "withdraw",
    titleKey: "orders.withdraw_title",
  },
} as const;

/** Authenticated landing route (order center). */
export const SDKWORK_ORDER_H5_HOME_PATH = SDKWORK_ORDER_H5_ORDER_ROUTES.orderCenter.path;

export interface SdkworkOrderH5RouteDefinition {
  readonly id: string;
  readonly path: string;
  readonly screen: string;
  readonly titleKey: string;
}

/** Aggregated route manifest for shell metadata and tests. */
export const SDKWORK_ORDER_H5_ROUTES: readonly SdkworkOrderH5RouteDefinition[] = [
  ...Object.values(SDKWORK_ORDER_H5_ORDER_ROUTES),
  ...Object.values(SDKWORK_ORDER_H5_HOST_ROUTES),
];
