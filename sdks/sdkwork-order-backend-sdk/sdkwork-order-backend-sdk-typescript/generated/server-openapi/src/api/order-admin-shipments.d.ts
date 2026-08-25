import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { CreateShipmentPackageRequest, PageInfo, ShipmentPackageSummary, ShipmentSummary, UpdateShipmentPackageRequest } from '../types';
export interface OrderAdminShipmentsShipmentsPackagesManagementListParams {
    page?: string;
    pageSize?: string;
}
export declare class OrderAdminShipmentsShipmentsPackagesManagementApi {
    private client;
    constructor(client: HttpClient);
    /** List shipment packages */
    list(shipmentId: string, params?: OrderAdminShipmentsShipmentsPackagesManagementListParams, requestOptions?: ApiRequestOptions): Promise<{
        items: ShipmentPackageSummary[];
        pageInfo: PageInfo;
    }>;
}
export interface OrderAdminShipmentsShipmentsPackagesCreateParams {
    idempotencyKey: string;
}
export interface OrderAdminShipmentsShipmentsPackagesUpdateParams {
    idempotencyKey: string;
}
export declare class OrderAdminShipmentsShipmentsPackagesApi {
    private client;
    readonly management: OrderAdminShipmentsShipmentsPackagesManagementApi;
    constructor(client: HttpClient);
    /** Create shipment package */
    create(shipmentId: string, body: CreateShipmentPackageRequest, params: OrderAdminShipmentsShipmentsPackagesCreateParams, requestOptions?: ApiRequestOptions): Promise<ShipmentPackageSummary>;
    /** Update shipment package */
    update(shipmentId: string, packageId: string, body: UpdateShipmentPackageRequest, params: OrderAdminShipmentsShipmentsPackagesUpdateParams, requestOptions?: ApiRequestOptions): Promise<ShipmentPackageSummary>;
}
export interface OrderAdminShipmentsShipmentsListParams {
    status?: string;
    orderId?: string;
    fulfillmentId?: string;
    page?: string;
    pageSize?: string;
}
export declare class OrderAdminShipmentsShipmentsApi {
    private client;
    readonly packages: OrderAdminShipmentsShipmentsPackagesApi;
    constructor(client: HttpClient);
    /** List shipments for operator review */
    list(params?: OrderAdminShipmentsShipmentsListParams, requestOptions?: ApiRequestOptions): Promise<{
        items: ShipmentSummary[];
        pageInfo: PageInfo;
    }>;
    /** Retrieve shipment for operator review */
    retrieve(shipmentId: string, requestOptions?: ApiRequestOptions): Promise<ShipmentSummary>;
}
export declare class OrderAdminShipmentsApi {
    readonly shipments: OrderAdminShipmentsShipmentsApi;
    constructor(client: HttpClient);
}
export declare function createOrderAdminShipmentsApi(client: HttpClient): OrderAdminShipmentsApi;
//# sourceMappingURL=order-admin-shipments.d.ts.map