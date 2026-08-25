import type { AfterSalesRequestResponse } from './after-sales-request-response';
export interface AfterSalesRequestsRetrieveResponse {
    code: 0;
    data: unknown & {
        item: AfterSalesRequestResponse;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=after-sales-requests-retrieve-response.d.ts.map