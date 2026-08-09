import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckCircle2, Clock, Loader2, XCircle } from "lucide-react";
import QRCode from "qrcode";
import { detectPaymentEnvironment, formatCashierCountdown, computeCashierRemainingSeconds, type PaymentEnvironment } from "@sdkwork/order-mobile-react-orders";

export type PaymentPanelStatus = "pending" | "paid" | "expired" | "failed";

export interface PaymentPanelProps {
  qrPayload: string;
  orderId: string;
  expiresAt?: string;
  environment?: PaymentEnvironment;
  getStatus: (orderId: string) => Promise<{ status: "completed" | "failed" | "pending" }>;
  onPaid?: () => void;
}

/**
 * Reusable mobile payment panel for the order-domain H5 subscription
 * surfaces: renders the cashier QR code, counts down to expiry and polls the
 * order status until the payment settles.
 */
export function PaymentPanel({
  qrPayload,
  orderId,
  expiresAt,
  environment: environmentProp,
  getStatus,
  onPaid,
}: PaymentPanelProps) {
  const { t } = useTranslation();
  const environment = environmentProp ?? detectPaymentEnvironment();
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const [remainingSeconds, setRemainingSeconds] = useState(0);
  const [status, setStatus] = useState<PaymentPanelStatus>("pending");
  const pollTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const countdownTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const paidRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    void QRCode.toDataURL(qrPayload, { width: 220, margin: 1 }).then((dataUrl) => {
      if (!cancelled) {
        setQrDataUrl(dataUrl);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [qrPayload]);

  const stopTimers = useCallback(() => {
    if (pollTimerRef.current) {
      clearInterval(pollTimerRef.current);
      pollTimerRef.current = null;
    }
    if (countdownTimerRef.current) {
      clearInterval(countdownTimerRef.current);
      countdownTimerRef.current = null;
    }
  }, []);

  useEffect(() => {
    setStatus("pending");
    const startedAt = Date.now();
    setRemainingSeconds(computeCashierRemainingSeconds(expiresAt, startedAt, startedAt));
    pollTimerRef.current = setInterval(() => {
      void getStatus(orderId)
        .then((payment) => {
          if (payment.status === "completed" && !paidRef.current) {
            paidRef.current = true;
            stopTimers();
            setStatus("paid");
            onPaid?.();
          } else if (payment.status === "failed") {
            stopTimers();
            setStatus("failed");
          }
        })
        .catch(() => {
          // transient network errors keep polling
        });
    }, 3000);
    countdownTimerRef.current = setInterval(() => {
      setRemainingSeconds((previous) => {
        const next = computeCashierRemainingSeconds(expiresAt, startedAt, Date.now());
        if (next <= 0 && previous > 0) {
          stopTimers();
          setStatus("expired");
        }
        return next;
      });
    }, 1000);
    return stopTimers;
  }, [expiresAt, getStatus, onPaid, orderId, stopTimers]);

  if (status === "paid") {
    return (
      <div className="flex flex-col items-center justify-center py-10 text-center">
        <CheckCircle2 className="w-16 h-16 text-green-500 mb-3" />
        <p className="text-[16px] font-semibold text-text-main">
          {t("subscription.payment_completed", "支付完成")}
        </p>
        <p className="text-[13px] text-text-sub mt-1">
          {t("subscription.payment_completed_desc", "权益已到账，请返回查看")}
        </p>
      </div>
    );
  }

  if (status === "expired") {
    return (
      <div className="flex flex-col items-center justify-center py-10 text-center">
        <XCircle className="w-16 h-16 text-text-sub mb-3" />
        <p className="text-[16px] font-semibold text-text-main">
          {t("subscription.payment_expired", "订单已过期")}
        </p>
        <p className="text-[13px] text-text-sub mt-1">
          {t("subscription.payment_expired_desc", "请重新创建订单后继续支付")}
        </p>
      </div>
    );
  }

  if (status === "failed") {
    return (
      <div className="flex flex-col items-center justify-center py-10 text-center">
        <XCircle className="w-16 h-16 text-[#FA5151] mb-3" />
        <p className="text-[16px] font-semibold text-text-main">
          {t("subscription.payment_failed", "支付失败")}
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center py-6">
      {qrDataUrl ? (
        <div className="bg-white p-3 rounded-lg border border-border-color/60">
          <img src={qrDataUrl} alt={t("subscription.payment_qr", "支付二维码")} className="w-[220px] h-[220px]" />
        </div>
      ) : (
        <div className="flex flex-col items-center gap-2 py-8">
          <Loader2 className="w-10 h-10 text-text-sub/60 animate-spin" />
          <span className="text-[13px] text-text-sub">
            {t("subscription.payment_creating", "正在生成支付二维码...")}
          </span>
        </div>
      )}

      <p className="text-[13px] text-text-sub mt-4">
        {environment === "wechat"
          ? t("subscription.payment_tip_wechat", "请长按识别二维码完成支付")
          : environment === "alipay"
            ? t("subscription.payment_tip_alipay", "请使用支付宝扫一扫完成支付")
            : t("subscription.payment_tip_scan", "请使用微信或支付宝扫一扫完成支付")}
      </p>

      <div className="flex items-center gap-1 mt-3 text-[#FA5151]">
        <Clock className="w-4 h-4" />
        <span className="text-[13px] font-medium">
          {t("subscription.payment_countdown", "剩余 {{countdown}} 自动关闭", {
            countdown: formatCashierCountdown(remainingSeconds),
          })}
        </span>
      </div>
    </div>
  );
}
