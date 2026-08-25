import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { SdkWorkPageData } from '../types';
export interface OrderPaymentsPaymentsOrderPaymentsListParams {
    page?: number;
    pageSize?: number;
}
export declare class OrderPaymentsPaymentsOrderPaymentsApi {
    private client;
    constructor(client: HttpClient);
    /** Payments order Payments list. */
    list(orderId: string, params?: OrderPaymentsPaymentsOrderPaymentsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
}
export declare class OrderPaymentsPaymentsApi {
    readonly orderPayments: OrderPaymentsPaymentsOrderPaymentsApi;
    constructor(client: HttpClient);
}
export declare class OrderPaymentsApi {
    readonly payments: OrderPaymentsPaymentsApi;
    constructor(client: HttpClient);
}
export declare function createOrderPaymentsApi(client: HttpClient): OrderPaymentsApi;
//# sourceMappingURL=order-payments.d.ts.map