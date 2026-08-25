export interface CouponSubscriptionBenefit {
    kind: 'subscription';
    productId: string;
    skuId: string;
    packageId: string;
    period: 'day' | 'week' | 'month' | 'year';
    durationDays: string;
    dailyQuota: string;
    totalQuota: string;
    subscriptionId: string;
    startsAt: string;
    expiresAt: string;
}
//# sourceMappingURL=coupon-subscription-benefit.d.ts.map