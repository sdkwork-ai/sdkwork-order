import type { MembershipOrderCreateResult } from './membership-order-create-result';
export interface MembershipOrderCreateResponse {
    code: 0;
    data: unknown & {
        item: MembershipOrderCreateResult;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=membership-order-create-response.d.ts.map