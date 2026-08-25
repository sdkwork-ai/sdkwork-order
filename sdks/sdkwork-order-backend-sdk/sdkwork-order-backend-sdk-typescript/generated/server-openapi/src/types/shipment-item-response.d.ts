import type { ShipmentSummary } from './shipment-summary';
export interface ShipmentItemResponse {
    code: 0;
    data: unknown & {
        item: ShipmentSummary;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=shipment-item-response.d.ts.map