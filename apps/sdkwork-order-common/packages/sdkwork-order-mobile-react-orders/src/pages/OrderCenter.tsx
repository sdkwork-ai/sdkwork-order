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

import { CapabilityUnavailablePage } from "../components/CapabilityUnavailablePage";

export function OrderCenter() {
  return (
    <CapabilityUnavailablePage />
  );
}
