import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { MembershipOrderCreateCommand, MembershipOrderCreateResult } from '../types';
export interface OrderMembershipsMembershipsOrdersCreateParams {
    idempotencyKey: string;
}
export declare class OrderMembershipsMembershipsOrdersApi {
    private client;
    constructor(client: HttpClient);
    /** Create or reuse a membership purchase-intent order. */
    create(body: MembershipOrderCreateCommand, params: OrderMembershipsMembershipsOrdersCreateParams, requestOptions?: ApiRequestOptions): Promise<MembershipOrderCreateResult>;
}
export declare class OrderMembershipsMembershipsApi {
    readonly orders: OrderMembershipsMembershipsOrdersApi;
    constructor(client: HttpClient);
}
export declare class OrderMembershipsApi {
    readonly memberships: OrderMembershipsMembershipsApi;
    constructor(client: HttpClient);
}
export declare function createOrderMembershipsApi(client: HttpClient): OrderMembershipsApi;
//# sourceMappingURL=order-memberships.d.ts.map