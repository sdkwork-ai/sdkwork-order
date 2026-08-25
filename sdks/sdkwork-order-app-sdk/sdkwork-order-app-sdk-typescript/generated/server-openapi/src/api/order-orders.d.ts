import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { CommerceOperationCommand, CouponRedemptionCreateCommand, CouponRedemptionResult, OrderPaymentSuccess, OrdersPaymentsWebhooksReceiveRequest, OrdersRefundsWebhooksReceiveRequest, RefundRequestCreateCommand, SdkWorkCommandData, SdkWorkPageData } from '../types';
export declare class OrderOrdersOrdersRefundsWebhooksApi {
    private client;
    constructor(client: HttpClient);
    /** Receive PSP refund webhook */
    receive(providerCode: string, body: OrdersRefundsWebhooksReceiveRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
}
export declare class OrderOrdersOrdersRefundsApi {
    readonly webhooks: OrderOrdersOrdersRefundsWebhooksApi;
    constructor(client: HttpClient);
}
export interface OrderOrdersOrdersReceiptsConfirmParams {
    idempotencyKey: string;
}
export declare class OrderOrdersOrdersReceiptsApi {
    private client;
    constructor(client: HttpClient);
    /** Orders receipt confirmations create. */
    confirm(orderId: string, params: OrderOrdersOrdersReceiptsConfirmParams, body?: CommerceOperationCommand, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
}
export interface OrderOrdersOrdersRefundRequestsListParams {
    status?: string;
    page?: number;
    pageSize?: number;
}
export interface OrderOrdersOrdersRefundRequestsCreateParams {
    idempotencyKey: string;
}
export declare class OrderOrdersOrdersRefundRequestsApi {
    private client;
    constructor(client: HttpClient);
    /** Order refund requests list. */
    list(params?: OrderOrdersOrdersRefundRequestsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
    /** Order refund requests create. */
    create(body: RefundRequestCreateCommand, params: OrderOrdersOrdersRefundRequestsCreateParams, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
    /** Order refund requests retrieve. */
    retrieve(refundRequestId: string, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
}
export interface OrderOrdersOrdersCouponRedemptionsCreateParams {
    idempotencyKey: string;
}
export declare class OrderOrdersOrdersCouponRedemptionsApi {
    private client;
    constructor(client: HttpClient);
    /** Redeem a coupon into Token Bank credit or a quota-limited subscription. */
    create(body: CouponRedemptionCreateCommand, params: OrderOrdersOrdersCouponRedemptionsCreateParams, requestOptions?: ApiRequestOptions): Promise<CouponRedemptionResult>;
}
export declare class OrderOrdersOrdersStatusApi {
    private client;
    constructor(client: HttpClient);
    /** Orders status retrieve. */
    retrieve(orderId: string, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
}
export declare class OrderOrdersOrdersStatisticsApi {
    private client;
    constructor(client: HttpClient);
    /** Orders statistics retrieve. */
    retrieve(requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
}
export declare class OrderOrdersOrdersPaymentSuccessApi {
    private client;
    constructor(client: HttpClient);
    /** Orders payment Success retrieve. */
    retrieve(orderId: string, requestOptions?: ApiRequestOptions): Promise<OrderPaymentSuccess>;
}
export interface OrderOrdersOrdersCancellationsCreateParams {
    idempotencyKey: string;
}
export declare class OrderOrdersOrdersCancellationsApi {
    private client;
    constructor(client: HttpClient);
    /** Orders cancellations create. */
    create(orderId: string, params: OrderOrdersOrdersCancellationsCreateParams, body?: CommerceOperationCommand, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
}
export interface OrderOrdersOrdersEventsListParams {
    page?: number;
    pageSize?: number;
}
export declare class OrderOrdersOrdersEventsApi {
    private client;
    constructor(client: HttpClient);
    /** Orders events list. */
    list(orderId: string, params?: OrderOrdersOrdersEventsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
}
export declare class OrderOrdersOrdersPaymentsWebhooksApi {
    private client;
    constructor(client: HttpClient);
    /** Receive PSP payment webhook */
    receive(providerCode: string, body: OrdersPaymentsWebhooksReceiveRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
}
export interface OrderOrdersOrdersPaymentsCreateParams {
    idempotencyKey: string;
}
export declare class OrderOrdersOrdersPaymentsApi {
    private client;
    readonly webhooks: OrderOrdersOrdersPaymentsWebhooksApi;
    constructor(client: HttpClient);
    /** Orders payments create. */
    create(orderId: string, body: CommerceOperationCommand, params: OrderOrdersOrdersPaymentsCreateParams, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
}
export interface OrderOrdersOrdersListParams {
    status?: string;
    page?: number;
    pageSize?: number;
}
export declare class OrderOrdersOrdersApi {
    private client;
    readonly payments: OrderOrdersOrdersPaymentsApi;
    readonly events: OrderOrdersOrdersEventsApi;
    readonly cancellations: OrderOrdersOrdersCancellationsApi;
    readonly paymentSuccess: OrderOrdersOrdersPaymentSuccessApi;
    readonly statistics: OrderOrdersOrdersStatisticsApi;
    readonly status: OrderOrdersOrdersStatusApi;
    readonly couponRedemptions: OrderOrdersOrdersCouponRedemptionsApi;
    readonly refundRequests: OrderOrdersOrdersRefundRequestsApi;
    readonly receipts: OrderOrdersOrdersReceiptsApi;
    readonly refunds: OrderOrdersOrdersRefundsApi;
    constructor(client: HttpClient);
    /** Orders list. */
    list(params?: OrderOrdersOrdersListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
    /** Orders retrieve. */
    retrieve(orderId: string, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
}
export declare class OrderOrdersApi {
    readonly orders: OrderOrdersOrdersApi;
    constructor(client: HttpClient);
}
export declare function createOrderOrdersApi(client: HttpClient): OrderOrdersApi;
//# sourceMappingURL=order-orders.d.ts.map