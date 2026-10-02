import React from "react";
import { useTranslation } from "react-i18next";
import { PackageSearch, Truck } from "lucide-react";

import type { OrderShipmentSummary } from "../services/RefundTypes";

/** Readable carrier fallbacks for the common codes (hosts may ship resources). */
const CARRIER_FALLBACKS: Readonly<Record<string, string>> = {
  sf: "顺丰速运",
  yt: "圆通速递",
  zto: "中通快递",
  yunda: "韵达快递",
  ems: "邮政 EMS",
  jd: "京东物流",
};

/** Readable shipment status fallbacks (unknown statuses render verbatim). */
const SHIPMENT_STATUS_FALLBACKS: Readonly<Record<string, string>> = {
  created: "已发货",
  in_transit: "运输中",
  delivered: "已签收",
};

/**
 * Buyer logistics tracking card: shipment header (carrier / tracking no /
 * status) plus the tracking-event timeline. Rendered under the items card on
 * the order detail page when `OrderService.getOrderShipment` resolves a
 * fulfillment.
 */
export const OrderShipmentCard: React.FC<{ shipment: OrderShipmentSummary }> = ({ shipment }) => {
  const { t } = useTranslation();
  const carrierLabel = shipment.carrierCode
    ? t(
        `orders.shipment_carrier_${shipment.carrierCode}`,
        CARRIER_FALLBACKS[shipment.carrierCode] ?? shipment.carrierCode,
      )
    : t("orders.shipment_carrier_unknown", "承运方待更新");
  const statusLabel = t(
    `orders.shipment_status_${shipment.status}`,
    SHIPMENT_STATUS_FALLBACKS[shipment.status] ?? shipment.status,
  );

  return (
    <div className="bg-chat-other-bg rounded-xl p-4 shadow-sm flex flex-col gap-3">
      <h3 className="text-[14px] font-bold text-text-main flex items-center gap-2">
        <Truck className="h-4 w-4 text-primary-blue" aria-hidden="true" />
        {t("orders.shipment_title", "物流追踪")}
      </h3>
      <div className="flex items-center justify-between">
        <span className="text-[13px] text-text-sub">
          {t("orders.shipment_carrier", "承运方")}
        </span>
        <span className="text-[13px] text-text-main">{carrierLabel}</span>
      </div>
      {shipment.trackingNo && (
        <div className="flex items-center justify-between">
          <span className="text-[13px] text-text-sub">
            {t("orders.shipment_tracking_no", "运单号")}
          </span>
          <span className="text-[13px] text-text-main">{shipment.trackingNo}</span>
        </div>
      )}
      <div className="flex items-center justify-between">
        <span className="text-[13px] text-text-sub">
          {t("orders.shipment_status", "物流状态")}
        </span>
        <span className="text-[13px] text-text-main">{statusLabel}</span>
      </div>

      {shipment.events.length > 0 ? (
        <ol className="mt-1 flex flex-col gap-0">
          {shipment.events.map((event, index) => (
            <li
              className="relative flex gap-3 pb-4 last:pb-0"
              key={event.eventId || `${event.eventTime}-${index}`}
            >
              <span
                aria-hidden="true"
                className="mt-1.5 flex flex-col items-center"
              >
                <span className="h-2 w-2 rounded-full bg-primary-blue" />
                {index < shipment.events.length - 1 && (
                  <span className="mt-0.5 w-px flex-1 bg-border-color" />
                )}
              </span>
              <div className="flex flex-col gap-0.5">
                <span className="text-[13px] text-text-main">
                  {t(
                    `orders.shipment_event_${event.eventType}`,
                    event.eventStatus ?? event.eventType,
                  )}
                </span>
                {event.locationText && (
                  <span className="text-[12px] text-text-sub">{event.locationText}</span>
                )}
                <span className="text-[12px] text-text-sub">{event.eventTime}</span>
              </div>
            </li>
          ))}
        </ol>
      ) : (
        <div className="flex items-center gap-2 text-[12px] text-text-sub">
          <PackageSearch className="h-4 w-4" aria-hidden="true" />
          {t("orders.shipment_events_pending", "暂无物流轨迹更新")}
        </div>
      )}
    </div>
  );
};
