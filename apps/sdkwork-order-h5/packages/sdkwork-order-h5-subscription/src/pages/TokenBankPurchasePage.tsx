import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { Wallet, Check } from "lucide-react";
import { PageLayout } from "@sdkwork/ui-mobile-react";

import {
  createSubscriptionPurchaseService,
  type SubscriptionPurchasePort,
} from "../services/SubscriptionPurchaseService";
import type { TokenBankPlan } from "../services/SubscriptionCatalogPort";

export interface TokenBankPurchasePageProps {
  service?: SubscriptionPurchasePort;
  getBalance?: () => Promise<string>;
  /** Host cashier route path with `:orderId` placeholder. */
  cashierPath?: string;
}

const DEFAULT_CASHIER_PATH = "/me/orders/:orderId/cashier";

export function TokenBankPurchasePage({
  service: serviceProp,
  getBalance,
  cashierPath = DEFAULT_CASHIER_PATH,
}: TokenBankPurchasePageProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const service = serviceProp ?? createSubscriptionPurchaseService();
  const [plans, setPlans] = useState<TokenBankPlan[]>([]);
  const [balance, setBalance] = useState<string | null>(null);
  const [selectedCode, setSelectedCode] = useState<string | null>(null);
  const [agreed, setAgreed] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [creating, setCreating] = useState(false);

  const load = useCallback(() => {
    setIsLoading(true);
    setLoadFailed(false);
    void Promise.all([
      service.listTokenBankPlans(),
      getBalance ? getBalance().catch(() => null) : Promise.resolve(null),
    ])
      .then(([planList, balanceValue]) => {
        setPlans(planList);
        setBalance(balanceValue);
      })
      .catch(() => setLoadFailed(true))
      .finally(() => setIsLoading(false));
  }, [getBalance, service]);

  useEffect(() => {
    load();
  }, [load]);

  /**
   * 下单后跳转到宿主收银台路由（移动端在收银台内完成环境适配的支付，
   * 不再在本页展示二维码）。
   */
  const handlePay = async () => {
    if (!selectedCode || !agreed || creating) {
      return;
    }
    setCreating(true);
    try {
      const result = await service.createRechargeOrder(selectedCode);
      if (result.orderId) {
        navigate(cashierPath.replace(":orderId", encodeURIComponent(result.orderId)));
      }
    } finally {
      setCreating(false);
    }
  };

  const selectedPlan = plans.find((plan) => plan.planCode === selectedCode) ?? null;

  return (
    <PageLayout title={t("subscription.token_bank_title", "算力积分购买")} bgClass="bg-[#F8F9FA] dark:bg-black">
      <div className="flex-1 overflow-y-auto p-4 pb-28">
          {isLoading ? (
            <div className="flex items-center justify-center py-20 text-[14px] text-text-sub">
              {t("subscription.loading_plans", "正在加载充值套餐...")}
            </div>
          ) : loadFailed ? (
            <button type="button" className="flex w-full items-center justify-center py-20 text-[14px] text-primary-blue" onClick={load}>
              {t("subscription.load_failed", "套餐加载失败，点击重试")}
            </button>
          ) : plans.length === 0 ? (
            <div className="flex items-center justify-center py-20 text-[14px] text-text-sub">
              {t("subscription.empty_plans", "暂无可用充值套餐")}
            </div>
          ) : (
            <>
              {/* Balance */}
              {balance !== null && (
                <div className="bg-gradient-to-br from-blue-600 to-indigo-700 dark:from-blue-900/80 dark:to-indigo-950 rounded-2xl p-5 text-white mb-4">
                  <div className="flex items-center gap-2 text-white/90 mb-2">
                    <Wallet className="w-5 h-5" strokeWidth={1.5} />
                    <span className="text-[15px] font-bold">{t("subscription.token_bank_balance", "我的算力积分")}</span>
                  </div>
                  <div className="text-[32px] font-bold font-mono leading-none">
                    {Number(balance).toLocaleString("zh-CN")} {t("subscription.points_unit", "算力积分")}
                  </div>
                </div>
              )}

              {/* Plans */}
              <div className="grid grid-cols-2 gap-3">
                {plans.map((plan) => {
                  const active = plan.planCode === selectedCode;
                  return (
                    <button
                      key={plan.planCode}
                      type="button"
                      onClick={() => setSelectedCode(plan.planCode)}
                      className={`relative rounded-xl p-4 text-left border transition-colors ${
                        active
                          ? "border-primary-blue bg-primary-blue/5"
                          : "border-border-color bg-chat-other-bg"
                      }`}
                    >
                      {active && (
                        <span className="absolute top-2 right-2 w-5 h-5 rounded-full bg-primary-blue text-white flex items-center justify-center">
                          <Check className="w-3 h-3" strokeWidth={3} />
                        </span>
                      )}
                      <div className="text-[15px] font-bold text-text-main">{plan.displayName}</div>
                      <div className="mt-1 text-[13px] text-text-sub">
                        {t("subscription.grant", "到账")} {Number(plan.grantAmount).toLocaleString("zh-CN")} {t("subscription.points_unit", "算力积分")}
                      </div>
                      {Number(plan.bonusAmount) > 0 && (
                        <div className="mt-0.5 text-[12px] text-orange-500">
                          {t("subscription.bonus", "赠送")} {Number(plan.bonusAmount).toLocaleString("zh-CN")} {t("subscription.points_unit", "算力积分")}
                        </div>
                      )}
                      <div className="mt-3 text-[18px] font-bold text-primary-blue">
                        ¥{Number(plan.priceAmount).toFixed(2)}
                      </div>
                    </button>
                  );
                })}
              </div>

              {/* Agreement */}
              <label className="flex items-start gap-2 mt-5 px-1 cursor-pointer">
                <input
                  type="checkbox"
                  checked={agreed}
                  onChange={(event) => setAgreed(event.target.checked)}
                  className="mt-0.5 accent-[#FA5151]"
                />
                <span className="text-[12px] text-text-sub leading-relaxed">
                  {t("subscription.agreement", "支付前请阅读并同意《算力积分充值服务协议》")}
                </span>
              </label>

              <p className="text-[12px] text-text-sub mt-3 text-center leading-relaxed">
                {t("subscription.token_notice", "温馨提示：算力积分不可兑换现金、不可转赠；充值后有效期以平台规则为准。")}
              </p>
            </>
          )}
        </div>

      <div className="absolute bottom-0 inset-x-0 flex items-center gap-4 p-4 bg-white dark:bg-[#1A1A1A] border-t border-border-color shadow-[0_-4px_20px_rgba(0,0,0,0.05)] pb-safe z-30">
        <div className="flex-1 min-w-0">
          {selectedPlan ? (
            <>
              <div className="text-[20px] font-bold text-primary-blue leading-none">
                ¥{Number(selectedPlan.priceAmount).toFixed(2)}
              </div>
                <div className="mt-1 text-[12px] text-text-sub truncate">
                  {selectedPlan.displayName}
                  <span className="ml-1">
                    {t("subscription.grant", "到账")} {Number(selectedPlan.grantAmount).toLocaleString("zh-CN")} {t("subscription.points_unit", "算力积分")}
                  </span>
                </div>
            </>
          ) : (
            <div className="text-[14px] text-text-sub">
              {t("subscription.select_plan_first", "请先选择充值套餐")}
            </div>
          )}
        </div>
        <button
          type="button"
          disabled={!selectedCode || !agreed || creating}
          onClick={() => void handlePay()}
          className="shrink-0 h-[46px] px-8 bg-primary-blue active:opacity-80 disabled:opacity-40 text-white font-bold text-[15px] rounded-full transition-opacity"
        >
          {creating
            ? t("subscription.creating_order", "正在创建订单...")
            : t("subscription.confirm_payment", "同意并支付")}
        </button>
      </div>
    </PageLayout>
  );
}
