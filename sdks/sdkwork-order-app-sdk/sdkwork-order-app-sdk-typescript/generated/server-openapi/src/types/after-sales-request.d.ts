import type { AfterSalesItem } from './after-sales-item';
export interface AfterSalesRequest {
    id: string;
    tenantId: string;
    organizationId?: string;
    afterSalesNo: string;
    orderId: string;
    ownerUserId?: string;
    shopId?: string;
    refundId?: string;
    replacementOrderId?: string;
    afterSalesType: string;
    status: string;
    refundStatus: string;
    returnStatus: string;
    exchangeStatus: string;
    reasonCode?: string;
    description?: string;
    evidenceSnapshot: Record<string, unknown>[];
    requestedAmount: string;
    approvedAmount: string;
    currencyCode: string;
    requestedByType: string;
    requestedBy?: string;
    reviewerType?: string;
    reviewerId?: string;
    reviewedAt?: string;
    closedAt?: string;
    requestNo: string;
    idempotencyKey: string;
    items: AfterSalesItem[];
    createdAt: string;
    updatedAt: string;
}
//# sourceMappingURL=after-sales-request.d.ts.map