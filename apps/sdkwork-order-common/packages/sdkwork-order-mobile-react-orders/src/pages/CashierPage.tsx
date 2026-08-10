import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams, useSearchParams } from "react-router";
import {
  AlertCircle,
  CheckCircle2,
  Clock,
  ExternalLink,
  Loader2,
  QrCode,
  Smartphone,
  Store,
  XCircle,
} from "lucide-react";
import QRCode from "qrcode";
import { PageLayout, showToast } from "@sdkwork/ui-mobile-react";

import {
  formatAmountCny,
  OrderService,
  ORDER_PAYMENT_METHOD_LABELS,
  resolveAvailablePaymentMethods,
  type Order,
  type OrderPaymentMethod,
  type PaymentSession,
} from "../services/OrderService";
import { toUserErrorMessage } from "../services/errorMessage";
import {
  CASHIER_POLL_INTERVAL_MS,
  computeCashierRemainingSeconds,
  formatCashierCountdown,
  resolveCashierPhaseFromPaymentStatus,
  resolveCashierWireMethod,
} from "../services/CashierLogic";
import type { CashierPhase } from "../services/CashierTypes";
import {
  detectPaymentEnvironment,
  detectPaymentRegion,
  isMobileDevice,
  type PaymentEnvironment,
  type PaymentRegion,
} from "../services/PaymentEnvironment";
import {
  buildCashierOAuthRedirect,
  parseWechatOAuthCallbackParams,
  stripWechatOAuthCallbackParams,
} from "../services/WechatPaymentOAuth";
import {
  invokeWechatJsapiPayment,
  isWechatJsapiResultCancelled,
  isWechatJsapiResultOk,
} from "../services/WechatJsapiInvoker";
import {
  ORDER_MOBILE_ROUTE_DEFINITIONS,
  resolveHostRoutePath,
} from "../routes";

/** Host-overridable order route templates (paths with `:orderId`). */
export interface CashierPageProps {
  orderDetailPath?: string;
  orderCenterPath?: string;
}

/** Brand presentation per payment method (badge mark + label/desc keys). */
const PAYMENT_METHOD_META: Readonly<
  Record<OrderPaymentMethod, { badge: string; badgeClass: string; descKey: string; descDefault: string }>
> = {
  wechat_pay: {
    badge: "微",
    badgeClass: "bg-[#07C160]",
    descKey: "orders.payment_method_desc_wechat_pay",
    descDefault: "微信扫码支付",
  },
  wechat_jsapi: {
    badge: "微",
    badgeClass: "bg-[#07C160]",
    descKey: "orders.payment_method_desc_wechat_pay",
    descDefault: "微信扫码支付",
  },
  alipay: {
    badge: "支",
    badgeClass: "bg-[#1677FF]",
    descKey: "orders.payment_method_desc_alipay",
    descDefault: "支付宝扫码支付",
  },
  alipay_wap: {
    badge: "支",
    badgeClass: "bg-[#1677FF]",
    descKey: "orders.payment_method_desc_alipay",
    descDefault: "支付宝扫码支付",
  },
  balance: {
    badge: "余",
    badgeClass: "bg-orange-500",
    descKey: "orders.payment_method_desc_balance",
    descDefault: "余额直接支付",
  },
};

