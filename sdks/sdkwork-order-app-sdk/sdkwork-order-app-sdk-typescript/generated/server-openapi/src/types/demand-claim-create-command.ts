export interface DemandClaimCreateCommand {
  demandId: string;
  demandTitle: string;
  demandCategory?: string;
  demandType?: string;
  appForm?: string;
  appForms?: string[];
  budgetText?: string;
  budgetMin?: string;
  budgetMax?: string;
  bidDeadline?: string;
  publisherUserId: string;
  publisherCompanyName?: string;
  bidderCompanyId?: string;
  quote: string;
  quoteCurrency?: string;
  deliveryDays?: number;
  proposal: string;
  requestNo?: string;
}
