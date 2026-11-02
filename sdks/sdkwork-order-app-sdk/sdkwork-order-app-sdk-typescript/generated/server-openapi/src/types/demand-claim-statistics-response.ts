import type { DemandClaimStatistics } from './demand-claim-statistics';

export interface DemandClaimStatisticsResponse {
  code: 0;
  data: unknown & { item: DemandClaimStatistics; };
  /** Server-owned request correlation id. */
  traceId: string;
}
