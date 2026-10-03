import type { AppTemplateOrderSummary } from './app-template-order-summary';
import type { PageInfo } from './page-info';

export interface AppTemplateOrderListResponse {
  code: 0;
  data: unknown & { items: AppTemplateOrderSummary[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
