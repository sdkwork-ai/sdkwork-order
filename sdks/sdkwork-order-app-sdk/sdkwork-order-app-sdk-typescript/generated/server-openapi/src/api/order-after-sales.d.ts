import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { AfterSalesRequestResponse, AfterSalesReturnShipmentResponse, CreateAfterSalesRequest, CreateAfterSalesReturnShipmentRequest, SdkWorkPageData, UpdateAfterSalesRequest } from '../types';
export interface OrderAfterSalesAfterSalesEventsListParams {
    page?: number;
    pageSize?: number;
}
export declare class OrderAfterSalesAfterSalesEventsApi {
    private client;
    constructor(client: HttpClient);
    /** After Sales events list. */
    list(afterSalesRequestId: string, params?: OrderAfterSalesAfterSalesEventsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
}
export interface OrderAfterSalesAfterSalesReturnShipmentsListParams {
    status?: string;
    page?: number;
    pageSize?: number;
}
export interface OrderAfterSalesAfterSalesReturnShipmentsCreateParams {
    idempotencyKey: string;
}
export declare class OrderAfterSalesAfterSalesReturnShipmentsApi {
    private client;
    constructor(client: HttpClient);
    /** After Sales return Shipments list. */
    list(afterSalesRequestId: string, params?: OrderAfterSalesAfterSalesReturnShipmentsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
    /** After Sales return Shipments create. */
    create(afterSalesRequestId: string, body: CreateAfterSalesReturnShipmentRequest, params: OrderAfterSalesAfterSalesReturnShipmentsCreateParams, requestOptions?: ApiRequestOptions): Promise<AfterSalesReturnShipmentResponse>;
}
export interface OrderAfterSalesAfterSalesRequestsListParams {
    status?: string;
    orderId?: string;
    page?: number;
    pageSize?: number;
}
export interface OrderAfterSalesAfterSalesRequestsCreateParams {
    idempotencyKey: string;
}
export interface OrderAfterSalesAfterSalesRequestsUpdateParams {
    idempotencyKey: string;
}
export declare class OrderAfterSalesAfterSalesRequestsApi {
    private client;
    constructor(client: HttpClient);
    /** After Sales requests list. */
    list(params?: OrderAfterSalesAfterSalesRequestsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
    /** After Sales requests create. */
    create(body: CreateAfterSalesRequest, params: OrderAfterSalesAfterSalesRequestsCreateParams, requestOptions?: ApiRequestOptions): Promise<AfterSalesRequestResponse>;
    /** After Sales requests retrieve. */
    retrieve(afterSalesRequestId: string, requestOptions?: ApiRequestOptions): Promise<AfterSalesRequestResponse>;
    /** After Sales requests update. */
    update(afterSalesRequestId: string, body: UpdateAfterSalesRequest, params: OrderAfterSalesAfterSalesRequestsUpdateParams, requestOptions?: ApiRequestOptions): Promise<AfterSalesRequestResponse>;
}
export declare class OrderAfterSalesAfterSalesApi {
    readonly requests: OrderAfterSalesAfterSalesRequestsApi;
    readonly returnShipments: OrderAfterSalesAfterSalesReturnShipmentsApi;
    readonly events: OrderAfterSalesAfterSalesEventsApi;
    constructor(client: HttpClient);
}
export declare class OrderAfterSalesApi {
    readonly afterSales: OrderAfterSalesAfterSalesApi;
    constructor(client: HttpClient);
}
export declare function createOrderAfterSalesApi(client: HttpClient): OrderAfterSalesApi;
//# sourceMappingURL=order-after-sales.d.ts.map