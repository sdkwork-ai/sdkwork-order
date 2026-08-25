import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { SdkWorkPageData } from '../types';
export interface OrderShipmentsShipmentsTrackingEventsListParams {
    page?: number;
    pageSize?: number;
}
export declare class OrderShipmentsShipmentsTrackingEventsApi {
    private client;
    constructor(client: HttpClient);
    /** Shipments tracking Events list. */
    list(shipmentId: string, params?: OrderShipmentsShipmentsTrackingEventsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
}
export interface OrderShipmentsShipmentsPackagesListParams {
    page?: number;
    pageSize?: number;
}
export declare class OrderShipmentsShipmentsPackagesApi {
    private client;
    constructor(client: HttpClient);
    /** Shipments packages list. */
    list(shipmentId: string, params?: OrderShipmentsShipmentsPackagesListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
}
export declare class OrderShipmentsShipmentsApi {
    private client;
    readonly packages: OrderShipmentsShipmentsPackagesApi;
    readonly trackingEvents: OrderShipmentsShipmentsTrackingEventsApi;
    constructor(client: HttpClient);
    /** Shipments retrieve. */
    retrieve(shipmentId: string, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
}
export declare class OrderShipmentsApi {
    readonly shipments: OrderShipmentsShipmentsApi;
    constructor(client: HttpClient);
}
export declare function createOrderShipmentsApi(client: HttpClient): OrderShipmentsApi;
//# sourceMappingURL=order-shipments.d.ts.map