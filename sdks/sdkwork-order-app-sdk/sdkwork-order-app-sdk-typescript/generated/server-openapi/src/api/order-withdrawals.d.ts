import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { WithdrawalRequestCreateCommand } from '../types';
export interface OrderWithdrawalsWithdrawalsRequestsCreateParams {
    idempotencyKey: string;
}
export declare class OrderWithdrawalsWithdrawalsRequestsApi {
    private client;
    constructor(client: HttpClient);
    /** Withdrawal requests create. */
    create(body: WithdrawalRequestCreateCommand, params: OrderWithdrawalsWithdrawalsRequestsCreateParams, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
    /** Withdrawal requests retrieve. */
    retrieve(withdrawalRequestId: string, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
}
export declare class OrderWithdrawalsWithdrawalsApi {
    readonly requests: OrderWithdrawalsWithdrawalsRequestsApi;
    constructor(client: HttpClient);
}
export declare class OrderWithdrawalsApi {
    readonly withdrawals: OrderWithdrawalsWithdrawalsApi;
    constructor(client: HttpClient);
}
export declare function createOrderWithdrawalsApi(client: HttpClient): OrderWithdrawalsApi;
//# sourceMappingURL=order-withdrawals.d.ts.map