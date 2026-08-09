/**
 * Canonical mobile Order route manifest.
 *
 * The Order owner package declares its H5 route ids and paths once; hosts
 * (e.g. sdkwork-im-h5) aggregate this manifest into their shell route
 * registry instead of duplicating path constants.
 */

export interface OrderMobileRouteDefinition {
  readonly id: string;
  readonly path: string;
  readonly screen: string;
  readonly titleKey: string;
}

export const ORDER_MOBILE_ROUTE_DEFINITIONS = {
  orderCenter: {
    id: "app.commerce.orders.center",
    path: "/orders",
    screen: "center",
    titleKey: "orders.title",
  },
  orderDetail: {
    id: "app.commerce.orders.detail",
    path: "/orders/:orderId",
    screen: "detail",
    titleKey: "orders.detail_title",
  },
  orderCashier: {
    id: "app.commerce.orders.cashier",
    path: "/orders/:orderId/cashier",
    screen: "cashier",
    titleKey: "orders.cashier_title",
  },
  voucherCode: {
    id: "app.commerce.orders.voucher",
    path: "/orders/voucher",
    screen: "voucher",
    titleKey: "orders.voucher_title",
  },
} as const satisfies Readonly<Record<string, OrderMobileRouteDefinition>>;

export type OrderMobileRouteId = keyof typeof ORDER_MOBILE_ROUTE_DEFINITIONS;

export const ORDER_MOBILE_ROUTE_DEFINITIONS_LIST: readonly OrderMobileRouteDefinition[] =
  Object.values(ORDER_MOBILE_ROUTE_DEFINITIONS);

/** Resolves a route path with `:orderId` replaced. */
export function resolveOrderRoutePath(
  definition: OrderMobileRouteDefinition,
  params: { readonly orderId?: string } = {},
): string {
  return resolveHostRoutePath(definition.path, params);
}

/**
 * Resolves a host-provided route template with `:orderId` replaced. Hosts
 * override the canonical order paths (e.g. `/me/orders/:orderId`), so pages
 * must navigate through their injected templates instead of the canonical
 * path constants.
 */
export function resolveHostRoutePath(
  template: string,
  params: { readonly orderId?: string } = {},
): string {
  if (params.orderId === undefined) {
    return template;
  }
  return template.replace(":orderId", encodeURIComponent(params.orderId));
}
