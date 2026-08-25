import type { OrderCancellation } from './order-cancellation';
import type { PageInfo } from './page-info';
export interface OrdersAdminCancellationsListResponse {
    code: 0;
    data: unknown & {
        items: OrderCancellation[];
        pageInfo: PageInfo;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=orders-admin-cancellations-list-response.d.ts.map