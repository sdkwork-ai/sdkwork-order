export interface CreateAfterSalesReturnShipmentRequest {
    shipmentDirection?: string;
    carrierCode: string;
    carrierName?: string;
    trackingNo: string;
    packageSnapshot?: Record<string, unknown>[];
    shipFromAddressSnapshot?: Record<string, unknown>;
    shipToAddressSnapshot?: Record<string, unknown>;
    shippedAt?: string;
}
//# sourceMappingURL=create-after-sales-return-shipment-request.d.ts.map