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
  /** Token Bank quota credited for recharge orders. */
  points?: string;
  partnerId?: string;
  partnerName?: string;
  partnerLevelNo?: string;
  /** Commerce scene the order was placed in. */
  scene: 'web' | 'h5' | 'app' | 'mini_program' | 'cashier' | 'membership' | 'recharge' | 'platform' | 'default';
  /** Canonical operator/attribution tags. Empty when untagged. */
  tags: string[];
  /** Operator note recorded beside the classification. */
  remark?: string;
}
