import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { CommerceOperationCommand, RechargeOrderCreateCommand, SdkWorkCommandData, SdkWorkPageData } from '../types';
export interface OrderRechargesRechargesPlansListParams {
    status?: string;
    page?: number;
    pageSize?: number;
}
export declare class OrderRechargesRechargesPlansApi {
    private client;
    constructor(client: HttpClient);
    /** Token Bank plans list. */
    list(params?: OrderRechargesRechargesPlansListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
}
export interface OrderRechargesRechargesOrdersListParams {
    subject?: string;
    status?: string;
    page?: number;
    pageSize?: number;
}
export interface OrderRechargesRechargesOrdersCreateParams {
    idempotencyKey: string;
}
export interface OrderRechargesRechargesOrdersCancelParams {
    idempotencyKey: string;
}
export declare class OrderRechargesRechargesOrdersApi {
    private client;
    constructor(client: HttpClient);
    /** Recharges orders list. */
    list(params?: OrderRechargesRechargesOrdersListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
    /** Recharges orders create. */
    create(body: RechargeOrderCreateCommand, params: OrderRechargesRechargesOrdersCreateParams, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
    /** Recharges orders retrieve. */
    retrieve(orderId: string, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
    /** Recharges orders cancel. */
    cancel(orderId: string, params: OrderRechargesRechargesOrdersCancelParams, body?: CommerceOperationCommand, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
}
export declare class OrderRechargesRechargesSettingsApi {
    private client;
    constructor(client: HttpClient);
    /** Recharges settings retrieve. */
    retrieve(requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
}
export interface OrderRechargesRechargesPackagesListParams {
    page?: number;
    pageSize?: number;
}
export declare class OrderRechargesRechargesPackagesApi {
    private client;
    constructor(client: HttpClient);
    /** Recharges packages list. */
    list(params?: OrderRechargesRechargesPackagesListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
}
export declare class OrderRechargesRechargesApi {
    readonly packages: OrderRechargesRechargesPackagesApi;
    readonly settings: OrderRechargesRechargesSettingsApi;
    readonly orders: OrderRechargesRechargesOrdersApi;
    readonly plans: OrderRechargesRechargesPlansApi;
    constructor(client: HttpClient);
}
export declare class OrderRechargesApi {
    readonly recharges: OrderRechargesRechargesApi;
    constructor(client: HttpClient);
}
export declare function createOrderRechargesApi(client: HttpClient): OrderRechargesApi;
//# sourceMappingURL=order-recharges.d.ts.map