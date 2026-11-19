import type { CommerceCartLine } from './commerce-cart-line';

export interface CommerceCart {
  id: string;
  status: string;
  currencyCode: string;
  items: CommerceCartLine[];
  totalQuantity: number;
  /** Cart subtotal in minor units (decimal string). */
  totalAmount: string;
}
