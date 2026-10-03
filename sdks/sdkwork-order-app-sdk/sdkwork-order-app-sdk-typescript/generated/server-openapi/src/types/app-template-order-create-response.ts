import type { AppTemplateOrderCreateResult } from './app-template-order-create-result';

export interface AppTemplateOrderCreateResponse {
  code: 0;
  data: unknown & { item: AppTemplateOrderCreateResult; };
  /** Server-owned request correlation id. */
  traceId: string;
}
