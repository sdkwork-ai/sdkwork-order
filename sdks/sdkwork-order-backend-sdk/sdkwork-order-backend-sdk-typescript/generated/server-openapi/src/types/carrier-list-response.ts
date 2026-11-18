import type { CarrierSummary } from './carrier-summary';
import type { PageInfo } from './page-info';

export interface CarrierListResponse {
  code: 0;
  data: unknown & { items: CarrierSummary[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
