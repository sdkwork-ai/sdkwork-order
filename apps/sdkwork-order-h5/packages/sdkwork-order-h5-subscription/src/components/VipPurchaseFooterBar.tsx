import React from "react";
import { useTranslation } from "react-i18next";
import type { MembershipPackage } from "../services/SubscriptionCatalogPort";

interface VipPurchaseFooterBarProps {
  packageItem: MembershipPackage;
  creating: boolean;
  onPurchase: () => void;
}

/** 底部 fixed 下单购买栏：展示选中套餐价格，C 位购买按钮。 */
export const VipPurchaseFooterBar: React.FC<VipPurchaseFooterBarProps> = ({
  packageItem,
  creating,
  onPurchase,
}) => {
  const { t } = useTranslation();

  return (
    <div className="absolute bottom-0 inset-x-0 flex items-center gap-4 p-4 bg-white dark:bg-[#1A1A1A] border-t border-border-color shadow-[0_-4px_20px_rgba(0,0,0,0.05)] pb-safe z-30">
      <div className="flex-1 min-w-0">
        <div className="flex items-baseline gap-1.5">
          <span className="text-[20px] font-bold text-primary-blue leading-none">
            ¥{packageItem.price}
          </span>
          {packageItem.originalPrice && (
            <span className="text-[12px] text-text-sub line-through">
              ¥{packageItem.originalPrice}
            </span>
          )}
        </div>
        <div className="mt-1 text-[12px] text-text-sub truncate">
          {packageItem.name}
          {packageItem.durationDays > 0 && (
            <span className="ml-1">
              {t("subscription.duration_days", "{{days}}天", { days: packageItem.durationDays })}
            </span>
          )}
        </div>
      </div>
      <button
        type="button"
        disabled={creating}
        onClick={onPurchase}
        className="shrink-0 h-[46px] px-8 bg-primary-blue active:opacity-80 disabled:opacity-40 text-white font-bold text-[15px] rounded-full transition-opacity"
      >
        {creating
          ? t("subscription.creating_order", "正在创建订单...")
          : t("subscription.confirm_payment", "立即开通")}
      </button>
    </div>
  );
};
