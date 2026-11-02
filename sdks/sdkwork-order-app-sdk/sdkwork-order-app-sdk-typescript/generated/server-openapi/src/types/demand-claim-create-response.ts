import type { DemandClaim } from './demand-claim';

export interface DemandClaimCreateResponse {
  code: 0;
  data: unknown & { item: DemandClaim; };
  /** Server-owned request correlation id. */
  traceId: string;
}
