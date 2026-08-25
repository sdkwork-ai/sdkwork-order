export interface AfterSalesReturnShipment {
    id: string;
    tenantId: string;
    organizationId?: string;
    afterSalesId: string;
    returnShipmentNo: string;
    shipmentDirection: string;
    carrierCode?: string;
    carrierName?: string;
    trackingNo?: string;
    packageSnapshot: Record<string, unknown>[];
    shipFromAddressSnapshot: Record<string, unknown>;
    shipToAddressSnapshot: Record<string, unknown>;
    status: string;
    shippedAt?: string;
    receivedAt?: string;
    requestNo: string;
    idempotencyKey: string;
    createdAt: string;
    updatedAt: string;
}
//# sourceMappingURL=after-sales-return-shipment.d.ts.map