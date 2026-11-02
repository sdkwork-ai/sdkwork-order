import type { DemandClaimEvent } from './demand-claim-event';
import type { PageInfo } from './page-info';

export interface DemandClaimEventListResponse {
  code: 0;
  data: unknown & { items: DemandClaimEvent[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
