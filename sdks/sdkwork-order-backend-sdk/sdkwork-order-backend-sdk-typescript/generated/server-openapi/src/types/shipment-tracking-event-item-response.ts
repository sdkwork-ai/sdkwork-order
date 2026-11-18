import type { ShipmentTrackingEventSummary } from './shipment-tracking-event-summary';

export interface ShipmentTrackingEventItemResponse {
  code: 0;
  data: unknown & { item: ShipmentTrackingEventSummary; };
  /** Server-owned request correlation id. */
  traceId: string;
}
