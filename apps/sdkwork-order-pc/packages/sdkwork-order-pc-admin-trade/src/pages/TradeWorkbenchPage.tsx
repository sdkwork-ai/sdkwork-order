import { useEffect, useMemo, useState } from "react";
import {
  ArrowRight,
  Ban,
  ClipboardCheck,
  Coins,
  Layers,
  RefreshCw,
  ShoppingCart,
  Truck,
  Undo2,
  WalletCards,
} from "lucide-react";
import {
  TradeAdminIntlProvider,
  useTradeAdminI18n,
  type TradeAdminIntlProps,
} from "../i18n/intl";
import { useTradeAdminLink } from "../navigation";
import { Button, LoadingBlock, StatusNotice } from "@sdkwork/ui-pc-react";
import { getSdkworkOrderBackendSdkClient } from "@sdkwork/order-pc-admin-core";
import {
  createTradeAdminService,
  type TradeAdminService,
  type TradeWorkbenchSummary,
} from "../trade-admin-service";
import {
  formatAmount,
  formatTimestamp,
  TradeStatusBadge,
} from "../components/trade-shared";

export interface SdkworkOrderTradeWorkbenchProps extends TradeAdminIntlProps {
  service?: TradeAdminService;
}

const EMPTY_SUMMARY: TradeWorkbenchSummary = {
  pendingAfterSales: 0,
  pendingRefunds: 0,
  pendingShipments: 0,
  pendingWithdrawals: 0,
  pendingCancellations: 0,
  recentOrders: [],
};

const QUICK_ENTRIES = [
  { href: "/admin/trade/orders", icon: ShoppingCart, key: "admin.trade.workbench.orders" },
  { href: "/admin/trade/after-sales", icon: ClipboardCheck, key: "admin.trade.workbench.afterSales" },
  { href: "/admin/trade/shipments", icon: Truck, key: "admin.trade.workbench.shipments" },
  { href: "/admin/trade/refunds", icon: Undo2, key: "admin.trade.workbench.refunds" },
  { href: "/admin/trade/withdrawals", icon: WalletCards, key: "admin.trade.workbench.withdrawals" },
  { href: "/admin/trade/cancellations", icon: Ban, key: "admin.trade.workbench.cancellations" },
  { href: "/admin/trade/account-value-packages", icon: Coins, key: "admin.menu.trade.accountValuePackages" },
  { href: "/admin/trade/token-bank-plans", icon: Layers, key: "admin.menu.trade.tokenBankPlans" },
] as const;

function PendingStatCard({
  count,
  icon,
  label,
  href,
}: {
  count: number;
  icon: React.ReactNode;
  label: string;
  href: string;
}) {
  const Link = useTradeAdminLink();
  return (
    <Link
      aria-label={label}
      href={href}
      className="group flex items-start justify-between gap-3 rounded-xl border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] p-4 transition-colors hover:border-[var(--sdk-color-border-focus)]"
    >
      <div className="min-w-0">
        <p className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{label}</p>
        <p className="mt-2 text-3xl font-bold tabular-nums text-[var(--sdk-color-text-primary)]">
          {count.toLocaleString()}
        </p>
      </div>
      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-[var(--sdk-color-brand-primary-soft)] text-[var(--sdk-color-brand-primary)] transition-colors group-hover:bg-[var(--sdk-color-brand-primary)] group-hover:text-white">
        {icon}
      </div>
    </Link>
  );
}

export function SdkworkOrderTradeWorkbenchPage({
  service: injectedService,
  locale,
  messages,
}: SdkworkOrderTradeWorkbenchProps) {
  return (
    <TradeAdminIntlProvider locale={locale} messages={messages}>
      <TradeWorkbenchPageInner service={injectedService} />
    </TradeAdminIntlProvider>
  );
}

