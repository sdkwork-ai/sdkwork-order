export interface WithdrawalRequestCreateCommand {
    asset?: 'cash';
    amount: string | number;
    currencyCode: string;
    payoutMethod?: string;
    payoutAccountRef?: string;
    reasonCode?: string;
}
//# sourceMappingURL=withdrawal-request-create-command.d.ts.map