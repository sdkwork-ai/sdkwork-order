export interface CheckoutOrder {
  orderId: string;
  orderNo: string;
  orderSn: string;
  status: string;
  totalAmount: string;
  /** Partner bound to the order's customer at creation time. */
  partnerId?: string;
}
