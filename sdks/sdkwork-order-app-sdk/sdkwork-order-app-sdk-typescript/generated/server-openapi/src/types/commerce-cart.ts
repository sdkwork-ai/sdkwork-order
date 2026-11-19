import type { CommerceCartLine } from './commerce-cart-line';

export interface CommerceCart {
  id: string;
  status: 'active' | 'converted' | 'abandoned' | 'expired';
  currencyCode: string;
  items: CommerceCartLine[];
  itemCount: number;
  totalQuantity: number;
  selectedQuantity: number;
  /** Subtotal of the selected lines in minor units (decimal string). */
  totalAmount: string;
  /** True when a selected line can no longer be sold. */
  hasUnavailableSelection: boolean;
}
