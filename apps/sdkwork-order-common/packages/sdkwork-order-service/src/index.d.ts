import type { SdkworkAppClient } from "@sdkwork/order-app-sdk";
import { type BootstrapSdkworkOrderAppServiceInput } from "./transport.ts";
type PublicSdkPort<T> = {
    readonly [TKey in keyof T]: T[TKey] extends (...args: infer TArgs) => infer TResult ? (...args: TArgs) => TResult : T[TKey] extends object ? PublicSdkPort<T[TKey]> : T[TKey];
};
export type SdkworkOrderOrdersService = PublicSdkPort<SdkworkAppClient["orders"]>;
export type SdkworkOrderCheckoutService = PublicSdkPort<SdkworkAppClient["checkout"]>;
export type SdkworkOrderRechargesService = PublicSdkPort<SdkworkAppClient["recharges"]>;
export type SdkworkOrderMembershipsService = PublicSdkPort<SdkworkAppClient["memberships"]>;
export type SdkworkOrderWithdrawalsService = PublicSdkPort<SdkworkAppClient["withdrawals"]>;
export type SdkworkOrderAppService = {
    checkout: SdkworkOrderCheckoutService;
    memberships: SdkworkOrderMembershipsService;
    orders: SdkworkOrderOrdersService;
    recharges: SdkworkOrderRechargesService;
    withdrawals: SdkworkOrderWithdrawalsService;
};
export interface SdkworkPhysicalPurchaseItem {
    quantity: number;
    skuId: string;
}
export interface SdkworkPhysicalShippingAddress {
    city: string;
    countryCode: string;
    detailAddress: string;
    district?: string;
    postalCode?: string;
    province: string;
    receiverName: string;
    receiverPhone: string;
}
export interface SdkworkPhysicalCheckoutInput {
    currencyCode?: string;
    items: SdkworkPhysicalPurchaseItem[];
    shippingAddress: SdkworkPhysicalShippingAddress;
}
export interface SdkworkPhysicalCheckout {
    checkoutSessionId: string;
    currencyCode: string;
    discountAmount: string;
    originalAmount: string;
    payableAmount: string;
    quoteId: string;
    status: string;
}
export interface SdkworkPhysicalOrder {
    orderId: string;
    orderNo: string;
    orderSn: string;
    status: string;
    totalAmount: string;
}
export interface SdkworkPhysicalPurchaseService {
    placeOrder(checkoutSessionId: string): Promise<SdkworkPhysicalOrder>;
    prepareCheckout(input: SdkworkPhysicalCheckoutInput): Promise<SdkworkPhysicalCheckout>;
}
export interface CreateSdkworkPhysicalPurchaseServiceOptions {
    appService?: SdkworkOrderAppService;
}
export type SdkworkMembershipCheckoutAction = "purchase" | "renew" | "upgrade" | "recharge";
export interface SdkworkMembershipCheckoutInput {
    action: SdkworkMembershipCheckoutAction;
    packageId: number;
    paymentMethod?: string;
    paymentProduct?: "alipay_native" | "mobile_cashier_h5" | "wechat_native";
    /** 订阅期额度充值数量（仅 action=recharge）。 */
    grantQuantity?: number;
    /** 订阅期额度充值金额（仅 action=recharge，货币金额字符串）。 */
    amountCny?: string;
}
export interface SdkworkMembershipCheckoutPayment {
    action?: SdkworkMembershipCheckoutAction;
    amountCny: number | null;
    cashierUrl?: string;
    durationDays: number | null;
    expiresAt?: string;
    orderId?: string;
    packageId: number | null;
    packageName?: string;
    qrCode?: string;
    reused?: boolean;
    status: "completed" | "failed" | "pending";
    targetLevelName?: string;
}
export interface SdkworkMembershipCheckoutService {
    createCheckout(input: SdkworkMembershipCheckoutInput): Promise<SdkworkMembershipCheckoutPayment>;
    getCheckoutStatus(orderId: string): Promise<SdkworkMembershipCheckoutPayment>;
}
export interface CreateSdkworkMembershipCheckoutServiceOptions {
    appService?: SdkworkOrderAppService;
}
export interface SdkworkPointsRechargePackage {
    id: string;
    priceAmount: number;
    currencyCode: string;
    bonusPoints: number;
    grantAmount: number;
    points: number;
}
export interface SdkworkPointsRechargePayment {
    amountCny: number | null;
    cashierUrl?: string;
    expiresAt?: string;
    orderId?: string;
    orderNo?: string;
    points: number;
    qrCode?: string;
    status: "completed" | "failed" | "pending";
}
export interface SdkworkPointsRechargeOrderInput {
    packageId: number | string;
    paymentMethod?: string;
    paymentProduct?: "alipay_native" | "mobile_cashier_h5" | "wechat_native";
    source?: string;
}
export interface SdkworkPointsRechargeService {
    listPackages(): Promise<SdkworkPointsRechargePackage[]>;
    createOrder(input: SdkworkPointsRechargeOrderInput): Promise<SdkworkPointsRechargePayment>;
    getOrderStatus(orderId: string): Promise<SdkworkPointsRechargePayment>;
}
export interface CreateSdkworkPointsRechargeServiceOptions {
    appService?: SdkworkOrderAppService;
}
export interface SdkworkCouponTokenBankRedemptionResult {
    benefitKind: "token_bank_credit";
    grantAmount: number;
    orderId: string;
    orderNo?: string;
    replayed: boolean;
    status: "completed" | "pending";
    targetAsset: "token_bank";
}
export interface SdkworkCouponPointsRedemptionResult {
    benefitKind: "points_credit";
    grantPoints: number;
    orderId: string;
    orderNo?: string;
    replayed: boolean;
    status: "completed" | "pending";
}
export interface SdkworkCouponCashRedemptionResult {
    benefitKind: "cash_credit";
    /** 现金发放金额（最小单位）。 */
    grantAmount: number;
    orderId: string;
    orderNo?: string;
    replayed: boolean;
    status: "completed" | "pending";
}
export type SdkworkCouponSubscriptionPeriod = "day" | "week" | "month" | "year";
export interface SdkworkCouponSubscriptionRedemptionResult {
    benefitKind: "subscription";
    dailyQuota: number;
    durationDays: number;
    expiresAt: string;
    orderId: string;
    orderNo?: string;
    packageId: string;
    period: SdkworkCouponSubscriptionPeriod;
    productId: string;
    replayed: boolean;
    skuId: string;
    startsAt: string;
    status: "completed" | "pending";
    subscriptionId: string;
    totalQuota: number;
}
export type SdkworkCouponRedemptionResult = SdkworkCouponTokenBankRedemptionResult | SdkworkCouponPointsRedemptionResult | SdkworkCouponCashRedemptionResult | SdkworkCouponSubscriptionRedemptionResult;
export type SdkworkCouponRechargeResult = SdkworkCouponRedemptionResult;
export interface SdkworkCouponRedemptionService {
    redeem(code: string): Promise<SdkworkCouponRedemptionResult>;
}
export type SdkworkCouponRechargeService = SdkworkCouponRedemptionService;
export interface CreateSdkworkCouponRechargeServiceOptions {
    appService?: SdkworkOrderAppService;
}
export type SdkworkOrderAppServiceProvider = () => SdkworkOrderAppService;
export interface SdkworkOrderSessionTokens {
    accessToken?: string;
    authToken?: string;
    refreshToken?: string;
}
export type SdkworkOrderSessionTokenProvider = () => SdkworkOrderSessionTokens;
export interface CreateSdkworkOrderAppServiceInput {
    appClient: PublicSdkPort<SdkworkAppClient>;
}
export interface SdkworkOrderResponseEnvelope<T> {
    code?: number | string;
    data?: T;
    message?: string;
    msg?: string;
}
export type SdkworkMediaKind = "archive" | "audio" | "document" | "image" | "model" | "other" | "video";
export type SdkworkMediaSource = "data_url" | "external_url" | "generated" | "object_storage" | "provider_asset";
export interface SdkworkMediaResource {
    kind: SdkworkMediaKind;
    publicUrl?: string;
    source: SdkworkMediaSource;
    url?: string;
    [key: string]: unknown;
}
export declare function configureSdkworkOrderAppServiceProvider(provider: SdkworkOrderAppServiceProvider | null): void;
export declare function configureSdkworkOrderSessionTokenProvider(provider: SdkworkOrderSessionTokenProvider | null): void;
export declare function getSdkworkOrderService(): SdkworkOrderAppService;
export declare function getSdkworkOrderSessionTokens(): SdkworkOrderSessionTokens;
export declare function hasSdkworkOrderSession(): boolean;
export declare function requireSdkworkOrderSession(message?: string): void;
export declare function createSdkworkOrderAppService(input: CreateSdkworkOrderAppServiceInput): SdkworkOrderAppService;
export declare function createSdkworkPhysicalPurchaseService(options?: CreateSdkworkPhysicalPurchaseServiceOptions): SdkworkPhysicalPurchaseService;
export type CreateSdkworkCouponRedemptionServiceOptions = CreateSdkworkCouponRechargeServiceOptions;
export declare function createSdkworkPointsRechargeService(options?: CreateSdkworkPointsRechargeServiceOptions): SdkworkPointsRechargeService;
export declare function createSdkworkCouponRechargeService(options?: CreateSdkworkCouponRechargeServiceOptions): SdkworkCouponRechargeService;
export declare function createSdkworkMembershipCheckoutService(options?: CreateSdkworkMembershipCheckoutServiceOptions): SdkworkMembershipCheckoutService;
export declare function unwrapSdkworkOrderResource<T>(value: unknown, fallbackMessage?: string): T;
export interface SdkworkOffsetPageInfo {
    hasMore?: boolean;
    mode?: string;
    page?: number;
    pageSize?: number;
    totalItems?: number;
    totalPages?: number;
}
export interface SdkworkOffsetListPage<T> {
    items: T[];
    pageInfo?: SdkworkOffsetPageInfo;
}
export declare function unwrapSdkworkOrderPage<T>(value: unknown, fallbackMessage?: string): T[];
export declare function unwrapSdkworkOrderListPage<T>(value: unknown, fallbackMessage?: string): SdkworkOffsetListPage<T>;
export declare function resolveSdkworkOffsetPagination(pageInfo: SdkworkOffsetPageInfo | null | undefined, fallbackPage: number, fallbackPageSize: number): {
    hasMore: boolean;
    page: number;
    pageSize: number;
    total: number;
    totalPages: number;
};
/** Maps UI kebab-case order status filters to API snake_case wire values. */
export declare function toApiOrderStatusWire(status: string): string;
export declare function unwrapSdkworkOrderResponse<T>(value: unknown, fallbackMessage?: string): T;
export declare function toSdkworkOrderOptionalString(value: unknown): string | undefined;
export declare function toNullableSdkworkOrderNumber(value: unknown): number | null;
export declare function toSdkworkOrderNumber(value: unknown, fallback?: number): number;
export declare function formatSdkworkOrderCurrencyCny(value: number | null | undefined, language?: string): string;
export declare function readSdkworkMediaResource(value: unknown): SdkworkMediaResource | undefined;
export declare const createSdkworkCouponRedemptionService: typeof createSdkworkCouponRechargeService;
export declare function bootstrapSdkworkOrderAppService(input: BootstrapSdkworkOrderAppServiceInput): SdkworkOrderAppService;
export { createOrderAppTransportClient, resolveOrderAppApiOrigin, type BootstrapSdkworkOrderAppServiceInput, } from "./transport.ts";
export { bootstrapSdkworkOrderBackendSdk, createOrderBackendTransportClient, getSdkworkOrderBackendSdkClient, resetSdkworkOrderBackendSdkClient, resolveOrderBackendApiOrigin, type BootstrapSdkworkOrderBackendSdkInput, } from "./backend-transport.ts";
export { createSdkworkIdempotencyParams, type SdkworkIdempotencyParams, } from "./idempotency.ts";
//# sourceMappingURL=index.d.ts.map