import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { SdkWorkPageData } from '../types';
export interface OrderFulfillmentsFulfillmentsListParams {
    status?: string;
    page?: number;
    pageSize?: number;
}
export declare class OrderFulfillmentsFulfillmentsApi {
    private client;
    constructor(client: HttpClient);
    /** Fulfillments list. */
    list(params?: OrderFulfillmentsFulfillmentsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
    /** Fulfillments retrieve. */
    retrieve(fulfillmentId: string, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
}
export declare class OrderFulfillmentsApi {
    readonly fulfillments: OrderFulfillmentsFulfillmentsApi;
    constructor(client: HttpClient);
}
export declare function createOrderFulfillmentsApi(client: HttpClient): OrderFulfillmentsApi;
//# sourceMappingURL=order-fulfillments.d.ts.map