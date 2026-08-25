export interface AfterSalesItem {
    id: string;
    tenantId: string;
    organizationId?: string;
    afterSalesId: string;
    orderItemId: string;
    skuId?: string;
    skuSnapshot: Record<string, unknown>;
    requestedQuantity: number;
    approvedQuantity: number;
    receivedQuantity: number;
    refundedQuantity: number;
    refundAmount: string;
    replacementSkuId?: string;
    itemStatus: string;
    createdAt: string;
    updatedAt: string;
}
//# sourceMappingURL=after-sales-item.d.ts.map