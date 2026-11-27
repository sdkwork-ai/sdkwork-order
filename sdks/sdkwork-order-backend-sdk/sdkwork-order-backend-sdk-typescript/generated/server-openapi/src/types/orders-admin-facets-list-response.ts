import type { OrderFacet } from './order-facet';

export interface OrdersAdminFacetsListResponse {
  code: 0;
  data: unknown & { item: OrderFacet; };
  /** Server-owned request correlation id. */
  traceId: string;
}
