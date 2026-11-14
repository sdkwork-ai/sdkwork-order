export interface MembershipOrderCreateCommand {
  /** Membership lifecycle action. Recharge adds quota to the current active subscription's entitlement account. `add_seats` expands the buyer organization's existing active/grace-period team subscription by `quantity` seats without extending the period (charged at the package's full seat price for the current term); it participates in business purchase-intent identity. */
  action: 'purchase' | 'renew' | 'upgrade' | 'recharge' | 'add_seats';
  /** Catalog package external id (purchase/renew/upgrade/add_seats). Recharge uses the synthetic 'membership-quota-recharge' package id and ignores the catalog. */
  packageId: string;
  paymentMethod: string;
  /** QR payment product. H5 returns the order-bound cashierUrl; native products create a provider payment intent. */
  paymentProduct?: 'mobile_cashier_h5' | 'wechat_native' | 'alipay_native';
  clientRequestNo?: string;
  source?: string;
  /** Quota units to add to the membership entitlement account (required for action=recharge). */
  grantQuantity?: string;
  /** Recharge price in currency amount (required for action=recharge). */
  amount?: string;
  /** Seat quantity for team/organization packages whose priceAmount is the price of one seat for one billing period. Defaults to 1; allowed for action purchase, renew, upgrade, and add_seats; rejected for recharge. For `add_seats` it is the number of seats to ADD to the existing subscription (1..10000), not the new total. */
  quantity?: string;
}
