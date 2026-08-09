import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Crown, Shield, Zap } from "lucide-react";
import { PageLayout } from "@sdkwork/ui-mobile-react";

import { PaymentPanel } from "../components/PaymentPanel";
import {
  createSubscriptionPurchaseService,
  type SubscriptionPurchasePort,
} from "../services/SubscriptionPurchaseService";
import type { MembershipPackage } from "../services/SubscriptionCatalogPort";

export interface VipSubscriptionPageProps {
  service?: SubscriptionPurchasePort;
}

const FALLBACK_BENEFITS = [
  { icon: Crown, title: "专属标识", desc: "尊贵身份的外显标识" },
  { icon: Shield, title: "安全防护", desc: "高级别的账号找回与安全" },
  { icon: Zap, title: "优先体验", desc: "最新功能提前体验" },
];

export function VipSubscriptionPage({ service: serviceProp }: VipSubscriptionPageProps) {
  const { t } = useTranslation();
  const service = serviceProp ?? createSubscriptionPurchaseService();
  const [packages, setPackages] = useState<MembershipPackage[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [payment, setPayment] = useState<{ orderId: string; qrCode: string; expiresAt?: string } | null>(null);
  const [creating, setCreating] = useState(false);

  const load = useCallback(() => {
    setIsLoading(true);
    setLoadFailed(false);
    service
      .listMembershipPackages()
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

  const handlePay = async () => {
    if (!selectedId || creating) {
      return;
    }
    setCreating(true);
    try {
      const result = await service.createSubscriptionOrder(selectedId);
      if (result.orderId && (result.qrCode ?? result.cashierUrl)) {
        setPayment({
          orderId: result.orderId,
          qrCode: result.qrCode ?? result.cashierUrl ?? "",
          expiresAt: result.expiresAt,
        });
      }
    } finally {
      setCreating(false);
    }
  };

  const getStatus = useCallback(
    (orderId: string) => service.getSubscriptionStatus(orderId),
    [service],
  );

  const selectedPackage = packages.find((pkg) => pkg.id === selectedId) ?? null;

  return (
    <PageLayout title={t("subscription.vip_title", "会员订阅")} bgClass="bg-[#F8F9FA] dark:bg-black">
      {payment ? (
        <PaymentPanel
          expiresAt={payment.expiresAt}
          getStatus={getStatus}
          orderId={payment.orderId}
          qrPayload={payment.qrCode}
        />
      ) : (
        <div className="flex-1 overflow-y-auto p-4">
          {isLoading ? (
            <div className="flex items-center justify-center py-20 text-[14px] text-text-sub">
              {t("subscription.loading_packages", "正在加载会员套餐...")}
            </div>
          ) : loadFailed ? (
            <button type="button" className="flex w-full items-center justify-center py-20 text-[14px] text-primary-blue" onClick={load}>
              {t("subscription.load_failed", "套餐加载失败，点击重试")}
            </button>
          ) : packages.length === 0 ? (
            <div className="flex items-center justify-center py-20 text-[14px] text-text-sub">
              {t("subscription.empty_packages", "暂无可用会员套餐")}
            </div>
          ) : (
            <>
              {/* Membership banner */}
              <div className="bg-gradient-to-br from-indigo-600 to-purple-700 dark:from-indigo-900/80 dark:to-purple-950 rounded-2xl p-6 text-white mb-4">
                <div className="flex items-center gap-2 text-white/90 mb-1">
                  <Crown className="w-6 h-6" />
                  <span className="text-[17px] font-bold">{t("subscription.vip_membership", "SDKWork 会员")}</span>
                </div>
                <p className="text-[13px] text-white/70">
                  {t("subscription.vip_desc", "开通会员，解锁专属权益与能力")}
                </p>
              </div>

              {/* Packages */}
              <div className="space-y-3">
                {packages.map((pkg) => {
                  const active = pkg.id === selectedId;
                  return (
                    <button
                      key={pkg.id}
                      type="button"
                      onClick={() => setSelectedId(pkg.id)}
                      className={`w-full rounded-xl p-4 text-left border transition-colors ${
                        active ? "border-primary-blue bg-primary-blue/5" : "border-border-color bg-chat-other-bg"
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <div className="text-[15px] font-bold text-text-main">{pkg.name}</div>
                        {pkg.recommended && (
                          <span className="text-[11px] text-white bg-orange-500 rounded-full px-2 py-0.5">
                            {t("subscription.recommended", "推荐")}
                          </span>
                        )}
                      </div>
                      {pkg.planName && (
                        <div className="mt-0.5 text-[12px] text-text-sub">{pkg.planName}</div>
                      )}
                      <div className="mt-2 flex items-baseline gap-1">
                        <span className="text-[20px] font-bold text-primary-blue">¥{pkg.price}</span>
                        {pkg.originalPrice && (
                          <span className="text-[13px] text-text-sub line-through">¥{pkg.originalPrice}</span>
                        )}
                        <span className="text-[12px] text-text-sub ml-1">
                          {t("subscription.duration_days", "{{days}}天", { days: pkg.durationDays })}
                        </span>
                      </div>
                      {Array.isArray(pkg.tags) && pkg.tags.length > 0 && (
                        <div className="mt-2 flex flex-wrap gap-1">
                          {pkg.tags.map((tag) => (
                            <span key={tag} className="text-[11px] text-text-sub bg-active-bg rounded px-1.5 py-0.5">
                              {tag}
                            </span>
                          ))}
                        </div>
                      )}
                    </button>
                  );
                })}
              </div>

              {/* Benefits */}
              <div className="mt-5 bg-chat-other-bg rounded-xl p-4 border border-border-color">
                <h3 className="text-[14px] font-bold text-text-main mb-3">
                  {t("subscription.vip_benefits", "会员权益")}
                </h3>
                <div className="space-y-3">
                  {FALLBACK_BENEFITS.map((benefit) => {
                    const Icon = benefit.icon;
                    return (
                      <div key={benefit.title} className="flex items-center gap-3">
                        <Icon className="w-5 h-5 text-primary-blue" />
                        <div>
                          <div className="text-[14px] text-text-main">{benefit.title}</div>
                          <div className="text-[12px] text-text-sub">{benefit.desc}</div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>

              <button
                type="button"
                disabled={!selectedPackage || creating}
                onClick={() => void handlePay()}
                className="w-full mt-4 bg-primary-blue active:opacity-80 disabled:opacity-40 text-white font-medium text-[15px] py-3 rounded-xl"
              >
                {creating
                  ? t("subscription.creating_order", "正在创建订单...")
                  : t("subscription.confirm_payment", "立即开通")}
              </button>
            </>
          )}
        </div>
      )}
    </PageLayout>
  );
}
