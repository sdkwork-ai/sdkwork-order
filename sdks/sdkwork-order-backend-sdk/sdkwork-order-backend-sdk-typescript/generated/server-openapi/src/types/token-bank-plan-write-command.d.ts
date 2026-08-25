export interface TokenBankPlanWriteCommand {
    planCode: string;
    displayName: string;
    planPeriod: 'monthly' | 'quarterly' | 'yearly' | 'continuous_monthly' | 'continuous_yearly';
    grantAmount: string;
    bonusAmount?: string;
    priceAmount: string;
    currencyCode: string;
    renewalPolicy?: string;
    status?: string;
    sortWeight?: number;
}
//# sourceMappingURL=token-bank-plan-write-command.d.ts.map