import type { CreateAfterSalesRequestItem } from './create-after-sales-request-item';
export interface CreateAfterSalesRequest {
    orderId: string;
    afterSalesType: string;
    reasonCode: string;
    description?: string;
    evidenceSnapshot?: Record<string, unknown>[];
    requestedAmount: string;
    currencyCode: string;
    items: CreateAfterSalesRequestItem[];
}
//# sourceMappingURL=create-after-sales-request.d.ts.map