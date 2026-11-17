import type { PageInfo } from './page-info';
import type { ShipmentTrackingEventSummary } from './shipment-tracking-event-summary';

export interface ShipmentTrackingEventListResponse {
  code: 0;
  data: unknown & { items: ShipmentTrackingEventSummary[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
