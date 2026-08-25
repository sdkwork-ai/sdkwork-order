export interface MembershipOrderCreateCommand {
    /** Membership lifecycle action. Recharge adds quota to the current active subscription's entitlement account; it participates in business purchase-intent identity. */
    action: 'purchase' | 'renew' | 'upgrade' | 'recharge';
    /** Catalog package external id (purchase/renew/upgrade). Recharge uses the synthetic 'membership-quota-recharge' package id and ignores the catalog. */
    packageId: string;
    paymentMethod: string;
    /** QR payment product. H5 returns the order-bound cashierUrl; native products create a provider payment intent. */
    paymentProduct?: 'mobile_cashier_h5' | 'wechat_native' | 'alipay_native';
    clientRequestNo?: string;
    source?: string;
    /** Quota units to add to the membership entitlement account (required for action=recharge). */
    grantQuantity?: string;
    /** Recharge price in currency amount (required for action=recharge). */
    amount?: string;
}
//# sourceMappingURL=membership-order-create-command.d.ts.map