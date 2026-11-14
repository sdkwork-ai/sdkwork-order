export interface CashierPayment {
  paymentId: string;
  orderId: string;
  outTradeNo: string;
  amount: string;
  paymentMethod: string;
  paymentParams: Record<string, string>;
}
