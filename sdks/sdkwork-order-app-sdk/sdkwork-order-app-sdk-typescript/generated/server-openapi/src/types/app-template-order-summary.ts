export interface AppTemplateOrderSummary {
  orderId: string;
  orderNo: string;
  templateUuid: string;
  templateName: string;
  versionUuid?: string | null;
  amount: string;
  currencyCode: string;
  status: 'paid' | 'pending_payment' | 'closed';
  fulfillmentStatus: string;
  paidAt?: string | null;
  createdAt: string;
}
