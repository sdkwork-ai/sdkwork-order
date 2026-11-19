import type { CommerceCartLine } from './commerce-cart-line';

export interface CreateCartItemResponse {
  code: 0;
  data: unknown & { item: CommerceCartLine; };
  /** Server-owned request correlation id. */
  traceId: string;
}