function TradeWorkbenchPageInner({ service: injectedService }: { service?: TradeAdminService }) {
  const { t, locale } = useTradeAdminI18n();
  const Link = useTradeAdminLink();
  const service = useMemo(
    () => injectedService ?? createTradeAdminService(getSdkworkOrderBackendSdkClient()),
    [injectedService],
  );
  const [summary, setSummary] = useState<TradeWorkbenchSummary>(EMPTY_SUMMARY);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    void service.getWorkbenchSummary()
      .then((value) => { if (active) setSummary(value); })
      .catch(() => {
        if (!active) return;
        setError(t("admin.trade.workbench.loadError", "The workbench could not be loaded. Please check permissions and network, then retry."));
        setSummary(EMPTY_SUMMARY);
      })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [refreshKey, service, t]);

  return (
    <div aria-label={t("admin.trade.workbench.title", "Trading Center Workbench")} className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-lg font-bold text-[var(--sdk-color-text-primary)]">
            {t("admin.trade.workbench.title", "Trading Center Workbench")}
          </h2>
          <p className="mt-0.5 text-sm text-[var(--sdk-color-text-muted)]">
            {t("admin.trade.workbench.description", "Pending review, fulfillment, and fund operations across the order capability.")}
          </p>
        </div>
        <Button aria-label={t("admin.trade.list.refresh", "Refresh")} disabled={loading} size="icon" title={t("admin.trade.list.refresh", "Refresh")} type="button" variant="outline" onClick={() => setRefreshKey((current) => current + 1)}>
          <RefreshCw aria-hidden="true" className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
        </Button>
      </div>

      {error ? <StatusNotice tone="danger">{error}</StatusNotice> : null}

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-5">
        <PendingStatCard count={summary.pendingAfterSales} icon={<ClipboardCheck aria-hidden="true" className="h-5 w-5" />} label={t("admin.trade.workbench.pendingAfterSales", "After-sales to review")} href="/admin/trade/after-sales?status=submitted" />
        <PendingStatCard count={summary.pendingRefunds} icon={<Undo2 aria-hidden="true" className="h-5 w-5" />} label={t("admin.trade.workbench.pendingRefunds", "Refunds to review")} href="/admin/trade/refunds?status=pending" />
        <PendingStatCard count={summary.pendingWithdrawals} icon={<WalletCards aria-hidden="true" className="h-5 w-5" />} label={t("admin.trade.workbench.pendingWithdrawals", "Withdrawals to review")} href="/admin/trade/withdrawals?status=pending" />
        <PendingStatCard count={summary.pendingShipments} icon={<Truck aria-hidden="true" className="h-5 w-5" />} label={t("admin.trade.workbench.pendingShipments", "Shipments pending dispatch")} href="/admin/trade/shipments?status=created" />
        <PendingStatCard count={summary.pendingCancellations} icon={<Ban aria-hidden="true" className="h-5 w-5" />} label={t("admin.trade.workbench.pendingCancellations", "Cancellations to review")} href="/admin/trade/cancellations?status=pending" />
      </div>

      <section className="rounded-xl border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] p-4">
        <h3 className="text-sm font-semibold text-[var(--sdk-color-text-primary)]">
          {t("admin.trade.workbench.quickEntries", "Quick entries")}
        </h3>
        <div className="mt-3 grid grid-cols-1 gap-2 sm:grid-cols-2 xl:grid-cols-8">
          {QUICK_ENTRIES.map((entry) => {
            const Icon = entry.icon;
            return (
              <Link
                key={entry.href}
                href={entry.href}
                className="flex items-center gap-2 rounded-lg border border-[var(--sdk-color-border-default)] px-3 py-2.5 text-sm font-medium text-[var(--sdk-color-text-secondary)] transition-colors hover:border-[var(--sdk-color-border-focus)] hover:text-[var(--sdk-color-text-primary)]"
              >
                <Icon aria-hidden="true" className="h-4 w-4 shrink-0 text-[var(--sdk-color-brand-primary)]" />
                <span className="min-w-0 flex-1 truncate">{t(entry.key)}</span>
                <ArrowRight aria-hidden="true" className="h-3.5 w-3.5 shrink-0 text-[var(--sdk-color-text-muted)]" />
              </Link>
            );
          })}
        </div>
      </section>

      <section className="rounded-xl border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)]">
        <div className="flex items-center justify-between gap-3 border-b border-[var(--sdk-color-border-default)] px-4 py-3">
          <h3 className="text-sm font-semibold text-[var(--sdk-color-text-primary)]">
            {t("admin.trade.workbench.recentOrders", "Recent orders")}
          </h3>
          <Link href="/admin/trade/orders" className="text-sm font-medium text-[var(--sdk-color-brand-primary)] hover:underline">
            {t("admin.trade.workbench.viewAll", "View all")}
          </Link>
        </div>
        {loading ? (
          <div className="p-6"><LoadingBlock label={t("admin.trade.workbench.recentOrders", "Recent orders")} /></div>
        ) : summary.recentOrders.length === 0 ? (
          <p className="px-4 py-8 text-center text-sm text-[var(--sdk-color-text-muted)]">
            {t("admin.trade.list.emptyTitle", "No records")}
          </p>
        ) : (
          <ul className="divide-y divide-[var(--sdk-color-border-default)]">
            {summary.recentOrders.map((order) => (
              <li key={order.orderId} className="flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
                <Link href="/admin/trade/orders" className="min-w-0">
                  <p className="truncate text-sm font-semibold text-[var(--sdk-color-text-primary)] transition-colors hover:text-[var(--sdk-color-brand-primary)]">
                    {order.subject || order.orderSn || order.orderId}
                  </p>
                  <p className="mt-0.5 truncate font-mono text-xs text-[var(--sdk-color-text-muted)]">
                    {order.orderSn || order.orderId}
                  </p>
                </Link>
                <div className="flex shrink-0 items-center gap-4">
                  <TradeStatusBadge status={order.status} label={order.statusName || order.status} />
                  <span className="font-mono text-sm font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">
                    {formatAmount(order.totalAmount, locale)}
                  </span>
                  <time className="whitespace-nowrap text-xs text-[var(--sdk-color-text-muted)]" dateTime={order.createdAt}>
                    {formatTimestamp(order.createdAt, locale)}
                  </time>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
