import React, { useRef, useState } from "react";
import { createSdkworkIdempotencyParams } from "@sdkwork/order-service";
import { useTranslation } from "react-i18next";
import { Check, CheckCircle2, Copy, Loader2, RotateCw, Ticket } from "lucide-react";
import { PageLayout } from "@sdkwork/ui-mobile-react";

import {
  classifyCouponRedemptionFailure,
  createSubscriptionPurchaseService,
  type CouponRedemptionFailure,
  type CouponRedemptionFailureKind,
  type CouponRedemptionResult,
  type SubscriptionPurchasePort,
} from "../services/SubscriptionPurchaseService";

/** Cash grants are minor units of the coupon's own currency. */
function formatCashGrant(minorUnits: number, currencyCode?: string): string {
  const amount = (minorUnits / 100).toFixed(2);
  const currency = (currencyCode ?? "CNY").trim().toUpperCase();
  return currency === "CNY" ? `¥${amount}` : `${amount} ${currency}`;
}

export interface CouponRedemptionPageProps {
  service?: SubscriptionPurchasePort;
  onBalanceChanged?: () => void;
}

/**
 * One customer-visible message per failure family: a retryable post-consumption
 * delivery failure, a bad/expired code, and an already-consumed coupon are
 * never collapsed into one generic "redemption failed" line.
 */
const FAILURE_COPY: Record<CouponRedemptionFailureKind, { key: string; fallback: string }> = {
  benefit_delivery_failed: {
    key: "subscription.coupon_benefit_delivery_failed",
    fallback: "券已核销，但权益发放失败；请重试，重试不会重复扣券。",
  },
  invalid_or_expired: {
    key: "subscription.coupon_invalid_or_expired",
    fallback: "兑换码无效或已过期，请检查兑换码。",
  },
  already_used: {
    key: "subscription.coupon_already_used",
    fallback: "该优惠券已被使用。",
  },
  request_failed: {
    key: "subscription.coupon_request_failed",
    fallback: "兑换请求未完成，请稍后重试。",
  },
};

