import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { CheckoutOrder, CheckoutQuote, CheckoutSession, CreateCheckoutSessionRequest } from '../types';
export interface OrderCheckoutCheckoutSessionsOrdersCreateParams {
    idempotencyKey: string;
}
export declare class OrderCheckoutCheckoutSessionsOrdersApi {
    private client;
    constructor(client: HttpClient);
    /** Checkout sessions orders create. */
    create(checkoutSessionId: string, params: OrderCheckoutCheckoutSessionsOrdersCreateParams, requestOptions?: ApiRequestOptions): Promise<CheckoutOrder>;
}
export interface OrderCheckoutCheckoutSessionsQuotesCreateParams {
    idempotencyKey: string;
}
export declare class OrderCheckoutCheckoutSessionsQuotesApi {
    private client;
    constructor(client: HttpClient);
    /** Checkout sessions quotes create. */
    create(checkoutSessionId: string, params: OrderCheckoutCheckoutSessionsQuotesCreateParams, requestOptions?: ApiRequestOptions): Promise<CheckoutQuote>;
}
export interface OrderCheckoutCheckoutSessionsCreateParams {
    idempotencyKey: string;
}
export declare class OrderCheckoutCheckoutSessionsApi {
    private client;
    readonly quotes: OrderCheckoutCheckoutSessionsQuotesApi;
    readonly orders: OrderCheckoutCheckoutSessionsOrdersApi;
    constructor(client: HttpClient);
    /** Checkout sessions create. */
    create(body: CreateCheckoutSessionRequest, params: OrderCheckoutCheckoutSessionsCreateParams, requestOptions?: ApiRequestOptions): Promise<CheckoutSession>;
    /** Checkout sessions retrieve. */
    retrieve(checkoutSessionId: string, requestOptions?: ApiRequestOptions): Promise<CheckoutSession>;
}
export declare class OrderCheckoutCheckoutApi {
    readonly sessions: OrderCheckoutCheckoutSessionsApi;
    constructor(client: HttpClient);
}
export declare class OrderCheckoutApi {
    readonly checkout: OrderCheckoutCheckoutApi;
    constructor(client: HttpClient);
}
export declare function createOrderCheckoutApi(client: HttpClient): OrderCheckoutApi;
//# sourceMappingURL=order-checkout.d.ts.map