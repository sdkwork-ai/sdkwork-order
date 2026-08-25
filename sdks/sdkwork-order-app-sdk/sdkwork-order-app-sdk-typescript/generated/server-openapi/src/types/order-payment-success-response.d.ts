import type { OrderPaymentSuccessData } from './order-payment-success-data';
export interface OrderPaymentSuccessResponse {
    code: 0;
    data: unknown & OrderPaymentSuccessData;
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=order-payment-success-response.d.ts.map