export function CouponRedemptionPage({
  service: serviceProp,
  onBalanceChanged,
}: CouponRedemptionPageProps) {
  const { t } = useTranslation();
  const service = serviceProp ?? createSubscriptionPurchaseService();
  const [code, setCode] = useState("");
  const [codeError, setCodeError] = useState<string | null>(null);
  const [failure, setFailure] = useState<CouponRedemptionFailure | null>(null);
  const [isRedeeming, setIsRedeeming] = useState(false);
  const [result, setResult] = useState<CouponRedemptionResult | null>(null);
  const [traceIdCopied, setTraceIdCopied] = useState(false);

  /**
   * Idempotency intent for the redemption the customer confirmed: the key is
   * derived once for the entered code and kept for every retry of that same
   * intent, so a retry after a post-consumption delivery failure replays the
   * original redemption instead of consuming another coupon. A different code
   * (or a completed redemption) starts a new intent and therefore a new key.
   */
  const intentRef = useRef<{ code: string; idempotencyKey: string } | null>(null);

  function idempotencyKeyFor(normalizedCode: string): string {
    if (intentRef.current?.code !== normalizedCode) {
      intentRef.current = {
        code: normalizedCode,
        idempotencyKey: createSdkworkIdempotencyParams().idempotencyKey,
      };
    }
    return intentRef.current.idempotencyKey;
  }

  async function submit() {
    const normalizedCode = code.trim();
    if (!normalizedCode) {
      setCodeError(t("subscription.coupon_code_required", "请输入兑换码"));
      return;
    }
    if (isRedeeming) {
      return;
    }
    setCodeError(null);
    setFailure(null);
    setTraceIdCopied(false);
    setIsRedeeming(true);
    try {
      const next = await service.redeemCoupon(normalizedCode, idempotencyKeyFor(normalizedCode));
      intentRef.current = null;
      setResult(next);
      onBalanceChanged?.();
    } catch (cause) {
      setFailure(classifyCouponRedemptionFailure(cause));
    } finally {
      setIsRedeeming(false);
    }
  }

  async function copyTraceId(traceId: string) {
    const clipboard = navigator.clipboard;
    if (!clipboard) {
      return;
    }
    try {
      await clipboard.writeText(traceId);
      setTraceIdCopied(true);
      window.setTimeout(() => setTraceIdCopied(false), 2000);
    } catch {
      // Clipboard permission denied: the trace id stays visible and selectable.
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
                ? t("subscription.coupon_token_bank_credited", "算力额度已存入 Token Bank：{{amount}}", {
                    amount: Number(result.grantAmount).toLocaleString("zh-CN"),
                  })
                : result.benefitKind === "points_credit" && result.grantAmount
                  ? t("subscription.coupon_points_credited", "会员积分已发放：{{amount}}", {
                      amount: Number(result.grantAmount).toLocaleString("zh-CN"),
                    })
                  : result.benefitKind === "cash_credit" && result.grantAmount
                    ? t("subscription.coupon_cash_credited", "现金已发放：{{amount}}", {
                        amount: formatCashGrant(Number(result.grantAmount), result.currencyCode),
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
                onChange={(event) => {
                  setCode(event.target.value);
                  setCodeError(null);
                  setFailure(null);
                  setTraceIdCopied(false);
                }}
                placeholder={t("subscription.coupon_code_placeholder", "请输入兑换码")}
                className="w-full rounded-xl border border-border-color bg-chat-other-bg px-4 py-3 text-[15px] text-text-main outline-none focus:border-primary-blue"
              />
              {codeError && (
                <p className="mt-2 text-[13px] text-red-500">{codeError}</p>
              )}
              {failure && (
                <CouponRedemptionFailureCard
                  copied={traceIdCopied}
                  failure={failure}
                  isRetrying={isRedeeming}
                  onCopy={() => void copyTraceId(failure.traceId ?? "")}
                  onRetry={() => void submit()}
                />
              )}
              <button
                type="button"
                disabled={isRedeeming || !code.trim()}
                onClick={() => void submit()}
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

function CouponRedemptionFailureCard({
  copied,
  failure,
  isRetrying,
  onCopy,
  onRetry,
}: {
  copied: boolean;
  failure: CouponRedemptionFailure;
  isRetrying: boolean;
  onCopy: () => void;
  onRetry: () => void;
}) {
  const { t } = useTranslation();
  const copy = FAILURE_COPY[failure.kind];
  return (
    <div
      role="alert"
      className="mt-2 rounded-xl border border-red-200 bg-red-50 p-3 dark:border-red-900/60 dark:bg-red-950/40"
    >
      <p className="text-[13px] font-medium leading-relaxed text-red-600 dark:text-red-300">
        {t(copy.key, copy.fallback)}
      </p>
      {failure.retryable && (
        <p className="mt-1 text-[12px] leading-relaxed text-red-500 dark:text-red-400">
          {t("subscription.coupon_support_hint", "如多次失败，请联系客服并提供追踪号")}
        </p>
      )}
      {failure.traceId && (
        <div className="mt-2 flex flex-wrap items-center gap-2 text-[12px] text-red-500 dark:text-red-400">
          <span>{t("subscription.coupon_trace_id", "追踪号")}</span>
          <code className="rounded bg-white/70 px-1.5 py-0.5 font-mono text-[11px] text-text-main dark:bg-white/10">
            {failure.traceId}
          </code>
          <button
            type="button"
            onClick={onCopy}
            className="inline-flex items-center gap-1 underline"
          >
            {copied ? (
              <Check className="w-3 h-3" strokeWidth={2.5} />
            ) : (
              <Copy className="w-3 h-3" strokeWidth={2.5} />
            )}
            {copied
              ? t("subscription.coupon_trace_id_copied", "已复制")
              : t("subscription.coupon_trace_id_copy", "复制")}
          </button>
        </div>
      )}
      {failure.retryable && (
        <button
          type="button"
          disabled={isRetrying}
          onClick={onRetry}
          className="mt-3 inline-flex items-center gap-2 bg-primary-blue active:opacity-80 disabled:opacity-40 text-white font-medium text-[13px] px-3 py-2 rounded-lg"
        >
          <RotateCw className="w-3.5 h-3.5" strokeWidth={2.5} />
          {t("subscription.coupon_retry", "重试")}
        </button>
      )}
    </div>
  );
}
