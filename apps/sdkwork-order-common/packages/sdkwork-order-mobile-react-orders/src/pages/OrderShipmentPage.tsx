import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router";
import { Package, Truck } from "lucide-react";

import { PageLayout } from "@sdkwork/ui-mobile-react";

import { getOrderLogisticsRuntime, type OrderLogisticsShipment } from "../services/OrderLogisticsRuntime";
import { formatOrderDateTime } from "../services/datetime";

/** Host-overridable order list path for the back affordance. */
export interface OrderShipmentPageProps {
  orderCenterPath?: string;
}

/**
 * Shipment tracking surface.
 *
 * Owns the tracking presentation (shipment header, packages, event timeline,
 * empty/error states). Data comes from the injected logistics runtime; this page
 * replaces per-host logistics copies and keeps the back action on the host's
 * own order route.
 */
export function OrderShipmentPage({ orderCenterPath = "/orders" }: OrderShipmentPageProps = {}) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const orderId = searchParams.get("orderId") ?? undefined;
  const shipmentId = searchParams.get("shipmentId") ?? undefined;
  const [shipments, setShipments] = useState<readonly OrderLogisticsShipment[]>([]);
  const [loading, setLoading] = useState(Boolean(orderId || shipmentId));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!orderId && !shipmentId) {
      setLoading(false);
      return;
    }
    const runtime = getOrderLogisticsRuntime();
    if (!runtime) {
      setLoading(false);
      setError(t("orders.shipment_unavailable", "物流能力尚未接入"));
      return;
    }
    let active = true;
    setLoading(true);
    setError(null);
    runtime
      .loadShipments({ orderId, shipmentId })
      .then((result) => {
        if (active) {
          setShipments(result.shipments);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(cause instanceof Error ? cause.message : t("orders.shipment_load_failed", "物流加载失败"));
        }
      })
      .finally(() => {
        if (active) {
          setLoading(false);
        }
      });
    return () => {
      active = false;
    };
  }, [orderId, shipmentId, t]);

  return (
    <PageLayout title={t("orders.shipment_title", "物流跟踪")}>
      <h1 className="sr-only">{t("orders.shipment_title", "物流跟踪")}</h1>
      <div className="flex flex-1 flex-col gap-3 overflow-y-auto px-3 py-3 pb-6">
        <button
          className="w-fit rounded-full border border-border-color px-4 py-1.5 text-[13px] text-text-main active:bg-active-bg"
          onClick={() => navigate(orderCenterPath)}
          type="button"
        >
          {t("orders.shipment_back", "返回订单列表")}
        </button>

        {!orderId && !shipmentId ? (
          <p className="py-8 text-center text-[13px] text-text-sub">
            {t("orders.shipment_missing", "缺少订单参数，请从订单列表进入")}
          </p>
        ) : null}

        {loading ? (
          <p className="py-8 text-center text-[13px] text-text-sub">
            {t("orders.loading", "加载中...")}
          </p>
        ) : null}

        {!loading && error ? (
          <div className="flex flex-col items-center gap-3 py-8 text-center">
            <p className="text-[13px] text-text-sub">{error}</p>
            <button
              className="rounded-full bg-primary-blue px-4 py-1.5 text-[13px] text-white active:opacity-80"
              onClick={() => {
                const next = new URLSearchParams();
                if (orderId) next.set("orderId", orderId);
                if (shipmentId) next.set("shipmentId", shipmentId);
                navigate(`?${next.toString()}`, { replace: true });
              }}
              type="button"
            >
              {t("orders.retry", "重试")}
            </button>
          </div>
        ) : null}

        {!loading && !error && (orderId || shipmentId) && shipments.length === 0 ? (
          <div className="flex flex-col items-center gap-2 py-10 text-center">
            <Truck className="h-10 w-10 text-text-sub/40" />
            <p className="text-[13px] text-text-sub">
              {t("orders.shipment_empty", "暂无物流信息，商家发货后可在此查看")}
            </p>
          </div>
        ) : null}

        {shipments.map((shipment) => (
          <section className="rounded-xl bg-chat-other-bg p-4" key={shipment.shipmentId}>
            <h2 className="text-[15px] font-semibold text-text-main">
              {shipment.shipmentNo
                ? t("orders.shipment_no", "运单号：{{no}}", { no: shipment.shipmentNo })
                : shipment.shipmentId}
            </h2>
            {shipment.carrier || shipment.statusLabel ? (
              <p className="mt-1 text-[12px] text-text-sub">
                {[shipment.carrier, shipment.statusLabel].filter(Boolean).join(" · ")}
              </p>
            ) : null}
            {shipment.packages.length > 0 ? (
              <p className="mt-1 text-[12px] text-text-sub">
                {t("orders.shipment_packages", "包裹：{{names}}", {
                  names: shipment.packages.map((pkg) => pkg.name || pkg.id).join("、"),
                })}
              </p>
            ) : null}

            {shipment.trackingEvents.length === 0 ? (
              <p className="mt-3 text-[12px] text-text-sub">
                {t("orders.shipment_events_empty", "暂无轨迹")}
              </p>
            ) : (
              <ol className="mt-3 flex flex-col gap-3">
                {shipment.trackingEvents.map((event, index) => (
                  <li className="flex gap-2.5" key={`${shipment.shipmentId}-${index}`}>
                    <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-primary-blue" />
                    <div className="min-w-0 flex-1">
                      <p className="text-[13px] leading-snug text-text-main">{event.description}</p>
                      {event.occurredAt || event.status ? (
                        <p className="mt-0.5 text-[11px] text-text-sub">
                          {[formatOrderDateTime(event.occurredAt, i18n.resolvedLanguage || "zh-CN"), event.status]
                            .filter(Boolean)
                            .join(" · ")}
                        </p>
                      ) : null}
                    </div>
                  </li>
                ))}
              </ol>
            )}
          </section>
        ))}

        {shipments.some((shipment) => shipment.packages.length > 0) ? (
          <p className="flex items-center gap-1.5 px-1 text-[11px] text-text-sub">
            <Package className="h-3.5 w-3.5" />
            {t("orders.shipment_hint", "物流轨迹由承运商回传，可能存在延迟")}
          </p>
        ) : null}
      </div>
    </PageLayout>
  );
}
