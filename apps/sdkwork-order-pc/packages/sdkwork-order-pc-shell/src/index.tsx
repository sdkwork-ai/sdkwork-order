import { useMemo } from "react";

import { sdkworkOrderPcRuntimeIdentity } from "@sdkwork/order-pc-core";
import { SdkworkOrderAdminOrdersPage } from "@sdkwork/order-pc-admin-orders";
import {
  SdkworkOrderAfterSalesPage,
  SdkworkOrderRefundsPage,
  SdkworkOrderShipmentsPage,
  SdkworkOrderTradeWorkbenchPage,
  SdkworkOrderWithdrawalsPage,
} from "@sdkwork/order-pc-admin-trade";
import { SdkworkOrderPage } from "@sdkwork/order-pc-order";
import { SdkworkThemeProvider } from "@sdkwork/ui-pc-react";

function resolveStandaloneSurface(): "app" | "backend-admin" {
  if (typeof window === "undefined") {
    return "app";
  }
  return window.location.pathname.startsWith("/admin") ? "backend-admin" : "app";
}

export function resolveAdminScreen(pathname: string): string {
  if (pathname.startsWith("/admin/trade/orders")) {
    return "trade-orders";
  }
  if (pathname.startsWith("/admin/trade/after-sales")) {
    return "trade-after-sales";
  }
  if (pathname.startsWith("/admin/trade/shipments")) {
    return "trade-shipments";
  }
  if (pathname.startsWith("/admin/trade/refunds")) {
    return "trade-refunds";
  }
  if (pathname.startsWith("/admin/trade/withdrawals")) {
    return "trade-withdrawals";
  }
  if (pathname.startsWith("/admin/trade")) {
    return "trade-workbench";
  }
  return "orders";
}

export interface OrderAppShellProps {
  /**
   * When false, surfaces show an auth configuration warning (missing access token).
   */
  authConfigured?: boolean;
  /**
   * Optional order page controller override. When omitted, the shell creates a
   * default controller bound to the runtime identity and the
   * `@sdkwork/order-service` SDK.
   */
  orderController?: React.ComponentProps<typeof SdkworkOrderPage>["controller"];
  /**
   * Initial locale for the order surface. Defaults to `zh-CN` to match the
   * document language set by `index.html`.
   */
  locale?: string;
  /**
   * Copy overrides forwarded to the order page i18n provider.
   */
  messages?: React.ComponentProps<typeof SdkworkOrderPage>["messages"];
  /**
   * Theme variant. Defaults to `light` to match the design system baseline.
   */
  theme?: "light" | "dark";
}

/**
 * Application shell for the standalone `sdkwork-order-pc` build.
 *
 * Composes:
 * - `SdkworkThemeProvider` — provides the design tokens via CSS variables.
 * - `SdkworkOrderPage` — the order capability PC surface.
 * - Trading center admin pages — dispatched by the `/admin/trade/*` path.
 *
 * The shell intentionally stays thin: routing, global navigation, and session
 * provisioning are owned by composition hosts (`sdkwork-mall`,
 * `sdkwork-cloudrouter`, …), not by this standalone capability build.
 */
export function OrderAppShell({
  authConfigured = true,
  orderController,
  locale = "zh-CN",
  messages,
  theme = "light",
}: OrderAppShellProps = {}) {
  const surface = resolveStandaloneSurface();
  const buyerPage = useMemo(
    () => (
      <SdkworkOrderPage
        controller={orderController}
        locale={locale}
        messages={messages}
      />
    ),
    [orderController, locale, messages],
  );
  const adminPage = useMemo(() => {
    const screen = resolveAdminScreen(window.location.pathname);
    switch (screen) {
      case "trade-orders":
        return <SdkworkOrderAdminOrdersPage capabilities={{ canManageOrders: true }} />;
      case "trade-after-sales":
        return <SdkworkOrderAfterSalesPage canManage />;
      case "trade-shipments":
        return <SdkworkOrderShipmentsPage canManage />;
      case "trade-refunds":
        return <SdkworkOrderRefundsPage canManage />;
      case "trade-withdrawals":
        return <SdkworkOrderWithdrawalsPage canManage />;
      case "trade-workbench":
        return <SdkworkOrderTradeWorkbenchPage />;
      default:
        return <SdkworkOrderAdminOrdersPage capabilities={{ canManageOrders: true }} />;
    }
  }, [surface]);

  return (
    <SdkworkThemeProvider defaultTheme={theme}>
      <a href="#order-content" className="order-shell-skip-link">
        跳到主内容
      </a>
      <main id="order-content" className="order-shell">
        <header className="order-shell-header" role="banner">
          <span className="order-shell-mark">SDKWork</span>
          <h1 className="order-shell-title">{sdkworkOrderPcRuntimeIdentity.appKey}</h1>
          {!authConfigured ? (
            <p className="order-shell-auth-warning" role="alert">
              未配置 VITE_SDKWORK_ACCESS_TOKEN：API 调用将失败，请在环境变量中设置访问令牌。
            </p>
          ) : null}
          <p className="order-shell-subtitle">
            {surface === "backend-admin"
              ? "Order backend-admin surface — trading center, order supervision, reviews, and fulfillment."
              : "Order capability PC surface — aligned with sdkwork-specs building-block model."}
          </p>
          <nav className="order-shell-nav" aria-label="Order surfaces">
            <a href="/app/order">买家订单</a>
            <a href="/admin/orders">订单监管</a>
            <a href="/admin/trade/overview">交易中心</a>
            <a href="/admin/trade/after-sales">售后单</a>
            <a href="/admin/trade/shipments">发货管理</a>
            <a href="/admin/trade/refunds">退款单</a>
            <a href="/admin/trade/withdrawals">提现单</a>
          </nav>
        </header>
        <section className="order-shell-body" role="main">
          {authConfigured ? (surface === "backend-admin" ? adminPage : buyerPage) : (
            <div className="order-shell-config-hint" role="alert">
              <p>
                未配置 SDK 会话。请设置 <code>VITE_SDKWORK_ACCESS_TOKEN</code>
                （及可选的 <code>VITE_SDKWORK_AUTH_TOKEN</code>）后重新加载。
              </p>
            </div>
          )}
        </section>
      </main>
    </SdkworkThemeProvider>
  );
}

export default OrderAppShell;
