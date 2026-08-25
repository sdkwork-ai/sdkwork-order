export interface UpdateAfterSalesRequest {
    /** Target status for the after-sales request lifecycle transition. */
    status?: string;
    reasonCode?: string;
    description?: string;
    evidenceSnapshot?: Record<string, unknown>[];
    requestedAmount?: string;
    approvedAmount?: string;
    currencyCode?: string;
    reviewerNote?: string;
}
//# sourceMappingURL=update-after-sales-request.d.ts.map