import type { CheckoutSession } from './checkout-session';
export interface CheckoutSessionResponse {
    code: 0;
    data: unknown & {
        item: CheckoutSession;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=checkout-session-response.d.ts.map