import type { CashierPayment } from './cashier-payment';

export interface CashierPaymentResponse {
  code: 0;
  data: unknown & { item: CashierPayment; };
  /** Server-owned request correlation id. */
  traceId: string;
}
