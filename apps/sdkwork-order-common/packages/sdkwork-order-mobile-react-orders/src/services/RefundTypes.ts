/**
 * Refund-request and logistics read-model types for the mobile order UI.
 *
 * Wire contract: `apis/app-api/order/order-app-api.openapi.json` —
 * `GET/POST /orders/refund_requests` (backend `AccountValueRequestResponse`
 * rows) and the buyer logistics chain `GET /fulfillments?order_id=` →
 * `GET /shipments/{shipmentId}` → `GET /shipments/{shipmentId}/tracking_events`.
 */

/** Refund target assets accepted by `RefundRequestCreateCommand.targetAsset`. */
export type RefundTargetAsset = "points" | "token_bank" | "cash";

/** Canonical asset list rendered by the refund form (order matters for UX). */
export const REFUND_TARGET_ASSETS: readonly RefundTargetAsset[] = [
  "points",
  "token_bank",
  "cash",
];

/** Narrows a wire/UI value onto the `RefundTargetAsset` union. */
export function isRefundTargetAsset(value: string): value is RefundTargetAsset {
  return (REFUND_TARGET_ASSETS as readonly string[]).includes(value);
}

/** Readable asset label fallbacks when a host ships no `orders.refund_asset_*` resource. */
export const REFUND_TARGET_ASSET_LABELS: Readonly<Record<RefundTargetAsset, string>> = {
  points: "积分",
  token_bank: "Token Bank",
  cash: "现金",
};

/**
 * Refund request read model aligned with the backend
 * `AccountValueRequestResponse` (camelCase). `amount` is a minor-unit integer
 * string for `cash` refunds and an integer asset-unit string for
 * `points`/`token_bank` refunds.
 */
export interface RefundRequestView {
  readonly requestId: string;
  readonly requestNo: string;
  readonly originalOrderId?: string;
  readonly subject: string;
  readonly targetAsset: string;
  readonly amount: string;
  readonly currencyCode: string;
  readonly status: string;
  readonly createdAt: string;
  readonly updatedAt: string;
}

/** User-facing create input. `amount` is user-entered: cash is a major-unit decimal (元). */
export interface CreateRefundRequestInput {
  readonly orderId: string;
  readonly targetAsset: RefundTargetAsset;
  readonly amount: string;
  readonly currencyCode?: string;
  readonly reasonCode?: string;
  readonly reasonDetail?: string;
}

/** One logistics tracking event (backend `ShipmentTrackingEventResponse`). */
export interface ShipmentTrackingEvent {
  readonly eventId: string;
  readonly eventType: string;
  readonly eventStatus?: string;
  readonly eventTime: string;
  readonly locationText?: string;
}

/**
 * Buyer logistics summary assembled by `OrderService.getOrderShipment`:
 * the order's first fulfillment plus its shipment header and one bounded
 * tracking-events page. Shipment fields stay unset when the fulfillment has
 * no shipment row yet, so the UI can still render the fulfillment header.
 */
export interface OrderShipmentSummary {
  readonly fulfillmentId: string;
  readonly fulfillmentNo: string;
  readonly fulfillmentStatus: string;
  readonly shipmentId?: string;
  readonly shipmentNo: string;
  readonly carrierCode: string;
  readonly trackingNo?: string;
  readonly status: string;
  readonly events: readonly ShipmentTrackingEvent[];
}

/** Cash amounts are major-unit decimals with at most two fraction digits (¥10.50). */
const CASH_MAJOR_AMOUNT_PATTERN = /^(\d+)(?:\.(\d{1,2}))?$/;

/**
 * Normalizes the user-entered refund amount onto the wire `amount` string.
 * - `cash` refunds are money: the major-unit decimal (元) converts to the
 *   minor-unit integer string (¥10.50 → "1050"), the same rule as the Token
 *   Bank catalog price conversion. Decimal string splicing avoids binary
 *   float rounding (`10.10 * 100` is not always `1010`).
 * - `points`/`token_bank` refunds are asset units: positive integer strings
 *   pass through verbatim; anything else (decimals, signs, "0") is invalid.
 * Returns `null` when the input is not a valid amount for the asset.
 */
export function normalizeRefundAmountWire(
  targetAsset: RefundTargetAsset,
  amount: string,
): string | null {
  const trimmed = amount.trim();
  if (targetAsset === "cash") {
    const match = CASH_MAJOR_AMOUNT_PATTERN.exec(trimmed);
    if (!match) {
      return null;
    }
    const cents = (match[2] ?? "").padEnd(2, "0");
    return `${match[1]}${cents}`.replace(/^0+(?=\d)/, "");
  }
  return /^(\d+)$/.test(trimmed) && trimmed !== "0" ? trimmed : null;
}
