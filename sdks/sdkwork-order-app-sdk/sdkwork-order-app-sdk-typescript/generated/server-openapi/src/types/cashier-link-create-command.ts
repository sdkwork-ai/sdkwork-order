export interface CashierLinkCreateCommand {
  /** Order primary key carried by the legacy cashier URL. */
  orderId: string;
  /** Merchant trade number carried by the legacy cashier URL. It must match the number bound to the order or its latest payment attempt. */
  outTradeNo: string;
}
