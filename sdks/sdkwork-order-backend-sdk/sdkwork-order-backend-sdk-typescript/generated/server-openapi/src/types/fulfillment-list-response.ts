import type { FulfillmentSummary } from './fulfillment-summary';
import type { PageInfo } from './page-info';

export interface FulfillmentListResponse {
  code: 0;
  data: unknown & { items: FulfillmentSummary[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
