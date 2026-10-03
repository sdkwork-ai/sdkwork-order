export interface AppTemplateOrderCreateResult {
  orderId: string;
  orderNo: string;
  outTradeNo: string;
  templateUuid: string;
  templateName: string;
  /** Published template version the order snapshot points at. */
  versionUuid?: string | null;
  amount: string;
  currencyCode: string;
  expiresAt: string;
  paymentMethod: string;
  paymentProduct: 'mobile_cashier_h5' | 'wechat_native' | 'alipay_native';
  qrCode: string;
  qrCodeType: 'cashier_url' | 'provider_native';
  paymentId?: string | null;
  paymentParams: Record<string, string>;
  /** paid means the buyer already owns the listing. */
  status: 'paid' | 'pending_payment' | 'closed';
  /** True when an idempotency replay, an existing unpaid intent, or an already-settled order was returned. */
  reused: boolean;
  cashierUrl: string;
}
