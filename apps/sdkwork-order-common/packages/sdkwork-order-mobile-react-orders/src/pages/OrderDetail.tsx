import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router";
import { Loader2, ReceiptText } from "lucide-react";
import { PageLayout, showToast } from "@sdkwork/ui-mobile-react";

import { OrderActionButtons } from "../components/OrderActionButtons";
import { OrderInfoCards } from "../components/OrderInfoCards";
import { OrderItemsCard } from "../components/OrderItemsCard";
import { OrderService, type Order } from "../services/OrderService";
import { toUserErrorMessage } from "../services/errorMessage";
import {
  ORDER_MOBILE_ROUTE_DEFINITIONS,
  resolveHostRoutePath,
} from "../routes";

/** Host-overridable order route template (path with `:orderId`). */
export interface OrderDetailProps {
  orderCashierPath?: string;
}

export function OrderDetail({
  orderCashierPath = ORDER_MOBILE_ROUTE_DEFINITIONS.orderCashier.path,
}: OrderDetailProps) {
  const { orderId } = useParams<{ orderId: string }>();
  const navigate = useNavigate();
  const { t } = useTranslation();

  const [order, setOrder] = useState<Order | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);

  const loadOrder = useCallback(async () => {
    if (!orderId) {
      setNotFound(true);
      setIsLoading(false);
      return;
    }
    setIsLoading(true);
    try {
      const loaded = await OrderService.getOrderById(orderId);
      if (loaded) {
        setOrder(loaded);
        setNotFound(false);
      } else {
        setNotFound(true);
      }
    } catch (error) {
      showToast(toUserErrorMessage(t, error));
      setNotFound(true);
    } finally {
      setIsLoading(false);
    }
  }, [orderId, t]);

  useEffect(() => {
    void loadOrder();
  }, [loadOrder]);

  const openCashier = () => {
    if (orderId) {
      navigate(resolveHostRoutePath(orderCashierPath, { orderId }));
    }
  };

  return (
    <PageLayout title={t("orders.detail_title", "订单详情")}>
      <div className="flex flex-col h-full bg-[#f5f6f8] dark:bg-[#1a1b1c]">
        {isLoading ? (
          <div className="flex flex-col items-center justify-center flex-1 text-text-sub">
            <Loader2 className="w-8 h-8 animate-spin mb-3" />
            <p className="text-[14px]">{t("orders.loading", "加载中...")}</p>
          </div>
        ) : notFound || !order ? (
          <div className="flex flex-col items-center justify-center flex-1 text-text-sub opacity-70">
            <ReceiptText className="w-12 h-12 mb-3 stroke-current opacity-40" />
            <span className="text-[14px]">{t("orders.not_found", "订单不存在")}</span>
          </div>
        ) : (
          <div className="flex-1 overflow-y-auto px-4 py-4 flex flex-col gap-3 pb-24">
            <div className="bg-white dark:bg-[#1E1E1E] rounded-xl p-4 shadow-sm flex items-center justify-between">
              <span className="text-[14px] font-semibold text-text-main">
                {order.subject}
              </span>
              <span
                className={
                  order.status === "pending_payment"
                    ? "text-[13px] font-medium text-primary-blue"
                    : "text-[13px] font-medium text-text-sub"
                }
              >
                {t(`orders.status_${order.status}`, order.statusText)}
              </span>
            </div>

            <OrderItemsCard order={order} />
            <OrderInfoCards order={order} />

            {order.status === "pending_payment" && (
              <div className="fixed bottom-0 inset-x-0 bg-white dark:bg-[#1E1E1E] border-t border-border-color/50 px-4 py-3 flex justify-end gap-2">
                <OrderActionButtons
                  order={order}
                  onRefresh={() => void loadOrder()}
                  onPay={openCashier}
                />
              </div>
            )}
          </div>
        )}
      </div>
    </PageLayout>
  );
}
