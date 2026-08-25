import type { OrderEvent } from './order-event';
import type { PageInfo } from './page-info';
export interface OrdersAdminEventsListResponse {
    code: 0;
    data: unknown & {
        items: OrderEvent[];
        pageInfo: PageInfo;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=orders-admin-events-list-response.d.ts.map