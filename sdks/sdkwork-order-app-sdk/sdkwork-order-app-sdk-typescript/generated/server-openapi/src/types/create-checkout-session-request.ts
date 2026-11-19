import type { CheckoutLineRequest } from './checkout-line-request';
import type { ShippingAddressRequest } from './shipping-address-request';

/** Provide either items or cartItemIds, not both. */
export interface CreateCheckoutSessionRequest {
  items?: CheckoutLineRequest[];
  currencyCode?: string;
  /** 实物商品必填；数字商品（如兑换码、虚拟权益）可省略。 */
  shippingAddress?: ShippingAddressRequest;
  /** Cart lines to check out. When present, `items` must be empty: the server takes the SKU and quantity from the cart snapshot and removes exactly these lines once the session exists. */
  cartItemIds?: string[];
}
