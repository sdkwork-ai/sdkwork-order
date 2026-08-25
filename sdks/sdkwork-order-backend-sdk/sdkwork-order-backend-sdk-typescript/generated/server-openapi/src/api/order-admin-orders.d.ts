import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { AdminCreateRefundRequest, CancelOrderRequest, CloseOrderRequest, ConfirmOrderPaymentRequest, OrderCancellation, OrderDetail, OrderEvent, OrderSummary, PageInfo, SdkWorkCommandData } from '../types';
export interface OrderAdminOrdersOrdersPaymentConfirmationsCreateParams {
    idempotencyKey: string;
}
export declare class OrderAdminOrdersOrdersPaymentConfirmationsApi {
    private client;
    constructor(client: HttpClient);
    /** Reconcile provider payment and run the shared order settlement saga */
    create(orderId: string, body: ConfirmOrderPaymentRequest, params: OrderAdminOrdersOrdersPaymentConfirmationsCreateParams, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
}
export interface OrderAdminOrdersOrdersAdminEventsListParams {
    page?: string;
    pageSize?: string;
}
export declare class OrderAdminOrdersOrdersAdminEventsApi {
    private client;
    constructor(client: HttpClient);
    /** List order lifecycle events */
    list(orderId: string, params?: OrderAdminOrdersOrdersAdminEventsListParams, requestOptions?: ApiRequestOptions): Promise<{
        items: OrderEvent[];
        pageInfo: PageInfo;
    }>;
}
export interface OrderAdminOrdersOrdersAdminRefundRequestsCreateParams {
    idempotencyKey: string;
}
export declare class OrderAdminOrdersOrdersAdminRefundRequestsApi {
    private client;
    constructor(client: HttpClient);
    /** Create an order refund request from the admin surface */
    create(orderId: string, body: AdminCreateRefundRequest, params: OrderAdminOrdersOrdersAdminRefundRequestsCreateParams, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
}
export interface OrderAdminOrdersOrdersAdminCancellationsListParams {
    status?: string;
    page?: string;
    pageSize?: string;
}
export declare class OrderAdminOrdersOrdersAdminCancellationsApi {
    private client;
    constructor(client: HttpClient);
    /** List order cancellation audit records */
    list(params?: OrderAdminOrdersOrdersAdminCancellationsListParams, requestOptions?: ApiRequestOptions): Promise<{
        items: OrderCancellation[];
        pageInfo: PageInfo;
    }>;
}
export interface OrderAdminOrdersOrdersAdminListParams {
    status?: string;
    q?: string;
    createdFrom?: string;
    createdTo?: string;
    page?: string;
    pageSize?: string;
}
export interface OrderAdminOrdersOrdersAdminCancelParams {
    idempotencyKey: string;
}
export interface OrderAdminOrdersOrdersAdminCloseParams {
    idempotencyKey: string;
}
export declare class OrderAdminOrdersOrdersAdminApi {
    private client;
    readonly cancellations: OrderAdminOrdersOrdersAdminCancellationsApi;
    readonly refundRequests: OrderAdminOrdersOrdersAdminRefundRequestsApi;
    readonly events: OrderAdminOrdersOrdersAdminEventsApi;
    constructor(client: HttpClient);
    /** List orders for operator review */
    list(params?: OrderAdminOrdersOrdersAdminListParams, requestOptions?: ApiRequestOptions): Promise<{
        items: OrderSummary[];
        pageInfo: PageInfo;
    }>;
    /** Retrieve order detail for operator review */
    retrieve(orderId: string, requestOptions?: ApiRequestOptions): Promise<OrderDetail>;
    /** Cancel an order from the admin surface */
    cancel(orderId: string, params: OrderAdminOrdersOrdersAdminCancelParams, body?: CancelOrderRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
    /** Close an order from the admin surface */
    close(orderId: string, params: OrderAdminOrdersOrdersAdminCloseParams, body?: CloseOrderRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
}
export declare class OrderAdminOrdersOrdersApi {
    readonly admin: OrderAdminOrdersOrdersAdminApi;
    readonly paymentConfirmations: OrderAdminOrdersOrdersPaymentConfirmationsApi;
    constructor(client: HttpClient);
}
export declare class OrderAdminOrdersApi {
    readonly orders: OrderAdminOrdersOrdersApi;
    constructor(client: HttpClient);
}
export declare function createOrderAdminOrdersApi(client: HttpClient): OrderAdminOrdersApi;
//# sourceMappingURL=order-admin-orders.d.ts.map