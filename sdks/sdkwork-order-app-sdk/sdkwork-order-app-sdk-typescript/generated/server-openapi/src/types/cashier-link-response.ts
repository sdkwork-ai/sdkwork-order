import type { CashierLink } from './cashier-link';

export interface CashierLinkResponse {
  code: 0;
  data: unknown & { item: CashierLink; };
  /** Server-owned request correlation id. */
  traceId: string;
}
