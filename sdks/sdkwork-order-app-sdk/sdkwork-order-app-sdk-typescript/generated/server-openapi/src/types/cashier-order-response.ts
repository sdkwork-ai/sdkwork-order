import type { CashierOrder } from './cashier-order';

export interface CashierOrderResponse {
  code: 0;
  data: unknown & { item: CashierOrder; };
  /** Server-owned request correlation id. */
  traceId: string;
}
