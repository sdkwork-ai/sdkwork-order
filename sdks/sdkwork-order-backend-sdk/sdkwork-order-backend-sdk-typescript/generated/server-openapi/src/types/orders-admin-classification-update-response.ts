import type { OrderClassification } from './order-classification';

export interface OrdersAdminClassificationUpdateResponse {
  code: 0;
  data: unknown & { item: OrderClassification; };
  /** Server-owned request correlation id. */
  traceId: string;
}
