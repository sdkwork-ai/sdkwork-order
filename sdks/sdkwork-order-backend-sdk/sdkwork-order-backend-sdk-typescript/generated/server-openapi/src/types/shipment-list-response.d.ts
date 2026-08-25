import type { PageInfo } from './page-info';
import type { ShipmentSummary } from './shipment-summary';
export interface ShipmentListResponse {
    code: 0;
    data: unknown & {
        items: ShipmentSummary[];
        pageInfo: PageInfo;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=shipment-list-response.d.ts.map