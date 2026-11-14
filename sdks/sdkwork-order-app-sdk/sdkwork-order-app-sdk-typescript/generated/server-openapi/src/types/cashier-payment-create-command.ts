export interface CashierPaymentCreateCommand {
  /** Identity-bound instruments such as balance are not available without signing in. */
  paymentMethod: 'wechat_pay' | 'wechat_h5' | 'wechat_jsapi' | 'alipay' | 'alipay_wap';
  /** WeChat payer openid issued by the anonymous IAM payment OAuth. Accepted only for wechat_jsapi and only together with openidBinding. */
  openid?: string;
  /** Signature binding the openid to this capability token. The caller cannot mint it; a mismatched or foreign binding is rejected. */
  openidBinding?: string;
}
