import { useTranslation } from "react-i18next";
import React from "react";
import { Store, ChevronRight } from "lucide-react";
import { cn } from "@sdkwork/ui-mobile-react";
import { motion } from "motion/react";
import { formatAmountCny, type Order } from "../services/OrderService";

interface OrderCardProps {
  order: Order;
  onClick: () => void;
  renderActionButtons: (order: Order) => React.ReactNode;
}

export const OrderCard: React.FC<OrderCardProps> = ({
  order,
  onClick,
  renderActionButtons,
}) => {
  const { t, i18n } = useTranslation();
  const pending = order.status === "pending_payment";
  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ duration: 0.2 }}
      onClick={onClick}
      className="bg-white dark:bg-[#1E1E1E] rounded-xl p-3 flex flex-col gap-3 cursor-pointer active:scale-[0.98] transition-transform"
    >
      {/* Order Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5 cursor-pointer active:opacity-70">
          <Store className="w-4 h-4 text-text-main" />
          <span className="text-[14px] font-semibold text-text-main">
            {order.subject}
          </span>
          <ChevronRight className="w-4 h-4 text-text-sub" />
        </div>
        <span
          className={cn(
            "text-[13px] font-medium",
            pending ? "text-primary-blue" : "text-text-sub",
          )}
        >
          {t(`orders.status_${order.status}`, order.statusText)}
        </span>
      </div>

      {/* Items */}
      <div className="flex flex-col gap-3">
        {order.items.map((item) => (
          <div key={item.id} className="flex gap-2.5">
            <div className="w-20 h-20 rounded-lg bg-black/5 dark:bg-white/5 shrink-0 flex items-center justify-center">
              <Store className="w-8 h-8 text-text-sub/40" />
            </div>
            <div className="flex-1 min-w-0 flex flex-col">
              <div className="flex justify-between gap-2 items-start">
                <h4 className="text-[13px] text-text-main leading-[1.4] line-clamp-2 font-medium">
                  {item.title}
                </h4>
                <div className="flex flex-col items-end shrink-0">
                  <span className="text-[13px] font-medium text-text-main">
                    {formatAmountCny(item.unitPrice, order.currencyCode, i18n.language)}
                  </span>
                  <span className="text-[12px] text-text-sub mt-0.5">
                    x{item.quantity}
                  </span>
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Total */}
      <div className="flex justify-end items-center gap-1 mt-1">
        <span className="text-[13px] text-text-sub">
          {t("orders.total_items", { count: order.quantity })}
        </span>
        <span className="text-[13px] text-text-main ml-1">
          {t("orders.payable", "实付款")}
        </span>
        <span className="text-[15px] font-bold text-text-main">
          {formatAmountCny(order.totalAmount, order.currencyCode, i18n.language)}
        </span>
      </div>

      {/* Actions */}
      <div className="flex justify-end items-center gap-2 mt-1 pt-3 border-t border-border-color/50">
        {renderActionButtons(order)}
      </div>
    </motion.div>
  );
};
