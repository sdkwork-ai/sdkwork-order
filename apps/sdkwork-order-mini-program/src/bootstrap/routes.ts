/**
 * Mini-program route contributions (MINI_PROGRAM_APP_ARCHITECTURE_SPEC.md
 * section 5 route placement + APP_MINI_PROGRAM_UI_SPEC.md section 4).
 *
 * Route ids follow `<surface>.<domain>.<capability>.<screen>` and align with
 * the PC/H5 order surfaces where the same workflow exists. Physical page
 * paths may differ from other clients; ids/titleKey/auth must align.
 *
 * This root keeps every page in the main package (single-package decision,
 * docs/decisions.md): the placement metadata below carries `rootPackage: true`
 * for each route, so a future subpackage projection is a mechanical
 * `subpackage` + preload move without touching ids.
 */

export interface MiniProgramRoutePlacement {
  rootPackage?: boolean;
  subpackage?: string;
  pagePath: string;
  preload?: boolean;
}

export interface SdkworkMiniProgramRouteContribution {
  id: string;
  surface: "app" | "console" | "admin";
  domain: string;
  capability: string;
  screen: string;
  titleKey: string;
  auth: "public" | "required";
  permissionHint?: string;
  miniProgram: MiniProgramRoutePlacement;
}

export const orderMiniProgramRoutes: readonly SdkworkMiniProgramRouteContribution[] = [
  {
    id: "app.commerce.orders.center",
    surface: "app",
    domain: "commerce",
    capability: "orders",
    screen: "center",
    titleKey: "commerce.orders.center.title",
    auth: "required",
    miniProgram: { rootPackage: true, pagePath: "pages/orders/index" },
  },
  {
    id: "app.commerce.orders.detail",
    surface: "app",
    domain: "commerce",
    capability: "orders",
    screen: "detail",
    titleKey: "commerce.orders.detail.title",
    auth: "required",
    miniProgram: { rootPackage: true, pagePath: "pages/order-detail/index" },
  },
  {
    id: "app.commerce.orders.cashier",
    surface: "app",
    domain: "commerce",
    capability: "orders",
    screen: "cashier",
    titleKey: "commerce.orders.cashier.title",
    auth: "required",
    miniProgram: { rootPackage: true, pagePath: "pages/cashier/index" },
  },
  {
    id: "app.commerce.orders.payment_result",
    surface: "app",
    domain: "commerce",
    capability: "orders",
    screen: "payment_result",
    titleKey: "commerce.orders.payment_result.title",
    auth: "required",
    miniProgram: { rootPackage: true, pagePath: "pages/payment-result/index" },
  },
  {
    id: "app.commerce.orders.coupon_redemption",
    surface: "app",
    domain: "commerce",
    capability: "orders",
    screen: "coupon_redemption",
    titleKey: "commerce.orders.coupon_redemption.title",
    auth: "required",
    miniProgram: { rootPackage: true, pagePath: "pages/coupon/index" },
  },
  {
    id: "app.commerce.orders.refund",
    surface: "app",
    domain: "commerce",
    capability: "orders",
    screen: "refund",
    titleKey: "commerce.orders.refund.title",
    auth: "required",
    miniProgram: { rootPackage: true, pagePath: "pages/refund/index" },
  },
  {
    id: "app.commerce.recharges.center",
    surface: "app",
    domain: "commerce",
    capability: "recharges",
    screen: "center",
    titleKey: "commerce.recharges.center.title",
    auth: "required",
    miniProgram: { rootPackage: true, pagePath: "pages/recharge/index" },
  },
  {
    id: "app.commerce.withdrawals.requests",
    surface: "app",
    domain: "commerce",
    capability: "withdrawals",
    screen: "requests",
    titleKey: "commerce.withdrawals.requests.title",
    auth: "required",
    miniProgram: { rootPackage: true, pagePath: "pages/withdraw/index" },
  },
  {
    id: "app.commerce.profile.center",
    surface: "app",
    domain: "commerce",
    capability: "profile",
    screen: "center",
    titleKey: "commerce.profile.center.title",
    auth: "public",
    miniProgram: { rootPackage: true, pagePath: "pages/profile/index" },
  },
  {
    id: "app.commerce.session.login",
    surface: "app",
    domain: "commerce",
    capability: "session",
    screen: "login",
    titleKey: "commerce.session.login.title",
    auth: "public",
    miniProgram: { rootPackage: true, pagePath: "pages/login/index" },
  },
];
