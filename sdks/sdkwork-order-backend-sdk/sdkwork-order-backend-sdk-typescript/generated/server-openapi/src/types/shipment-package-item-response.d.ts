import type { ShipmentPackageSummary } from './shipment-package-summary';
export interface ShipmentPackageItemResponse {
    code: 0;
    data: unknown & {
        item: ShipmentPackageSummary;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=shipment-package-item-response.d.ts.map