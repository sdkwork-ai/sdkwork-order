import type { CouponRedemptionResult } from './coupon-redemption-result';
export interface CouponRedemptionResponse {
    code: 0;
    data: unknown & {
        item: CouponRedemptionResult;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=coupon-redemption-response.d.ts.map