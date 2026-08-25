import type { CheckoutQuote } from './checkout-quote';
export interface CreateCheckoutQuoteResponse {
    code: 0;
    data: unknown & {
        item: CheckoutQuote;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=create-checkout-quote-response.d.ts.map