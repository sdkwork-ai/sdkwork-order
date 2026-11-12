export interface CouponCashBenefit {
  kind: 'cash_credit';
  /** Cash grant in the currency's minor unit (for example 1250 = 12.50). */
  grantAmount: string;
  /** ISO-4217 currency of the cash wallet that was credited. */
  currencyCode: string;
}
