import { useTranslation } from "react-i18next";
import React from "react";
import { Store } from "lucide-react";
import { formatAmountCny, type Order } from "../services/OrderService";

interface OrderItemsCardProps {
  order: Order;
}

export const OrderItemsCard: React.FC<OrderItemsCardProps> = ({ order }) => {
  const { t, i18n } = useTranslation();
  return (
    <div className="bg-white dark:bg-[#1E1E1E] rounded-xl p-4 shadow-sm flex flex-col gap-4">
      <div className="flex items-center gap-1.5">
        <Store className="w-4 h-4 text-text-main" />
        <span className="text-[14px] font-semibold text-text-main">
          {order.subject}
        </span>
      </div>

      <div className="flex flex-col gap-4">
        {order.items.map((item) => (
          <div key={item.id} className="flex gap-3">
            <div className="w-20 h-20 rounded-lg bg-black/5 dark:bg-white/5 shrink-0 flex items-center justify-center">
              <Store className="w-8 h-8 text-text-sub/40" />
            </div>
            <div className="flex-1 min-w-0 flex flex-col">
              <div className="flex justify-between gap-2 items-start">
                <h4 className="text-[14px] text-text-main leading-[1.4] line-clamp-2 font-medium">
                  {item.title}
                </h4>
                <div className="flex flex-col items-end shrink-0">
                  <span className="text-[14px] font-medium text-text-main">
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
    </div>
  );
};
