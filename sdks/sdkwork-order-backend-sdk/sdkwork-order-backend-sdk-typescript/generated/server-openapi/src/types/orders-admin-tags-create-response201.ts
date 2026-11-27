import type { OrderClassification } from './order-classification';

export interface OrdersAdminTagsCreateResponse201 {
  code: 0;
  data: unknown & { item: OrderClassification; };
  /** Server-owned request correlation id. */
  traceId: string;
}
