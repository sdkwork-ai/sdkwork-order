import type { PageInfo } from './page-info';
import type { ShipmentPackageSummary } from './shipment-package-summary';
export interface ShipmentPackageListResponse {
    code: 0;
    data: unknown & {
        items: ShipmentPackageSummary[];
        pageInfo: PageInfo;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=shipment-package-list-response.d.ts.map