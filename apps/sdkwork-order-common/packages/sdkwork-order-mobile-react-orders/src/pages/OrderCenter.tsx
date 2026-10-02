import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router";
import { Inbox, RefreshCw } from "lucide-react";
import { PageLayout } from "@sdkwork/ui-mobile-react";

import { OrderActionButtons } from "../components/OrderActionButtons";
import { OrderCard } from "../components/OrderCard";
import { OrderTabsNav } from "../components/OrderTabsNav";
import {
  OrderService,
  type Order,
  type OrderTab,
  type OrderTabId,
} from "../services/OrderService";
import { toUserErrorMessage } from "../services/errorMessage";
import {
  ORDER_MOBILE_ROUTE_DEFINITIONS,
  resolveHostRoutePath,
} from "../routes";

/** Host-overridable order route templates (paths with `:orderId`). */
export interface OrderCenterProps {
  orderDetailPath?: string;
  orderCashierPath?: string;
}

/** Readable tab label fallbacks when a host ships no `orders.tab_*` resource. */
const TAB_LABEL_FALLBACKS: Readonly<Record<string, string>> = {
  "orders.tab_all": "全部",
  "orders.tab_pending_payment": "待付款",
  "orders.tab_paid": "待发货",
  "orders.tab_fulfilled": "待收货",
  "orders.tab_completed": "已完成",
  "orders.tab_cancelled": "已取消",
};

function isOrderTabId(value: string | null | undefined): value is OrderTabId {
  return value === "all"
    || value === "pending_payment"
    || value === "paid"
    || value === "fulfilled"
    || value === "completed"
    || value === "cancelled";
}

const DEFAULT_ORDER_DETAIL_PATH = ORDER_MOBILE_ROUTE_DEFINITIONS.orderDetail.path;
const DEFAULT_ORDER_CASHIER_PATH = ORDER_MOBILE_ROUTE_DEFINITIONS.orderCashier.path;

export function OrderCenter({
  orderDetailPath = DEFAULT_ORDER_DETAIL_PATH,
  orderCashierPath = DEFAULT_ORDER_CASHIER_PATH,
}: OrderCenterProps = {}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const activeTabParam = searchParams.get("tab");
  const activeTab: OrderTabId = isOrderTabId(activeTabParam) ? activeTabParam : "all";

  const [tabs, setTabs] = useState<readonly OrderTab[]>([]);
  const [orders, setOrders] = useState<readonly Order[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      // Tabs are static today, but they resolve through the same service
      // seam so a future server-driven tab set needs no page change.
      const [tabList, pageOrders] = await Promise.all([
        OrderService.getOrderTabs(),
        OrderService.getOrders(activeTab),
      ]);
      setTabs(tabList);
      setOrders(pageOrders);
    } catch (err) {
      setError(toUserErrorMessage(t, err));
    } finally {
      setLoading(false);
    }
  }, [activeTab, t]);

  useEffect(() => {
    void load();
  }, [load]);

  const handleTabChange = (tabId: string) => {
    setSearchParams(tabId === "all" ? {} : { tab: tabId }, { replace: true });
  };

  const handleOpenDetail = (order: Order) => {
    navigate(resolveHostRoutePath(orderDetailPath, { orderId: order.id }));
  };

  const handlePay = (order: Order) => {
    navigate(resolveHostRoutePath(orderCashierPath, { orderId: order.id }));
  };

  const renderActionButtons = (order: Order) => (
    <OrderActionButtons order={order} onRefresh={() => void load()} onPay={handlePay} />
  );

  return (
    <PageLayout
      title={t("orders.title", "订单中心")}
      rightElement={
        <button
          type="button"
          aria-label={t("orders.refresh", "刷新")}
          className="p-2 text-text-main active:opacity-70"
          onClick={() => void load()}
        >
          <RefreshCw className="h-5 w-5" />
        </button>
      }
    >
      <OrderTabsNav
        tabs={tabs.map((tab) => ({
          id: tab.id,
          label: t(tab.labelKey, TAB_LABEL_FALLBACKS[tab.labelKey] ?? tab.id),
        }))}
        activeTab={activeTab}
        onTabChange={handleTabChange}
      />
      <div className="flex-1 overflow-y-auto px-3 py-3">
        {loading ? (
          <div className="flex h-40 items-center justify-center text-[14px] text-text-sub">
            {t("orders.loading", "加载中…")}
          </div>
        ) : error ? (
          <div className="flex flex-col items-center gap-3 pt-16 text-center">
            <p className="max-w-sm text-[14px] text-text-sub">{error}</p>
            <button
              type="button"
              className="rounded-full border border-border-color px-4 py-1.5 text-[13px] text-text-main active:bg-active-bg"
              onClick={() => void load()}
            >
              {t("orders.retry", "重试")}
            </button>
          </div>
        ) : orders.length === 0 ? (
          <div className="flex flex-col items-center gap-3 pt-16 text-center">
            <Inbox className="h-10 w-10 text-text-sub/60" />
            <p className="text-[14px] text-text-sub">
              {t("orders.empty", "暂无相关订单")}
            </p>
          </div>
        ) : (
          <div className="flex flex-col gap-3">
            {orders.map((order) => (
              <OrderCard
                key={order.id}
                order={order}
                onClick={() => handleOpenDetail(order)}
                renderActionButtons={renderActionButtons}
              />
            ))}
          </div>
        )}
      </div>
    </PageLayout>
  );
}
