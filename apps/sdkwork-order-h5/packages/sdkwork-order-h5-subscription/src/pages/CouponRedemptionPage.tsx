import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Ticket, CheckCircle2, Loader2 } from "lucide-react";
import { PageLayout } from "@sdkwork/ui-mobile-react";

import {
  createSubscriptionPurchaseService,
  type CouponRedemptionResult,
  type SubscriptionPurchasePort,
} from "../services/SubscriptionPurchaseService";

export interface CouponRedemptionPageProps {
  service?: SubscriptionPurchasePort;
  onBalanceChanged?: () => void;
}

export function CouponRedemptionPage({
  service: serviceProp,
  onBalanceChanged,
}: CouponRedemptionPageProps) {
  const { t } = useTranslation();
  const service = serviceProp ?? createSubscriptionPurchaseService();
  const [code, setCode] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isRedeeming, setIsRedeeming] = useState(false);
  const [result, setResult] = useState<CouponRedemptionResult | null>(null);

  async function handleRedeem() {
    const normalizedCode = code.trim();
    if (!normalizedCode) {
      setError(t("subscription.coupon_code_required", "请输入兑换码"));
      return;
    }
    if (isRedeeming) {
      return;
    }
    setError(null);
    setIsRedeeming(true);
    try {
      const next = await service.redeemCoupon(normalizedCode);
      setResult(next);
      onBalanceChanged?.();
    } catch (cause) {
      setError(
        cause instanceof Error
          ? cause.message
          : t("subscription.coupon_redeem_failed", "兑换失败，请稍后重试"),
      );
    } finally {
      setIsRedeeming(false);
    }
  }

  return (
    <PageLayout title={t("subscription.coupon_title", "优惠券兑换")} bgClass="bg-bg-color">
      <div className="flex-1 overflow-y-auto p-4">
        <div className="bg-gradient-to-br from-emerald-500 to-teal-600 dark:from-emerald-900/80 dark:to-teal-950 rounded-2xl p-6 text-white mb-4">
          <div className="flex items-center gap-2 text-white/90 mb-2">
            <Ticket className="w-5 h-5" strokeWidth={1.5} />
            <span className="text-[15px] font-bold">
              {t("subscription.coupon_banner_title", "兑换优惠券")}
            </span>
          </div>
          <p className="text-[13px] text-white/70 leading-relaxed">
            {t(
              "subscription.coupon_banner_desc",
              "输入兑换码，激活会员权益或向 Token Bank 存入算力额度。",
            )}
          </p>
        </div>

        {result ? (
          <div className="bg-chat-other-bg rounded-2xl p-6 border border-border-color flex flex-col items-center">
            <CheckCircle2 className="w-12 h-12 text-emerald-500 mb-3" strokeWidth={1.5} />
            <h3 className="text-[16px] font-bold text-text-main mb-1">
              {t("subscription.coupon_redeemed", "兑换成功")}
            </h3>
            <p className="text-[13px] text-text-sub text-center leading-relaxed">
              {result.benefitKind === "token_bank_credit" && result.grantAmount
                ? t("subscription.coupon_token_bank_credited", "算力额度已存入 Token Bank：{{amount}} T", {
                    amount: Number(result.grantAmount).toLocaleString("zh-CN"),
                  })
                : result.benefitKind === "subscription"
                  ? t("subscription.coupon_subscription_activated", "会员订阅已激活")
                  : t("subscription.coupon_credited", "权益已发放")}
            </p>
            <button
              type="button"
              onClick={() => {
                setResult(null);
                setCode("");
              }}
              className="w-full mt-5 bg-primary-blue active:opacity-80 text-white font-medium text-[15px] py-3 rounded-xl"
            >
              {t("subscription.coupon_redeem_again", "继续兑换")}
            </button>
          </div>
        ) : (
          <>
            <div className="bg-chat-other-bg rounded-2xl p-5 border border-border-color">
              <label className="text-[14px] font-medium text-text-main mb-2 block">
                {t("subscription.coupon_code_label", "兑换码")}
              </label>
              <input
                value={code}
                onChange={(event) => setCode(event.target.value)}
                placeholder={t("subscription.coupon_code_placeholder", "请输入兑换码")}
                className="w-full rounded-xl border border-border-color bg-chat-other-bg px-4 py-3 text-[15px] text-text-main outline-none focus:border-primary-blue"
              />
              {error && (
                <p className="mt-2 text-[13px] text-red-500">{error}</p>
              )}
              <button
                type="button"
                disabled={isRedeeming || !code.trim()}
                onClick={() => void handleRedeem()}
                className="w-full mt-4 bg-primary-blue active:opacity-80 disabled:opacity-40 text-white font-medium text-[15px] py-3 rounded-xl flex items-center justify-center gap-2"
              >
                {isRedeeming ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" strokeWidth={2.5} />
                    {t("subscription.coupon_redeeming", "正在兑换...")}
                  </>
                ) : (
                  t("subscription.coupon_redeem", "立即兑换")
                )}
              </button>
            </div>

            <p className="text-[12px] text-text-sub mt-3 text-center leading-relaxed">
              {t(
                "subscription.coupon_notice",
                "温馨提示：兑换码激活的权益以平台规则为准，不可转赠、不可提现。",
              )}
            </p>
          </>
        )}
      </div>
    </PageLayout>
  );
}
