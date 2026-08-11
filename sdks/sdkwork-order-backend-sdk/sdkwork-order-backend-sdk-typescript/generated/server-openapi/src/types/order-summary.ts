export interface OrderSummary {
  orderId: string;
  orderSn: string;
  status: string;
  statusName: string;
  subject: string;
  totalAmount: string;
  paidAmount?: string;
  discountAmount?: string;
  quantity: string;
  createdAt: string;
  payTime?: string;
  expireTime?: string;
  paymentMethod?: string;
  /** Partner bound to the order's customer at creation time. */
  partnerId?: string;
  /** Partner display name snapshot at order creation. */
  partnerName?: string;
  /** Partner level number snapshot at order creation. */
  partnerLevelNo?: string;
  /** Partner status snapshot at order creation. */
  partnerStatus?: string;
}
