export interface CouponTokenBankBenefit {
  kind: 'token_bank_credit';
  targetAsset: 'token_bank';
  /** Credited Token Bank Compute Credits in integer micro-points (1 Compute Credit = 1,000,000 micro-points). Convert to whole credits for display. */
  grantAmount: string;
}