export function CashierPage({
  orderDetailPath = ORDER_MOBILE_ROUTE_DEFINITIONS.orderDetail.path,
  orderCenterPath = ORDER_MOBILE_ROUTE_DEFINITIONS.orderCenter.path,
}: CashierPageProps) {
  const { orderId } = useParams<{ orderId: string }>();
  const navigate = useNavigate();
  const { t, i18n } = useTranslation();
  const [searchParams, setSearchParams] = useSearchParams();

  /** Payment environment is stable for the page lifetime (UA-based). */
  const environment = useMemo<PaymentEnvironment>(() => detectPaymentEnvironment(), []);
  /** Deployment region: narrows the offered payment methods. */
  const region = useMemo<PaymentRegion>(() => detectPaymentRegion(), []);
  /** Mobile payers jump to the provider H5 cashier instead of QR scanning. */
  const isMobile = useMemo<boolean>(() => isMobileDevice(), []);
  const availableMethods = useMemo(
    () => resolveAvailablePaymentMethods(environment, region),
    [environment, region],
  );
  const [paymentMethod, setPaymentMethod] = useState<OrderPaymentMethod>(
    () => availableMethods[0] ?? "wechat_pay",
  );
  /** Payer openid recovered from the WeChat OAuth callback (URL query). */
  const [openid, setOpenid] = useState<string | null>(() => {
    const params = parseWechatOAuthCallbackParams(searchParams);
    return params.openid ?? null;
  });
  const openidRef = useRef<string | null>(openid);
  useEffect(() => {
    openidRef.current = openid;
  }, [openid]);

  const [phase, setPhase] = useState<CashierPhase>("loading");
  const [order, setOrder] = useState<Order | null>(null);
  const [paymentSession, setPaymentSession] = useState<PaymentSession | null>(null);
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  /** Alipay in-app QR link (qr.alipay.com) that opens the payer page. */
  const [qrLaunchUrl, setQrLaunchUrl] = useState<string | null>(null);
  /** Provider H5 cashier URL for mobile browsers (WeChat H5 / Alipay WAP). */
  const [h5CashierUrl, setH5CashierUrl] = useState<string | null>(null);
  const [remainingSeconds, setRemainingSeconds] = useState(0);
  const [errorMessageText, setErrorMessageText] = useState<string | null>(null);
  /** Non-fatal launch feedback (e.g. JSAPI cancelled / bridge unavailable). */
  const [launchNotice, setLaunchNotice] = useState<string | null>(null);

  const paymentCreatedAtRef = useRef(0);
  const pollTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const countdownTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const phaseRef = useRef<CashierPhase>("loading");
  phaseRef.current = phase;
  const orderRef = useRef<Order | null>(null);

  /** Cleans the WeChat OAuth callback parameters once, after they are read. */
  useEffect(() => {
    const params = parseWechatOAuthCallbackParams(searchParams);
    if (params.openid || params.error) {
      setSearchParams(stripWechatOAuthCallbackParams(searchParams), { replace: true });
    }
  }, [searchParams, setSearchParams]);

  const clearTimers = useCallback(() => {
    if (pollTimerRef.current) {
      clearInterval(pollTimerRef.current);
      pollTimerRef.current = null;
    }
    if (countdownTimerRef.current) {
      clearInterval(countdownTimerRef.current);
      countdownTimerRef.current = null;
    }
  }, []);

  const stopCashier = useCallback((nextPhase: CashierPhase) => {
    clearTimers();
    setPhase(nextPhase);
  }, [clearTimers]);

  const pollPaymentStatus = useCallback(async (targetOrderId: string) => {
    try {
      const status = await OrderService.getPaymentStatus(targetOrderId);
      const currentOrder = await OrderService.getOrderById(targetOrderId);
      const resolved = resolveCashierPhaseFromPaymentStatus(
        status,
        currentOrder?.status ?? status.status,
      );
      if (resolved !== "pending") {
        stopCashier(resolved);
        if (resolved === "paid") {
          setOrder(currentOrder);
        }
      }
    } catch {
      // Transient network errors keep the cashier pending; the next poll
      // retries. Permanent failures surface through the countdown expiry.
    }
  }, [stopCashier]);

  const startPolling = useCallback((targetOrderId: string) => {
    clearTimers();
    pollTimerRef.current = setInterval(() => {
      void pollPaymentStatus(targetOrderId);
    }, CASHIER_POLL_INTERVAL_MS);
    countdownTimerRef.current = setInterval(() => {
      const now = Date.now();
      setRemainingSeconds((previous) => {
        const next = computeCashierRemainingSeconds(
          orderRef.current?.expireTime,
          paymentCreatedAtRef.current || now,
          now,
        );
        if (next <= 0 && previous > 0 && phaseRef.current === "pending") {
          // One final authoritative check before declaring expiry.
          void pollPaymentStatus(targetOrderId).then(() => {
            if (phaseRef.current === "pending") {
              stopCashier("expired");
            }
          });
        }
        return next;
      });
    }, 1000);
  }, [clearTimers, pollPaymentStatus, stopCashier]);

  /** Renders the payment QR from the session payload, if any. */
  const renderQrCode = useCallback(
    async (params: Readonly<Record<string, string>>): Promise<string | null> => {
      const payload = params.qrCodePayload ?? params.qrCodeUrl ?? params.cashierUrl;
      if (!payload) {
        return null;
      }
      return QRCode.toDataURL(payload, { width: 220, margin: 1 });
    },
    [],
  );

  /** Reacts to a payment session: JSAPI invoke, WAP redirect or QR render. */
  const handlePaymentSession = useCallback(
    async (targetOrderId: string, session: PaymentSession) => {
      setPaymentSession(session);
      setLaunchNotice(null);
      setH5CashierUrl(null);
      paymentCreatedAtRef.current = Date.now();
      setRemainingSeconds(computeCashierRemainingSeconds(
        orderRef.current?.expireTime,
        paymentCreatedAtRef.current,
        Date.now(),
      ));
      const params = session.paymentParams;

      // WeChat app: JSAPI payload → invoke the WeChat bridge. On cancel or
      // bridge failure the payer retries in-app; no QR is rendered on mobile.
      if (environment === "wechat" && params.jsapiPayload) {
        setPhase("pending");
        startPolling(targetOrderId);
        try {
          const jsapiPayload = JSON.parse(params.jsapiPayload) as Record<string, unknown>;
          const result = await invokeWechatJsapiPayment(jsapiPayload);
          if (isWechatJsapiResultCancelled(result)) {
            setLaunchNotice(t("orders.cashier_jsapi_cancelled", "已取消支付，可重新唤起"));
          } else if (!isWechatJsapiResultOk(result)) {
            setLaunchNotice(t("orders.cashier_jsapi_failed", "唤起微信支付失败，请重试"));
          }
        } catch {
          setLaunchNotice(t("orders.cashier_jsapi_failed", "唤起微信支付失败，请重试"));
        }
        return;
      }

      // Alipay app: WAP redirect jumps to the Alipay H5 cashier in place
      // (the Alipay webview requires in-page navigation for WAP payments).
      if (environment === "alipay" && params.payUrl) {
        setPhase("pending");
        window.location.assign(params.payUrl);
        return;
      }

      // Alipay app without a WAP link: the qr.alipay.com URL opens the payer
      // page directly inside the Alipay app via the "去支付" button.
      if (environment === "alipay" && params.qrCodeUrl) {
        setPhase("pending");
        setQrLaunchUrl(params.qrCodeUrl);
        startPolling(targetOrderId);
        return;
      }

      // Mobile browser: jump to the provider H5 cashier (WeChat H5 pay /
      // Alipay WAP / provider cashier URL). QR scanning is desktop-only.
      if (isMobile) {
        const h5Url = params.payUrl ?? params.cashierUrl;
        setPhase("pending");
        setQrLaunchUrl(null);
        setQrDataUrl(null);
        if (h5Url) {
          setH5CashierUrl(h5Url);
          startPolling(targetOrderId);
          window.location.assign(h5Url);
        } else {
          // No H5 channel configured for this deployment: guide the payer
          // into the WeChat/Alipay client instead of showing a scan QR.
          setH5CashierUrl(null);
          startPolling(targetOrderId);
        }
        return;
      }

      // Desktop browser: render the QR code for scanning.
      setPhase("pending");
      setQrLaunchUrl(null);
      setQrDataUrl(await renderQrCode(params));
      startPolling(targetOrderId);
    },
    [environment, isMobile, renderQrCode, startPolling, t],
  );

  const createPayment = useCallback(async (targetOrderId: string, method: OrderPaymentMethod) => {
    setPhase("creating");
    setErrorMessageText(null);
    setLaunchNotice(null);
    try {
      // WeChat app without a payer openid: acquire it through the IAM
      // payment OAuth flow; the callback returns to this cashier URL.
      if (environment === "wechat" && method === "wechat_pay" && !openidRef.current) {
        setPhase("oauth_waiting");
        const redirect = buildCashierOAuthRedirect(window.location);
        const authorizeUrl = await OrderService.fetchWechatOAuthAuthorizeUrl(redirect);
        window.location.assign(authorizeUrl);
        return;
      }
      const wireMethod = resolveCashierWireMethod(
        environment,
        method,
        Boolean(openidRef.current),
      );
      const session = await OrderService.payOrder(targetOrderId, wireMethod, {
        openid: openidRef.current ?? undefined,
      });
      await handlePaymentSession(targetOrderId, session);
    } catch (error) {
      // Payment creation failed: stay on the cashier UI so the payer can
      // switch method or retry. The provider error surfaces as a toast.
      const message = toUserErrorMessage(t, error);
      setErrorMessageText(message);
      setPaymentSession(null);
      setQrDataUrl(null);
      setQrLaunchUrl(null);
      setH5CashierUrl(null);
      setRemainingSeconds(0);
      showToast(message);
      stopCashier("pending");
    }
  }, [environment, handlePaymentSession, stopCashier, t]);

  useEffect(() => {
    if (!orderId) {
      setPhase("not_payable");
      return undefined;
    }
    let cancelled = false;
    void (async () => {
      try {
        const loaded = await OrderService.getOrderById(orderId);
        if (cancelled) {
          return;
        }
        if (!loaded) {
          setPhase("not_payable");
          return;
        }
        orderRef.current = loaded;
        setOrder(loaded);
        if (loaded.status === "pending_payment") {
          await createPayment(orderId, availableMethods[0] ?? "wechat_pay");
        } else if (
          ["paid", "fulfilled", "completed", "refunding", "refunded"].includes(String(loaded.status))
        ) {
          setPhase("paid");
        } else if (loaded.status === "cancelled") {
          setPhase("cancelled");
        } else if (loaded.status === "expired") {
          setPhase("expired");
        } else {
          setPhase("not_payable");
        }
      } catch (error) {
        if (!cancelled) {
          setErrorMessageText(toUserErrorMessage(t, error));
          setPhase("failed");
        }
      }
    })();
    return () => {
      cancelled = true;
      clearTimers();
    };
  }, [orderId, clearTimers, createPayment, availableMethods, t]);

  const changePaymentMethod = (method: OrderPaymentMethod) => {
    if (phase !== "pending" || !orderId) {
      return;
    }
    setPaymentMethod(method);
    setQrDataUrl(null);
    setQrLaunchUrl(null);
    setH5CashierUrl(null);
    void createPayment(orderId, method);
  };

  const retryPayment = () => {
    if (orderId) {
      void createPayment(orderId, paymentMethod);
    }
  };

  /** Re-launches the provider H5 cashier if the auto redirect was blocked. */
  const launchH5Cashier = () => {
    if (h5CashierUrl) {
      window.location.assign(h5CashierUrl);
    }
  };

  /** Re-invokes the WeChat bridge after a cancelled/failed JSAPI launch. */
  const retryLaunchPayment = () => {
    setLaunchNotice(null);
    retryPayment();
  };

  const cancelPayment = async () => {
    if (!orderId) {
      return;
    }
    try {
      await OrderService.cancelOrder(orderId);
      showToast(t("orders.cancelled_toast", "订单已取消"));
      stopCashier("cancelled");
    } catch (error) {
      showToast(toUserErrorMessage(t, error));
    }
  };

  const openExternalCashier = () => {
    const url = paymentSession?.paymentParams.cashierUrl;
    if (url) {
      window.open(url, "_blank", "noopener,noreferrer");
    }
  };

  const launchQrPayment = () => {
    if (qrLaunchUrl) {
      window.location.assign(qrLaunchUrl);
    }
  };

  const goToOrderDetail = () => {
    if (orderId) {
      navigate(resolveHostRoutePath(orderDetailPath, { orderId }));
    }
  };

  const goToOrderCenter = () => {
    navigate(resolveHostRoutePath(orderCenterPath));
  };

  const renderResult = (
    title: string,
    description: string,
    icon: React.ReactNode,
    extraActions?: React.ReactNode,
  ) => (
    <div className="flex flex-col items-center justify-center flex-1 px-8 text-center">
      <div className="mb-5">{icon}</div>
      <h2 className="text-[18px] font-bold text-text-main mb-2">{title}</h2>
      <p className="text-[13px] text-text-sub leading-relaxed">{description}</p>
      <div className="flex flex-col gap-3 w-full max-w-[280px] mt-8">
        {extraActions}
        <button
          onClick={goToOrderDetail}
          className="w-full bg-primary-blue active:opacity-80 text-white font-medium text-[15px] py-3 rounded-xl transition-opacity"
        >
          {t("orders.cashier_view_order", "查看订单")}
        </button>
        <button
          onClick={goToOrderCenter}
          className="w-full border border-border-color active:bg-active-bg text-text-main font-medium text-[15px] py-3 rounded-xl transition-colors"
        >
          {t("orders.cashier_back_center", "返回订单中心")}
        </button>
      </div>
    </div>
  );

  const renderPayTip = () => {
    if (environment === "wechat") {
      if (launchNotice) {
        return launchNotice;
      }
      return t("orders.cashier_pay_tip_wechat_jsapi", "正在唤起微信支付，请在微信中完成支付");
    }
    if (environment === "alipay") {
      return t("orders.cashier_pay_tip_alipay", "点击下方按钮跳转支付宝完成支付");
    }
    if (isMobile) {
      return h5CashierUrl
        ? t("orders.cashier_jumping", "正在跳转收银台…")
        : t("orders.cashier_mobile_open_app", "请在微信或支付宝中打开本页面完成支付");
    }
    return t("orders.cashier_scan_pay", "请使用微信或支付宝扫一扫完成支付");
  };

  return (
    <PageLayout title={t("orders.cashier_title", "收银台")}>
      <div className="flex flex-col h-full bg-[#f5f6f8] dark:bg-[#1a1b1c]">
        {phase === "loading" || phase === "creating" || phase === "oauth_waiting" ? (
          <div className="flex flex-col items-center justify-center flex-1 text-text-sub">
            <Loader2 className="w-8 h-8 animate-spin mb-3" />
            <p className="text-[14px]">
              {phase === "oauth_waiting"
                ? t("orders.cashier_oauth_waiting", "正在获取微信支付授权…")
                : phase === "loading"
                  ? t("orders.loading", "加载中...")
                  : t("orders.cashier_creating", "正在创建支付...")}
            </p>
          </div>
        ) : phase === "pending" && order ? (
          <>
            <div className="bg-white dark:bg-[#1E1E1E] m-4 rounded-xl shadow-sm flex flex-col items-center py-6 px-4">
              <span className="text-[13px] text-text-sub mb-1">
                {t("orders.cashier_amount_label", "支付金额")}
              </span>
              <span className="text-[30px] font-bold text-text-main mb-4">
                {formatAmountCny(
                  paymentSession?.amount ?? order.totalAmount,
                  order.currencyCode,
                  i18n.language,
                )}
              </span>

              <div className="flex items-center gap-1 text-text-sub mb-4">
                <Store className="w-4 h-4" />
                <span className="text-[13px]">{order.subject}</span>
              </div>

              {paymentSession ? (
                qrDataUrl ? (
                  <div className="bg-white p-3 rounded-lg border border-border-color/60">
                    <img src={qrDataUrl} alt={t("orders.cashier_qr_alt", "收银台二维码")} className="w-[220px] h-[220px]" />
                  </div>
                ) : isMobile && h5CashierUrl ? (
                  <div className="flex flex-col items-center gap-3 py-4">
                    <Loader2 className="w-10 h-10 text-primary-blue animate-spin" />
                    <span className="text-[13px] text-text-sub">
                      {t("orders.cashier_jumping", "正在跳转收银台…")}
                    </span>
                    <button
                      onClick={launchH5Cashier}
                      className="w-full max-w-[220px] bg-primary-blue active:opacity-80 text-white font-medium text-[14px] py-2.5 rounded-lg transition-opacity"
                    >
                      {t("orders.cashier_jump_fallback", "未跳转？点击继续支付")}
                    </button>
                  </div>
                ) : isMobile ? (
                  <div className="flex flex-col items-center gap-2 py-6">
                    <Smartphone className="w-12 h-12 text-text-sub/50" />
                    <span className="text-[13px] text-text-sub">
                      {t("orders.cashier_mobile_open_app", "请在微信或支付宝中打开本页面完成支付")}
                    </span>
                  </div>
                ) : (
                  <div className="flex flex-col items-center gap-2 py-6">
                    <QrCode className="w-12 h-12 text-text-sub/50" />
                    <span className="text-[13px] text-text-sub">
                      {t("orders.cashier_no_qr", "收银台暂未生成二维码")}
                    </span>
                  </div>
                )
              ) : (
                <div className="flex flex-col items-center gap-3 py-6">
                  <QrCode className="w-12 h-12 text-text-sub/50" />
                  <span className="text-[13px] text-text-sub">
                    {t("orders.cashier_pay_ready_tip", "支付尚未完成，请选择支付方式后重试")}
                  </span>
                  <button
                    onClick={retryPayment}
                    className="w-full max-w-[220px] bg-primary-blue active:opacity-80 text-white font-medium text-[14px] py-2.5 rounded-lg transition-opacity"
                  >
                    {t("orders.cashier_pay_now", "立即支付")}
                  </button>
                </div>
              )}

              {environment === "wechat" && launchNotice && (
                <button
                  onClick={retryLaunchPayment}
                  className="mt-4 w-full max-w-[220px] bg-[#07C160] active:opacity-80 text-white font-medium text-[14px] py-2.5 rounded-lg transition-opacity"
                >
                  {t("orders.cashier_retry_launch", "重新唤起支付")}
                </button>
              )}

              {environment === "alipay" && qrLaunchUrl && (
                <button
                  onClick={launchQrPayment}
                  className="mt-4 w-full max-w-[220px] bg-[#1677FF] active:opacity-80 text-white font-medium text-[14px] py-2.5 rounded-lg transition-opacity"
                >
                  {t("orders.cashier_go_pay", "去支付")}
                </button>
              )}

              {paymentSession && (
                <p className="text-[13px] text-text-sub mt-4">{renderPayTip()}</p>
              )}

              {paymentSession && (
                <div className="flex items-center gap-1 mt-3 text-[#FA5151]">
                  <Clock className="w-4 h-4" />
                  <span className="text-[13px] font-medium">
                    {t("orders.cashier_auto_close", "剩余 {countdown} 自动关闭", {
                      countdown: formatCashierCountdown(remainingSeconds),
                    })}
                  </span>
                </div>
              )}
            </div>

            <div className="bg-white dark:bg-[#1E1E1E] mx-4 rounded-xl shadow-sm p-4">
              <h3 className="text-[14px] font-bold text-text-main mb-3">
                {t("orders.cashier_pay_method", "支付方式")}
              </h3>
              <div className="flex flex-col gap-2">
                {availableMethods.map((method, index) => {
                  const meta = PAYMENT_METHOD_META[method];
                  return (
                    <label
                      key={method}
                      className="flex items-center gap-3 px-3 py-3 rounded-lg border border-border-color cursor-pointer active:bg-active-bg"
                    >
                      <span
                        className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-white text-[13px] font-bold ${meta.badgeClass}`}
                      >
                        {meta.badge}
                      </span>
                      <span className="flex-1 min-w-0">
                        <span className="flex items-center gap-1.5">
                          <span className="text-[14px] text-text-main">
                            {t(
                              `orders.payment_method_${method}`,
                              ORDER_PAYMENT_METHOD_LABELS[method] ?? method,
                            )}
                          </span>
                          {index === 0 && (
                            <span className="text-[10px] text-[#FA5151] bg-[#FA5151]/10 rounded-full px-1.5 py-0.5">
                              {t("orders.cashier_recommended", "推荐")}
                            </span>
                          )}
                        </span>
                        <span className="block text-[12px] text-text-sub truncate">
                          {t(meta.descKey, meta.descDefault)}
                        </span>
                      </span>
                      <input
                        type="radio"
                        name="payment-method"
                        checked={paymentMethod === method}
                        onChange={() => changePaymentMethod(method)}
                        className="accent-[#FA5151]"
                      />
                    </label>
                  );
                })}
              </div>
              {paymentSession && (
                <button
                  onClick={openExternalCashier}
                  className="mt-4 w-full flex items-center justify-center gap-1.5 border border-border-color active:bg-active-bg text-text-main text-[14px] py-2.5 rounded-lg transition-colors"
                >
                  <ExternalLink className="w-4 h-4" />
                  {t("orders.cashier_open_external", "打开外部收银台")}
                </button>
              )}
            </div>

            <div className="p-4 pb-6">
              <button
                onClick={() => void cancelPayment()}
                className="w-full border border-border-color active:bg-active-bg text-text-sub text-[14px] py-3 rounded-xl transition-colors"
              >
                {t("orders.cashier_cancel_pay", "取消支付")}
              </button>
            </div>
          </>
        ) : phase === "paid" ? (
          renderResult(
            t("orders.cashier_paid_title", "支付成功"),
            t("orders.cashier_paid_desc", "支付已完成，订单将尽快为您处理。"),
            <CheckCircle2 className="w-16 h-16 text-green-500" />,
          )
        ) : phase === "expired" ? (
          renderResult(
            t("orders.cashier_expired_title", "支付超时"),
            t("orders.cashier_expired_desc", "支付已超时，订单已自动关闭。"),
            <Clock className="w-16 h-16 text-text-sub" />,
          )
        ) : phase === "cancelled" ? (
          renderResult(
            t("orders.cashier_cancelled_title", "订单已取消"),
            t("orders.cashier_cancelled_desc", "订单已取消，如有疑问请联系商家。"),
            <XCircle className="w-16 h-16 text-text-sub" />,
          )
        ) : phase === "not_payable" ? (
          renderResult(
            t("orders.cashier_not_payable_title", "订单不可支付"),
            t("orders.cashier_not_payable_desc", "该订单当前状态不支持支付。"),
            <AlertCircle className="w-16 h-16 text-text-sub" />,
          )
        ) : (
          renderResult(
            t("orders.cashier_failed_title", "支付创建失败"),
            errorMessageText ?? t("orders.cashier_failed_desc", "请重试或稍后再试。"),
            <XCircle className="w-16 h-16 text-[#FA5151]" />,
            <button
              onClick={retryPayment}
              className="w-full bg-primary-blue active:opacity-80 text-white font-medium text-[15px] py-3 rounded-xl transition-opacity"
            >
              {t("orders.cashier_retry", "重新支付")}
            </button>,
          )
        )}
      </div>
    </PageLayout>
  );
}
