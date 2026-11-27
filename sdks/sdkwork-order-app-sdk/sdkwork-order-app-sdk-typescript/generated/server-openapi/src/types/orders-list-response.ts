import type { OrderSummary } from './order-summary';
import type { PageInfo } from './page-info';

export interface OrdersListResponse {
  code: 0;
  data: unknown & { items: OrderSummary[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
