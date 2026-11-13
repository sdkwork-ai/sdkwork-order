export interface CashierPaymentCreateCommand {
  /** Identity-bound instruments such as balance are not available without signing in. */
  paymentMethod: 'wechat_pay' | 'wechat_h5' | 'alipay' | 'alipay_wap';
}
