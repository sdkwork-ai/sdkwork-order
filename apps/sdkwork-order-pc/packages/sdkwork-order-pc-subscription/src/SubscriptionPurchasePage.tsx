import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Crown, Loader2, QrCode, RefreshCw } from "lucide-react";

import {
  createPcSubscriptionPurchaseService,
  type PcSubscriptionPackage,
  type PcSubscriptionPurchasePort,
} from "./subscriptionPurchaseService";

export interface SubscriptionPurchasePageProps {
  service?: PcSubscriptionPurchasePort;
}

/**
 * PC subscription purchase page owned by the order domain: lists the
 * membership packages (catalog from the membership domain), creates the
 * checkout order and shows the cashier QR code until payment settles.
 */
export function SubscriptionPurchasePage({ service: serviceProp }: SubscriptionPurchasePageProps) {
  const { t } = useTranslation();
  const service = serviceProp ?? createPcSubscriptionPurchaseService();
  const [packages, setPackages] = useState<PcSubscriptionPackage[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [checkout, setCheckout] = useState<{ orderId: string; qrCode?: string } | null>(null);
  const [creating, setCreating] = useState(false);

  const load = useCallback(() => {
    setIsLoading(true);
    setLoadFailed(false);
    service
      .listPackages()
      .then((items) => {
        setPackages(items);
        if (items.length > 0 && !selectedId) {
          setSelectedId(items.find((item) => item.recommended)?.id ?? items[0].id);
        }
      })
      .catch(() => setLoadFailed(true))
      .finally(() => setIsLoading(false));
  }, [selectedId, service]);

  useEffect(() => {
    load();
  }, [load]);

  const handleCheckout = async () => {
    if (!selectedId || creating) {
      return;
    }
    setCreating(true);
    try {
      const payment = await service.createCheckout(selectedId);
      if (payment.orderId) {
        setCheckout({ orderId: payment.orderId, qrCode: payment.qrCode ?? payment.cashierUrl });
      }
    } finally {
      setCreating(false);
    }
  };

  const selectedPackage = packages.find((pkg) => pkg.id === selectedId) ?? null;

  if (checkout) {
    return (
      <div className="mx-auto w-full max-w-3xl p-6" data-order-subscription-cashier>
        <div className="rounded-xl border border-border-color bg-chat-other-bg p-6 flex flex-col items-center">
          <div className="flex items-center gap-2 text-text-main font-semibold mb-4">
            <QrCode className="w-5 h-5" />
            {t("subscription.scan_to_pay", "请扫码完成支付")}
          </div>
          {checkout.qrCode ? (
            <img src={checkout.qrCode} alt="payment qr" className="w-[220px] h-[220px]" />
          ) : (
            <div className="flex items-center gap-2 text-text-sub">
              <Loader2 className="w-5 h-5 animate-spin" />
              {t("subscription.creating_payment", "正在生成支付二维码...")}
            </div>
          )}
          <p className="text-[13px] text-text-sub mt-4">
            {t("subscription.payment_polling", "支付完成后将自动开通会员")}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto w-full max-w-5xl p-6" data-order-subscription-surface>
      <div className="flex items-center gap-2 mb-6">
        <Crown className="w-7 h-7 text-primary-blue" />
        <h2 className="text-[22px] font-bold text-text-main">
          {t("subscription.vip_title", "会员订阅")}
        </h2>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-24 text-text-sub">
          <Loader2 className="w-6 h-6 animate-spin mr-2" />
          {t("subscription.loading_packages", "正在加载会员套餐...")}
        </div>
      ) : loadFailed ? (
        <button
          type="button"
          onClick={load}
          className="flex items-center gap-2 mx-auto py-24 text-primary-blue"
        >
          <RefreshCw className="w-5 h-5" />
          {t("subscription.load_failed", "套餐加载失败，点击重试")}
        </button>
      ) : packages.length === 0 ? (
        <div className="py-24 text-center text-text-sub">
          {t("subscription.empty_packages", "暂无可用会员套餐")}
        </div>
      ) : (
        <>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {packages.map((pkg) => {
              const active = pkg.id === selectedId;
              return (
                <button
                  key={pkg.id}
                  type="button"
                  onClick={() => setSelectedId(pkg.id)}
                  className={`rounded-xl border p-5 text-left transition-colors ${
                    active
                      ? "border-primary-blue bg-primary-blue/5 shadow-sm"
                      : "border-border-color bg-chat-other-bg hover:bg-hover-bg"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="text-[16px] font-semibold text-text-main">{pkg.name}</span>
                    {pkg.recommended && (
                      <span className="text-[11px] text-white bg-orange-500 rounded-full px-2 py-0.5">
                        {t("subscription.recommended", "推荐")}
                      </span>
                    )}
                  </div>
                  {pkg.planName && (
                    <div className="mt-1 text-[12px] text-text-sub">{pkg.planName}</div>
                  )}
                  <div className="mt-3 flex items-baseline gap-1">
                    <span className="text-[24px] font-bold text-primary-blue">¥{pkg.price}</span>
                    {pkg.originalPrice && (
                      <span className="text-[13px] text-text-sub line-through">¥{pkg.originalPrice}</span>
                    )}
                    <span className="text-[12px] text-text-sub ml-1">
                      {t("subscription.duration_days", "{{days}}天", { days: pkg.durationDays })}
                    </span>
                  </div>
                </button>
              );
            })}
          </div>

          <div className="mt-6 flex justify-end">
            <button
              type="button"
              disabled={!selectedPackage || creating}
              onClick={() => void handleCheckout()}
              className="px-8 py-2.5 bg-primary-blue active:opacity-80 disabled:opacity-40 text-white font-medium rounded-lg"
            >
              {creating
                ? t("subscription.creating_order", "正在创建订单...")
                : t("subscription.confirm_payment", "立即开通")}
            </button>
          </div>
        </>
      )}
    </div>
  );
}
