export interface SdkWorkItemResponse {
    code: 0;
    data: unknown & {
        item: Record<string, unknown>;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=sdk-work-item-response.d.ts.map