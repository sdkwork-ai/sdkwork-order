import type { DemandClaim } from './demand-claim';

export interface DemandClaimResponse {
  code: 0;
  data: unknown & { item: DemandClaim; };
  /** Server-owned request correlation id. */
  traceId: string;
}
