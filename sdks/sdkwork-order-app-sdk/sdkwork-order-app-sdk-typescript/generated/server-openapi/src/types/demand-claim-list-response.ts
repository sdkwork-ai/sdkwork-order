import type { DemandClaim } from './demand-claim';
import type { PageInfo } from './page-info';

export interface DemandClaimListResponse {
  code: 0;
  data: unknown & { items: DemandClaim[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
