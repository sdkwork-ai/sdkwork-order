import type { CommerceCart } from './commerce-cart';

export interface CartCurrentResponse {
  code: 0;
  data: unknown & { item: CommerceCart; };
  /** Server-owned request correlation id. */
  traceId: string;
}
