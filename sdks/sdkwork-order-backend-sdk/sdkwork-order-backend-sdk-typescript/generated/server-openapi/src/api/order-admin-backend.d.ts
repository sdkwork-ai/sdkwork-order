import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { AccountValuePackageWriteCommand, AccountValueRequestReviewCommand, SdkWorkCommandData, SdkWorkPageData, TokenBankPlanWriteCommand } from '../types';
export interface OrderAdminBackendBackendWithdrawalRequestsListParams {
    status?: string;
    page?: number;
    pageSize?: number;
}
export interface OrderAdminBackendBackendWithdrawalRequestsApproveParams {
    idempotencyKey: string;
}
export interface OrderAdminBackendBackendWithdrawalRequestsRejectParams {
    idempotencyKey: string;
}
export interface OrderAdminBackendBackendWithdrawalRequestsRetryParams {
    idempotencyKey: string;
}
export declare class OrderAdminBackendBackendWithdrawalRequestsApi {
    private client;
    constructor(client: HttpClient);
    /** Withdrawal requests list. */
    list(params?: OrderAdminBackendBackendWithdrawalRequestsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
    /** Withdrawal requests approve. */
    approve(withdrawalRequestId: string, params: OrderAdminBackendBackendWithdrawalRequestsApproveParams, body?: AccountValueRequestReviewCommand, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
    /** Withdrawal requests reject. */
    reject(withdrawalRequestId: string, params: OrderAdminBackendBackendWithdrawalRequestsRejectParams, body?: AccountValueRequestReviewCommand, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
    /** Withdrawal requests retry. */
    retry(withdrawalRequestId: string, params: OrderAdminBackendBackendWithdrawalRequestsRetryParams, body?: AccountValueRequestReviewCommand, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
}
export interface OrderAdminBackendBackendRefundRequestsListParams {
    status?: string;
    page?: number;
    pageSize?: number;
}
export interface OrderAdminBackendBackendRefundRequestsApproveParams {
    idempotencyKey: string;
}
export interface OrderAdminBackendBackendRefundRequestsRejectParams {
    idempotencyKey: string;
}
export interface OrderAdminBackendBackendRefundRequestsRetryParams {
    idempotencyKey: string;
}
export declare class OrderAdminBackendBackendRefundRequestsApi {
    private client;
    constructor(client: HttpClient);
    /** Refund requests list. */
    list(params?: OrderAdminBackendBackendRefundRequestsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
    /** Refund requests approve. */
    approve(refundRequestId: string, params: OrderAdminBackendBackendRefundRequestsApproveParams, body?: AccountValueRequestReviewCommand, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
    /** Refund requests reject. */
    reject(refundRequestId: string, params: OrderAdminBackendBackendRefundRequestsRejectParams, body?: AccountValueRequestReviewCommand, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
    /** Refund requests retry. */
    retry(refundRequestId: string, params: OrderAdminBackendBackendRefundRequestsRetryParams, body?: AccountValueRequestReviewCommand, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
}
export interface OrderAdminBackendBackendTokenBankPlansListParams {
    status?: string;
    page?: number;
    pageSize?: number;
}
export interface OrderAdminBackendBackendTokenBankPlansCreateParams {
    idempotencyKey: string;
}
export interface OrderAdminBackendBackendTokenBankPlansUpdateParams {
    idempotencyKey: string;
}
export interface OrderAdminBackendBackendTokenBankPlansRetireParams {
    idempotencyKey: string;
}
export declare class OrderAdminBackendBackendTokenBankPlansApi {
    private client;
    constructor(client: HttpClient);
    /** Token Bank plans list. */
    list(params?: OrderAdminBackendBackendTokenBankPlansListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
    /** Token Bank plans create. */
    create(body: TokenBankPlanWriteCommand, params: OrderAdminBackendBackendTokenBankPlansCreateParams, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
    /** Token Bank plans update. */
    update(planCode: string, body: TokenBankPlanWriteCommand, params: OrderAdminBackendBackendTokenBankPlansUpdateParams, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
    /** Token Bank plans retire. */
    retire(planCode: string, params: OrderAdminBackendBackendTokenBankPlansRetireParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
}
export interface OrderAdminBackendBackendAccountValuePackagesListParams {
    targetAsset?: string;
    status?: string;
    page?: number;
    pageSize?: number;
}
export interface OrderAdminBackendBackendAccountValuePackagesCreateParams {
    idempotencyKey: string;
}
export interface OrderAdminBackendBackendAccountValuePackagesUpdateParams {
    idempotencyKey: string;
}
export interface OrderAdminBackendBackendAccountValuePackagesRetireParams {
    idempotencyKey: string;
}
export declare class OrderAdminBackendBackendAccountValuePackagesApi {
    private client;
    constructor(client: HttpClient);
    /** Account value packages list. */
    list(params?: OrderAdminBackendBackendAccountValuePackagesListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData>;
    /** Account value packages create. */
    create(body: AccountValuePackageWriteCommand, params: OrderAdminBackendBackendAccountValuePackagesCreateParams, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
    /** Account value packages update. */
    update(packageId: string, body: AccountValuePackageWriteCommand, params: OrderAdminBackendBackendAccountValuePackagesUpdateParams, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>>;
    /** Account value packages retire. */
    retire(packageId: string, params: OrderAdminBackendBackendAccountValuePackagesRetireParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData>;
}
export declare class OrderAdminBackendBackendApi {
    readonly accountValuePackages: OrderAdminBackendBackendAccountValuePackagesApi;
    readonly tokenBankPlans: OrderAdminBackendBackendTokenBankPlansApi;
    readonly refundRequests: OrderAdminBackendBackendRefundRequestsApi;
    readonly withdrawalRequests: OrderAdminBackendBackendWithdrawalRequestsApi;
    constructor(client: HttpClient);
}
export declare class OrderAdminBackendApi {
    readonly backend: OrderAdminBackendBackendApi;
    constructor(client: HttpClient);
}
export declare function createOrderAdminBackendApi(client: HttpClient): OrderAdminBackendApi;
//# sourceMappingURL=order-admin-backend.d.ts.map