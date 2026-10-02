import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router";
import { Loader2, ReceiptText } from "lucide-react";
import { PageLayout } from "@sdkwork/ui-mobile-react";

import { OrderActionButtons } from "../components/OrderActionButtons";
import { OrderInfoCards } from "../components/OrderInfoCards";
import { OrderItemsCard } from "../components/OrderItemsCard";
import { OrderService, type Order } from "../services/OrderService";
import { localizeOrderTitle } from "../services/orderTitle";
import { toUserErrorMessage } from "../services/errorMessage";
import {
  ORDER_MOBILE_ROUTE_DEFINITIONS,
  resolveHostRoutePath,
} from "../routes";

/** Host-overridable order route template (path with `:orderId`). */
export interface OrderDetailProps {
  orderCashierPath?: string;
}

const DEFAULT_ORDER_CASHIER_PATH = ORDER_MOBILE_ROUTE_DEFINITIONS.orderCashier.path;

export function OrderDetail({
  orderCashierPath = DEFAULT_ORDER_CASHIER_PATH,
}: OrderDetailProps = {}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { orderId = "" } = useParams();

  const [order, setOrder] = useState<Order | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notFound, setNotFound] = useState(false);

  const load = useCallback(async () => {
    if (!orderId) {
      setNotFound(true);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    setNotFound(false);
    try {
      const value = await OrderService.getOrderById(orderId);
      if (!value) {
        setNotFound(true);
      } else {
        setOrder(value);
      }
    } catch (err) {
      setError(toUserErrorMessage(t, err));
    } finally {
      setLoading(false);
    }
  }, [orderId, t]);

  useEffect(() => {
    void load();
  }, [load]);

  const handlePay = (value: Order) => {
    navigate(resolveHostRoutePath(orderCashierPath, { orderId: value.id }));
  };

  return (
    <PageLayout title={order ? localizeOrderTitle(order.subject, t) : t("orders.detail_title", "订单详情")}>
      {loading ? (
        <div className="flex flex-1 items-center justify-center">
          <Loader2 className="h-8 w-8 animate-spin text-text-sub" />
        </div>
      ) : notFound || !order ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 px-8 text-center">
          <ReceiptText className="h-10 w-10 text-text-sub/60" />
          <p className="text-[14px] text-text-sub">
            {t("orders.detail_not_found", "订单不存在或已不可见")}
          </p>
        </div>
      ) : error ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 px-8 text-center">
          <p className="max-w-sm text-[14px] text-text-sub">{error}</p>
          <button
            type="button"
            className="rounded-full border border-border-color px-4 py-1.5 text-[13px] text-text-main active:bg-active-bg"
            onClick={() => void load()}
          >
            {t("orders.retry", "重试")}
          </button>
        </div>
      ) : (
        <div className="flex flex-1 flex-col gap-3 overflow-y-auto px-3 py-3 pb-6">
          <OrderInfoCards order={order} />
          <OrderItemsCard order={order} />
          <div className="mt-1 flex justify-end gap-2">
            <OrderActionButtons
              order={order}
              onRefresh={() => void load()}
              onPay={handlePay}
            />
          </div>
        </div>
      )}
    </PageLayout>
  );
}
