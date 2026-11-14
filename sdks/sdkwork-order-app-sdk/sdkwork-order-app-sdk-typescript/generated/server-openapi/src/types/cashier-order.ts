export interface CashierOrder {
  orderId: string;
  orderSn: string;
  subject: string;
  /** Payable amount in the currency's minor unit. */
  amount: string;
  currencyCode: string;
  status: string;
  payable: boolean;
  scene: string;
  allowedMethods: ('wechat_pay' | 'wechat_h5' | 'alipay' | 'alipay_wap')[];
  expireTime?: string | null;
}
