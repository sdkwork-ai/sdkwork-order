import type { CouponSubscriptionBenefit } from './coupon-subscription-benefit';
import type { CouponTokenBankBenefit } from './coupon-token-bank-benefit';
export interface CouponRedemptionResult {
    orderId: string;
    orderNo: string;
    status: string;
    replayed: boolean;
    benefit: CouponTokenBankBenefit | CouponSubscriptionBenefit;
}
//# sourceMappingURL=coupon-redemption-result.d.ts.map