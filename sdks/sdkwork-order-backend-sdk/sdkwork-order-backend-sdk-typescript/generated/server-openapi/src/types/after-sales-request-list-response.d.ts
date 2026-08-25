import type { AfterSalesRequestSummary } from './after-sales-request-summary';
import type { PageInfo } from './page-info';
export interface AfterSalesRequestListResponse {
    code: 0;
    data: unknown & {
        items: AfterSalesRequestSummary[];
        pageInfo: PageInfo;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=after-sales-request-list-response.d.ts.map