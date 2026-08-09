import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router";
import { Inbox, RefreshCw } from "lucide-react";
import { PageLayout, showToast } from "@sdkwork/ui-mobile-react";

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
  resolveOrderRoutePath,
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

export function OrderCenter({
  orderDetailPath = ORDER_MOBILE_ROUTE_DEFINITIONS.orderDetail.path,
  orderCashierPath = ORDER_MOBILE_ROUTE_DEFINITIONS.orderCashier.path,
}: OrderCenterProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();

  const initialTab: OrderTabId = isOrderTabId(searchParams.get("status"))
    ? (searchParams.get("status") as OrderTabId)
    : "all";

  const [tabs, setTabs] = useState<readonly OrderTab[]>([]);
  const [activeTab, setActiveTab] = useState<OrderTabId>(initialTab);
  const [orders, setOrders] = useState<readonly Order[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [pendingPaymentCount, setPendingPaymentCount] = useState<number | null>(null);

  const loadOrders = useCallback(async (tabId: OrderTabId) => {
    setIsLoading(true);
    try {
      setOrders(await OrderService.getOrders(tabId));
    } catch (error) {
      setOrders([]);
      showToast(toUserErrorMessage(t, error));
    } finally {
      setIsLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void loadOrders(activeTab);
  }, [activeTab, loadOrders]);

  useEffect(() => {
    void OrderService.getOrderStatistics()
      .then((statistics) => setPendingPaymentCount(statistics.pendingPayment))
      .catch(() => setPendingPaymentCount(null));
  }, []);

  useEffect(() => {
    void OrderService.getOrderTabs().then(setTabs).catch(() => setTabs([]));
  }, []);

  const openOrderDetail = (order: Order) => {
    navigate(resolveHostRoutePath(orderDetailPath, { orderId: order.id }));
  };

  const openCashier = (order: Order) => {
    navigate(resolveHostRoutePath(orderCashierPath, { orderId: order.id }));
  };

  const tabLabel = (tab: OrderTab) => {
    const label = t(tab.labelKey, TAB_LABEL_FALLBACKS[tab.labelKey] ?? tab.labelKey);
    if (tab.id === "pending_payment" && pendingPaymentCount !== null && pendingPaymentCount > 0) {
      return `${label} ${pendingPaymentCount}`;
    }
    return label;
  };

  return (
    <PageLayout
      title={t("orders.my_orders", "我的订单")}
      rightElement={
        <button
          onClick={() => void loadOrders(activeTab)}
          aria-label={t("orders.refresh", "刷新")}
          className="p-2 active:opacity-70"
        >
          <RefreshCw className="w-5 h-5 text-text-main" />
        </button>
      }
    >
      <div className="flex flex-col h-full bg-[#f5f6f8] dark:bg-[#1a1b1c]">
        <OrderTabsNav
          tabs={tabs.map((tab) => ({ id: tab.id, label: tabLabel(tab) }))}
          activeTab={activeTab}
          onTabChange={(tabId) => setActiveTab(tabId as OrderTabId)}
        />

        <div className="flex-1 overflow-y-auto px-4 pt-4 pb-[84px] flex flex-col gap-3">
          {isLoading ? (
            <div className="flex flex-col items-center justify-center py-20 text-text-sub opacity-70">
              <div className="w-8 h-8 rounded-full border-4 border-text-sub border-t-transparent animate-spin mb-3" />
              <p className="text-[14px]">{t("orders.loading", "加载中...")}</p>
            </div>
          ) : orders.length > 0 ? (
            orders.map((order) => (
              <OrderCard
                key={order.id}
                order={order}
                onClick={() => openOrderDetail(order)}
                renderActionButtons={(current) => (
                  <OrderActionButtons
                    order={current}
                    onRefresh={() => void loadOrders(activeTab)}
                    onPay={openCashier}
                  />
                )}
              />
            ))
          ) : (
            <div className="flex flex-col items-center justify-center py-20 text-text-sub opacity-70">
              <Inbox className="w-12 h-12 mb-3 stroke-current opacity-40" />
              <span className="text-[14px]">{t("orders.empty", "暂无订单数据")}</span>
            </div>
          )}
        </div>
      </div>
    </PageLayout>
  );
}
