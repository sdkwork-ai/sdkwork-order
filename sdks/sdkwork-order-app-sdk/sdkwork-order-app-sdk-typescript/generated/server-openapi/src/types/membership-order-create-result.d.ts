export interface MembershipOrderCreateResult {
    action: 'purchase' | 'renew' | 'upgrade' | 'recharge';
    orderId: string;
    orderNo: string;
    outTradeNo: string;
    amount: string;
    currencyCode: string;
    packageId: string;
    packageName: string;
    durationDays: string;
    expiresAt: string;
    paymentMethod: string;
    paymentProduct: 'mobile_cashier_h5' | 'wechat_native' | 'alipay_native';
    qrCode: string;
    qrCodeType: 'cashier_url' | 'provider_native';
    paymentId?: string | null;
    paymentParams: Record<string, string>;
    status: string;
    /** True when an idempotency replay or an existing active purchase intent was returned. */
    reused: boolean;
    cashierUrl: string;
    /** Quota units granted by a membership quota recharge order (present when action=recharge). */
    grantQuantity: string;
}
//# sourceMappingURL=membership-order-create-result.d.ts.map