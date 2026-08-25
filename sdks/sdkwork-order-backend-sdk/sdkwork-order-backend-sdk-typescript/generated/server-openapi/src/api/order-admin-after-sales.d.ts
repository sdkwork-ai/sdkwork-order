import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { AfterSalesRequestSummary, PageInfo, ReviewAfterSalesRequest } from '../types';
export interface OrderAdminAfterSalesAfterSalesReviewsCreateParams {
    idempotencyKey: string;
}
export declare class OrderAdminAfterSalesAfterSalesReviewsApi {
    private client;
    constructor(client: HttpClient);
    /** Review after-sales request */
    create(afterSalesRequestId: string, body: ReviewAfterSalesRequest, params: OrderAdminAfterSalesAfterSalesReviewsCreateParams, requestOptions?: ApiRequestOptions): Promise<AfterSalesRequestSummary>;
}
export interface OrderAdminAfterSalesAfterSalesManagementListParams {
    status?: string;
    afterSalesType?: string;
    orderId?: string;
    page?: string;
    pageSize?: string;
}
export declare class OrderAdminAfterSalesAfterSalesManagementApi {
    private client;
    constructor(client: HttpClient);
    /** List after-sales requests for operator review */
    list(params?: OrderAdminAfterSalesAfterSalesManagementListParams, requestOptions?: ApiRequestOptions): Promise<{
        items: AfterSalesRequestSummary[];
        pageInfo: PageInfo;
    }>;
    /** Retrieve after-sales request for operator review */
    retrieve(afterSalesRequestId: string, requestOptions?: ApiRequestOptions): Promise<AfterSalesRequestSummary>;
}
export declare class OrderAdminAfterSalesAfterSalesApi {
    readonly management: OrderAdminAfterSalesAfterSalesManagementApi;
    readonly reviews: OrderAdminAfterSalesAfterSalesReviewsApi;
    constructor(client: HttpClient);
}
export declare class OrderAdminAfterSalesApi {
    readonly afterSales: OrderAdminAfterSalesAfterSalesApi;
    constructor(client: HttpClient);
}
export declare function createOrderAdminAfterSalesApi(client: HttpClient): OrderAdminAfterSalesApi;
//# sourceMappingURL=order-admin-after-sales.d.ts.map