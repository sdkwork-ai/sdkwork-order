import type { CheckoutLineRequest } from './checkout-line-request';
import type { ShippingAddressRequest } from './shipping-address-request';
export interface CreateCheckoutSessionRequest {
    items: CheckoutLineRequest[];
    currencyCode?: string;
    /** 实物商品必填；数字商品（如兑换码、虚拟权益）可省略。 */
    shippingAddress?: ShippingAddressRequest;
}
//# sourceMappingURL=create-checkout-session-request.d.ts.map