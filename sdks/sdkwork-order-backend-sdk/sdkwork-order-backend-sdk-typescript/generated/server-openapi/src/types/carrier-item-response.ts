import type { CarrierSummary } from './carrier-summary';

export interface CarrierItemResponse {
  code: 0;
  data: unknown & { item: CarrierSummary; };
  /** Server-owned request correlation id. */
  traceId: string;
}
