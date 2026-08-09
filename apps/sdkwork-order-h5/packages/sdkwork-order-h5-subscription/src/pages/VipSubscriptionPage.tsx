import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { Check, Crown, Shield, Zap } from "lucide-react";
import { PageLayout, cn } from "@sdkwork/ui-mobile-react";

import { VipPlanTabs } from "../components/VipPlanTabs";
import { VipPurchaseFooterBar } from "../components/VipPurchaseFooterBar";
import {
  createSubscriptionPurchaseService,
  type SubscriptionPurchasePort,
} from "../services/SubscriptionPurchaseService";
import type { MembershipPackage, MembershipPackageGroup } from "../services/SubscriptionCatalogPort";

export interface VipSubscriptionPageProps {
  service?: SubscriptionPurchasePort;
  /** Host cashier route path with `:orderId` placeholder. */
  cashierPath?: string;
}

const FALLBACK_BENEFITS = [
  { icon: Crown, title: "专属标识", desc: "尊贵身份的外显标识" },
  { icon: Shield, title: "安全防护", desc: "高级别的账号找回与安全" },
  { icon: Zap, title: "优先体验", desc: "最新功能提前体验" },
];

const DEFAULT_CASHIER_PATH = "/me/orders/:orderId/cashier";

export function VipSubscriptionPage({
  service: serviceProp,
  cashierPath = DEFAULT_CASHIER_PATH,
}: VipSubscriptionPageProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const service = serviceProp ?? createSubscriptionPurchaseService();
  const [groups, setGroups] = useState<MembershipPackageGroup[]>([]);
  const [activeGroupId, setActiveGroupId] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [creating, setCreating] = useState(false);

  const load = useCallback(() => {
    setIsLoading(true);
    setLoadFailed(false);
    service
      .listMembershipPackageGroups()
      .then(async (groupItems) => {
        // 后端未配置分组时，回退为平铺套餐的"全部套餐"单分组
        let nextGroups = groupItems.filter((group) => group.packages.length > 0);
        if (nextGroups.length === 0) {
          const flat = await service.listMembershipPackages();
          if (flat.length > 0) {
            nextGroups = [
              {
                id: "all",
                name: t("subscription.all_plans", "全部套餐"),
                packages: flat,
              },
            ];
          }
        }
        setGroups(nextGroups);
        setActiveGroupId((current) =>
          current && nextGroups.some((group) => group.id === current)
            ? current
            : (nextGroups[0]?.id ?? null),
        );
      })
      .catch(() => setLoadFailed(true))
      .finally(() => setIsLoading(false));
  }, [service, t]);

  useEffect(() => {
    load();
  }, [load]);

  const activeGroup =
    groups.find((group) => group.id === activeGroupId) ?? groups[0] ?? null;
  const activePackages = activeGroup?.packages ?? [];
  const selectedPackage =
    activePackages.find((pkg) => pkg.id === selectedId) ?? null;

  // 切换分组后，确保选中项属于当前分组（优先推荐套餐）
  useEffect(() => {
    if (activePackages.length === 0 || selectedPackage) {
      return;
    }
    setSelectedId(
      activePackages.find((pkg) => pkg.recommended)?.id ?? activePackages[0].id,
    );
  }, [activeGroupId, activePackages, selectedPackage]);

  /**
   * 下单后跳转到宿主收银台路由（移动端在收银台内完成环境适配的支付，
   * 不再在本页展示二维码）。
   */
  const handlePay = async () => {
    if (!selectedPackage || creating) {
      return;
    }
    setCreating(true);
    try {
      const result = await service.createSubscriptionOrder(selectedPackage.id);
      if (result.orderId) {
        navigate(cashierPath.replace(":orderId", encodeURIComponent(result.orderId)));
      }
    } finally {
      setCreating(false);
    }
  };

  return (
    <PageLayout title={t("subscription.vip_title", "会员订阅")} bgClass="bg-[#F8F9FA] dark:bg-black">
      <VipPlanTabs
        groups={groups}
        activeGroupId={activeGroupId}
        onSelectGroup={setActiveGroupId}
      />

      <div className="flex-1 overflow-y-auto p-4 pb-28">
        {isLoading ? (
          <div className="flex items-center justify-center py-20 text-[14px] text-text-sub">
            {t("subscription.loading_packages", "正在加载会员套餐...")}
          </div>
        ) : loadFailed ? (
          <button type="button" className="flex w-full items-center justify-center py-20 text-[14px] text-primary-blue" onClick={load}>
            {t("subscription.load_failed", "套餐加载失败，点击重试")}
          </button>
        ) : activePackages.length === 0 ? (
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

            {/* Active group packages */}
            <div className="space-y-3">
              {activePackages.map((pkg) => {
                const active = pkg.id === selectedPackage?.id;
                return (
                  <button
                    key={pkg.id}
                    type="button"
                    onClick={() => setSelectedId(pkg.id)}
                    className={cn(
                      "w-full rounded-xl p-4 text-left border transition-colors",
                      active
                        ? "border-primary-blue bg-primary-blue/5"
                        : "border-border-color bg-chat-other-bg",
                    )}
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
                      <span className="ml-auto">
                        <span
                          className={cn(
                            "flex h-5 w-5 items-center justify-center rounded-full border",
                            active
                              ? "border-primary-blue bg-primary-blue text-white"
                              : "border-border-color bg-white dark:bg-[#1A1A1A]",
                          )}
                        >
                          {active && <Check className="w-3 h-3" strokeWidth={3} />}
                        </span>
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
          </>
        )}
      </div>

      {selectedPackage && !isLoading && !loadFailed && (
        <VipPurchaseFooterBar
          packageItem={selectedPackage}
          creating={creating}
          onPurchase={() => void handlePay()}
        />
      )}
    </PageLayout>
  );
}
