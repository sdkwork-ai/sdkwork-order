export interface AdminCreateRefundRequest {
    /** Refund amount (partial or full). Must be positive and must not exceed the remaining refundable amount of the order. */
    amount: string;
    /** Currency of the refund amount; defaults to the order currency. */
    currencyCode?: string;
    reasonCode?: string;
    reasonMessage?: string;
}
//# sourceMappingURL=admin-create-refund-request.d.ts.map