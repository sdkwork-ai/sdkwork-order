import type { OrderSummary } from './order-summary';
import type { PageInfo } from './page-info';
export interface OrdersAdminListResponse {
    code: 0;
    data: unknown & {
        items: OrderSummary[];
        pageInfo: PageInfo;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=orders-admin-list-response.d.ts.map