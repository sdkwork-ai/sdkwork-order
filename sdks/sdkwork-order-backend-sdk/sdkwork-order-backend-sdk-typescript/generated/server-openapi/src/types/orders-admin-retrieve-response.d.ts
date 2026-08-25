import type { OrderDetail } from './order-detail';
export interface OrdersAdminRetrieveResponse {
    code: 0;
    data: unknown & {
        item: OrderDetail;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=orders-admin-retrieve-response.d.ts.map