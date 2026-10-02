import React from "react";
import { useTranslation } from "react-i18next";
import { showToast } from "@sdkwork/ui-mobile-react";
import { OrderService, type Order } from "../services/OrderService";

/**
 * Order statuses that allow a buyer refund request. `refunding`/`refunded`
 * are excluded: a request is already in flight or settled.
 */
const REFUNDABLE_ORDER_STATUSES: readonly string[] = ["paid", "fulfilled", "completed"];

interface OrderActionButtonsProps {
  order: Order;
  onRefresh: () => void;
  onPay: (order: Order) => void;
  /**
   * Opens the refund-request surface for this order (navigation injected by
   * the hosting page). When absent, the refund button stays hidden.
   */
  onRefund?: (order: Order) => void;
}

export const OrderActionButtons: React.FC<OrderActionButtonsProps> = ({
  order,
  onRefresh,
  onPay,
  onRefund,
}) => {
  const { t } = useTranslation();

  const handleAction = async (
    e: React.MouseEvent,
    action: () => Promise<void>,
    successMsg: string
  ) => {
    e.stopPropagation();
    try {
      await action();
      showToast(successMsg);
      onRefresh();
    } catch (err) {
      showToast(t("orders.operation_failed", "操作失败"));
    }
  };

  if (order.status === "pending_payment") {
    return (
      <>
        <button
          onClick={(e) =>
            handleAction(
              e,
              () => OrderService.cancelOrder(order.id),
              t("orders.cancelled_toast", "订单已取消")
            )
          }
          className="px-4 py-1.5 rounded-full border border-border-color text-[13px] text-text-main font-medium active:bg-active-bg transition-colors"
        >
          {t("orders.cancel_order", "取消订单")}
        </button>
        <button
          onClick={(e) => {
            e.stopPropagation();
            onPay(order);
          }}
          className="px-4 py-1.5 rounded-full border border-primary-blue bg-primary-blue text-white text-[13px] font-medium active:opacity-80 transition-opacity"
        >
          {t("orders.pay_now", "付款")}
        </button>
      </>
    );
  }

  const postSaleActions: React.ReactNode[] = [];
  if (order.status === "fulfilled") {
    postSaleActions.push(
      <button
        key="confirm-receipt"
        onClick={(e) =>
          handleAction(
            e,
            () => OrderService.confirmReceipt(order.id),
            t("orders.receipt_confirmed_toast", "确认收货成功")
          )
        }
        className="px-4 py-1.5 rounded-full border border-primary-blue bg-primary-blue text-white text-[13px] font-medium active:opacity-80 transition-opacity"
      >
        {t("orders.confirm_receipt", "确认收货")}
      </button>,
    );
  }
  if (onRefund && REFUNDABLE_ORDER_STATUSES.includes(order.status)) {
    postSaleActions.push(
      <button
        key="refund"
        onClick={(e) => {
          e.stopPropagation();
          onRefund(order);
        }}
        className="px-4 py-1.5 rounded-full border border-border-color text-[13px] text-text-main font-medium active:bg-active-bg transition-colors"
      >
        {t("orders.apply_refund", "申请退款")}
      </button>,
    );
  }
  if (postSaleActions.length === 0) {
    return null;
  }
  return <>{postSaleActions}</>;
};
