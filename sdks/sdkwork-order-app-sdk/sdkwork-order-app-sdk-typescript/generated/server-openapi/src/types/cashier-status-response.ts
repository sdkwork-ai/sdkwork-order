import type { CashierStatus } from './cashier-status';

export interface CashierStatusResponse {
  code: 0;
  data: unknown & { item: CashierStatus; };
  /** Server-owned request correlation id. */
  traceId: string;
}
