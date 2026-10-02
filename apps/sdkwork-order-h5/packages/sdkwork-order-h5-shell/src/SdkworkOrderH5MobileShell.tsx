import type { ReactNode } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Crown, Coins, Ticket, ClipboardList, Wallet, Undo2, type LucideIcon } from "lucide-react";

import "./i18n";

export interface SdkworkOrderH5ShellRuntime {
  readonly config: {
    readonly appDisplayName: string;
    readonly environment: string;
    readonly version: string;
  };
}

export interface SdkworkOrderH5MobileShellProps {
  children: ReactNode;
  runtime: SdkworkOrderH5ShellRuntime;
}

interface ShellTab {
  readonly icon: LucideIcon;
  readonly labelKey: string;
  readonly label: string;
  readonly match: (pathname: string) => boolean;
  readonly path: string;
}

const SHELL_TABS: readonly ShellTab[] = [
  {
    icon: ClipboardList,
    labelKey: "shell.tab_orders",
    label: "订单",
    match: (pathname) => pathname.startsWith("/orders"),
    path: "/orders",
  },
  {
    icon: Crown,
    labelKey: "shell.tab_value",
    label: "价值",
    match: (pathname) => pathname.startsWith("/value"),
    path: "/value",
  },
  {
    icon: Wallet,
    labelKey: "shell.tab_withdraw",
    label: "提现",
    match: (pathname) => pathname.startsWith("/withdraw"),
    path: "/withdraw",
  },
];

/**
 * Surfaces that keep the bottom tab bar. Stacked pages (order detail,
 * cashier, VIP purchase, ...) render without it: the tab bar belongs to the
 * three tab roots only (mall-h5 shell pattern).
 */
export function isSdkworkOrderH5TabBarSurface(pathname: string): boolean {
  return (
    pathname === "/orders" ||
    pathname === "/value" ||
    pathname === "/withdraw"
  );
}

export function SdkworkOrderH5MobileShell({ children, runtime }: SdkworkOrderH5MobileShellProps) {
  const location = useLocation();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const showTabBar = isSdkworkOrderH5TabBarSurface(location.pathname);

  return (
    <div className="sdk-h5-app">
      <header className="sdk-h5-topbar">
        <button
          aria-label={runtime.config.appDisplayName}
          className="sdk-h5-topbar-brand"
          onClick={() => navigate("/orders")}
          type="button"
        >
          {t("shell.app_title", "SDKWork 订单")}
        </button>
        <span className="sdk-h5-topbar-title">{runtime.config.appDisplayName}</span>
      </header>

      <main className="sdk-h5-main">{children}</main>

      {showTabBar ? (
        <nav aria-label="底部导航" className="sdk-h5-tabbar">
          {SHELL_TABS.map((tab) => {
            const Icon = tab.icon;
            const active = tab.match(location.pathname);
            return (
              <button
                aria-current={active ? "page" : undefined}
                className={active ? "sdk-h5-tab sdk-h5-tab-active" : "sdk-h5-tab"}
                key={tab.path}
                onClick={() => navigate(tab.path)}
                type="button"
              >
                <span className="sdk-h5-tab-icon">
                  <Icon aria-hidden="true" size={22} />
                </span>
                {t(tab.labelKey, tab.label)}
              </button>
            );
          })}
        </nav>
      ) : null}
    </div>
  );
}

const VALUE_ENTRIES: readonly [
  icon: LucideIcon,
  path: string,
  labelKey: string,
  label: string,
  descKey: string,
  desc: string,
][] = [
  [
    Crown,
    "/value/vip",
    "shell.value_vip",
    "VIP 订阅",
    "shell.value_vip_desc",
    "开通/续费会员权益",
  ],
  [
    Coins,
    "/value/token-bank",
    "shell.value_token_bank",
    "Token Bank",
    "shell.value_token_bank_desc",
    "算力积分充值",
  ],
  [
    Ticket,
    "/value/coupon",
    "shell.value_coupon",
    "券码兑换",
    "shell.value_coupon_desc",
    "兑换优惠券到账户",
  ],
  [
    Undo2,
    "/orders/refunds",
    "shell.value_refunds",
    "我的退款",
    "shell.value_refunds_desc",
    "退款申请与进度",
  ],
];

/**
 * Account value hub (`/value`): the entry tiles into the subscription,
 * Token Bank, coupon redemption and refund surfaces. Pure navigation
 * composition — no business services live in the shell.
 */
export function SdkworkOrderH5ValueHomePage() {
  const navigate = useNavigate();
  const { t } = useTranslation();

  return (
    <div className="sdk-h5-page">
      <h1 className="sdk-h5-page-title">{t("shell.value_title", "账户价值")}</h1>
      <div className="sdk-h5-section">
        <p style={{ margin: 0, fontSize: "0.82rem", color: "var(--sdk-h5-text-muted)" }}>
          {t("shell.value_subtitle", "会员订阅、算力储备与券码兑换")}
        </p>
      </div>
      <div className="sdk-h5-entry-grid">
        {VALUE_ENTRIES.map(([Icon, path, labelKey, label, descKey, desc]) => (
          <button
            className="sdk-h5-entry-item"
            key={path}
            onClick={() => navigate(path)}
            type="button"
          >
            <Icon aria-hidden="true" size={26} />
            <span>{t(labelKey, label)}</span>
            <span className="sdk-h5-entry-desc">{t(descKey, desc)}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
