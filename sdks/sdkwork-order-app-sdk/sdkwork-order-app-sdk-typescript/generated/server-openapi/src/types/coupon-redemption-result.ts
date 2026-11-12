import type { CouponCashBenefit } from './coupon-cash-benefit';
import type { CouponPointsBenefit } from './coupon-points-benefit';
import type { CouponSubscriptionBenefit } from './coupon-subscription-benefit';
import type { CouponTokenBankBenefit } from './coupon-token-bank-benefit';

export interface CouponRedemptionResult {
  orderId: string;
  orderNo: string;
  status: string;
  replayed: boolean;
  benefit: CouponTokenBankBenefit | CouponPointsBenefit | CouponCashBenefit | CouponSubscriptionBenefit;
}
