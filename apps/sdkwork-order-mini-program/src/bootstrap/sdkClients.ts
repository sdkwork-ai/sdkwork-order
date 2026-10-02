/**
 * Order SDK client contract for the mini-program.
 *
 * The MP runtime cannot execute the generated TypeScript transport that
 * PC/H5 consume (`@sdkwork/order-app-sdk`), so the order surface is composed
 * from the hand-authored spec-shaped service modules in `src/services/` —
 * each one maps 1:1 onto the order app-api domain
 * (apis/app-api/order/order-app-api.openapi.json) exactly like the H5
 * services (OrderService / SubscriptionPurchaseService /
 * WithdrawalRequestService). Pages consume the domain services directly via
 * CommonJS; this module is the typed contract they collectively implement.
 * When the generated WeChat MP SDK family lands, each service swaps its
 * transport calls for the generated client without page changes.
 *
 * Wire rules (API_SPEC.md section 4.5/13/14/15):
 * - ids and int64 fields stay strings end to end (int64-as-string contract);
 * - amounts are minor-unit integer strings ("6990" = CNY 69.90);
 * - success payloads are the unwrapped `data` of { code, data, traceId };
 * - list payloads are { items, pageInfo } with pageInfo.mode offset|cursor.
 */

/** GET /orders item (backend OrderSummaryResponse, camelCase wire). */
export interface SdkworkOrderMpOrderSummary {
  orderId: string;
  orderSn: string;
  status: string;
  statusName: string;
  subject: string;
  /** Minor-unit amount string. */
  totalAmount: string;
  /** Minor-unit amount string, present once paid. */
  paidAmount?: string;
  /** Minor-unit amount string. */
  discountAmount?: string;
  quantity: number;
  createdAt: string;
  payTime?: string;
  expireTime?: string;
  paymentMethod?: string;
  remark?: string;
  points?: number;
}

/** GET /orders/{orderId} item (OrderDetailResponse = flattened summary + lines). */
export interface SdkworkOrderMpOrderDetail extends SdkworkOrderMpOrderSummary {
  items: Array<{
    id: string;
    productName: string;
    quantity: number;
    /** Minor-unit amount string. */
    unitPrice: string;
    /** Minor-unit amount string. */
    totalAmount: string;
  }>;
  outTradeNo?: string;
  transactionId?: string;
}

/** GET /orders/{orderId}/events item. */
export interface SdkworkOrderMpOrderEvent {
  eventId: string;
  orderId: string;
  eventType: string;
  fromStatus?: string;
  toStatus: string;
  actorType?: string;
  actorId?: string;
  message?: string;
  createdAt: string;
}

/** GET /orders/{orderId}/status item. */
export interface SdkworkOrderMpOrderStatus {
  status: string;
  statusName: string;
}

/** GET /orders/{orderId}/payment_success item (payment polling contract). */
export interface SdkworkOrderMpPaymentStatus {
  paid: boolean;
  status: string;
  statusName: string;
}

/** POST /orders/{orderId}/payments result (OrderPaymentParamsResponse). */
export interface SdkworkOrderMpPaymentSession {
  amount: string;
  orderId: string;
  outTradeNo: string;
  paymentId: string;
  paymentMethod: string;
  /** Provider params; consumed by wx.requestPayment once that bridge lands. */
  paymentParams: Record<string, string>;
}

/** GET /orders/statistics item. */
export interface SdkworkOrderMpOrderStatistics {
  totalOrders: number;
  pendingPayment: number;
  pendingShipment: number;
  pendingReceipt: number;
  completed: number;
  /** Minor-unit amount string. */
  totalAmount: string;
}

/** POST /orders/coupon_redemptions item (CouponRedemptionResult). */
export interface SdkworkOrderMpCouponRedemptionResult {
  orderId: string;
  orderNo: string;
  status: string;
  replayed: boolean;
  benefit:
    | { kind: "token_bank_credit"; targetAsset: string; grantAmount: string }
    | {
        kind: "subscription";
        productId: string;
        skuId: string;
        packageId: string;
        period: string;
        durationDays: string;
        dailyQuota: string;
        totalQuota: string;
        subscriptionId: string;
        startsAt: string;
        expiresAt: string;
      }
    | Record<string, unknown>;
}

/** Refund/withdrawal read model (backend AccountValueRequestResponse). */
export interface SdkworkOrderMpAccountValueRequest {
  accountValueRequestId: string;
  requestNo: string;
  originalOrderId?: string;
  ownerUserId: string;
  subject: string;
  targetAsset: string;
  /** Minor-unit amount string. */
  amount: string;
  currencyCode: string;
  status: string;
  providerReferenceId?: string;
  createdAt: string;
  updatedAt: string;
}

/** GET /recharges/plans item (TokenBankPlanResponse). */
export interface SdkworkOrderMpTokenBankPlan {
  planCode: string;
  displayName: string;
  planPeriod: string;
  /** Minor-unit grant/bonus/price amount strings. */
  grantAmount: string;
  bonusAmount: string;
  priceAmount: string;
  currencyCode: string;
  renewalPolicy: string;
  status: string;
}

/** GET /recharges/packages item (RechargePackageResponse). */
export interface SdkworkOrderMpRechargePackage {
  id: string;
  /** Minor-unit amount string. */
  priceAmount: string;
  currencyCode: string;
  bonusPoints: number;
  grantAmount: number;
  points: number;
}

/** POST /recharges/orders item (SubmitRechargeResponse). */
export interface SdkworkOrderMpRechargeOrderCreated {
  success: boolean;
  orderId: string;
  orderNo: string;
  outTradeNo: string;
  subject: string;
  targetAsset: string;
  amount: string;
  grantAmount: string;
  currencyCode: string;
  points: number;
  providerCode: string;
  paymentMethod: string;
  paymentProduct: string;
  expiresAt?: string;
  status: string;
  nextAction: string;
  cashierUrl: string;
  qrCodePayload: string;
  requestPaymentPayload?: string;
}

/** GET /recharges/orders item + GET /recharges/orders/{orderId} item. */
export interface SdkworkOrderMpRechargeOrder {
  orderId: string;
  orderNo: string;
  status: string;
  subject: string;
  /** Minor-unit amount string. */
  amount: string;
  points: number;
  createdAt: string;
}

/** PageInfo per API_SPEC.md section 16; totalItems stays an int64 string. */
export interface SdkworkOrderMpPageInfo {
  mode: "offset" | "cursor";
  page?: number;
  pageSize?: number;
  totalItems?: string;
  totalPages?: number;
  nextCursor?: string | null;
  hasMore?: boolean;
}

export interface SdkworkOrderMpListResult<TItem> {
  items: TItem[];
  pageInfo: SdkworkOrderMpPageInfo;
}

/**
 * Order domain service (src/services/order-service.js).
 * Every POST command sends an `Idempotency-Key` header, mirroring the
 * generated SDK transport and the required write-command headers on the
 * app-api route crate.
 */
export interface SdkworkOrderMpOrderService {
  listOrders(input?: {
    status?: string;
    page?: number;
    pageSize?: number;
  }): Promise<SdkworkOrderMpListResult<SdkworkOrderMpOrderSummary>>;
  getStatistics(): Promise<SdkworkOrderMpOrderStatistics>;
  getOrderDetail(orderId: string): Promise<SdkworkOrderMpOrderDetail>;
  getOrderEvents(
    orderId: string,
    input?: { page?: number; pageSize?: number },
  ): Promise<SdkworkOrderMpListResult<SdkworkOrderMpOrderEvent>>;
  getOrderStatus(orderId: string): Promise<SdkworkOrderMpOrderStatus>;
  getPaymentSuccess(orderId: string): Promise<SdkworkOrderMpPaymentStatus>;
  createPayment(
    orderId: string,
    paymentMethod: string,
  ): Promise<SdkworkOrderMpPaymentSession>;
  cancelOrder(orderId: string, cancelReason?: string): Promise<Record<string, unknown>>;
  confirmReceipt(orderId: string): Promise<Record<string, unknown>>;
  redeemCoupon(
    couponCode: string,
  ): Promise<SdkworkOrderMpCouponRedemptionResult>;
  listRefundRequests(input?: {
    page?: number;
    pageSize?: number;
  }): Promise<SdkworkOrderMpListResult<SdkworkOrderMpAccountValueRequest>>;
  createRefundRequest(input: {
    originalOrderId: string;
    targetAsset: "points" | "token_bank" | "cash";
    /** Minor-unit amount string. */
    amount: string;
    currencyCode: string;
    reasonCode?: string;
    reasonDetail?: string;
  }): Promise<SdkworkOrderMpAccountValueRequest>;
  /** Backoff poll of payment_success until paid or deadline (cashier TTL). */
  pollPaymentSuccess(
    orderId: string,
    options?: {
      intervalMs?: number;
      maxIntervalMs?: number;
      deadlineMs?: number;
      onCountdown?: (remainingSeconds: number) => void;
    },
  ): Promise<SdkworkOrderMpPaymentStatus & { timedOut: boolean }>;
}

/** Recharge domain service (src/services/recharge-service.js). */
export interface SdkworkOrderMpRechargeService {
  listPlans(): Promise<SdkworkOrderMpTokenBankPlan[]>;
  listPackages(): Promise<SdkworkOrderMpRechargePackage[]>;
  createRechargeOrder(input: {
    subject:
      | "points_recharge"
      | "token_bank_recharge"
      | "token_bank_plan_purchase"
      | "token_bank_plan_renewal"
      | "account_recharge_package"
      | "coupon_recharge";
    targetAsset: "points" | "token_bank" | "cash";
    /** Minor-unit amount string. */
    amount: string;
    grantAmount?: string;
    currencyCode: string;
    packageId?: string;
    planCode?: string;
    planPeriod?: string;
    paymentMethod: string;
    source?: string;
  }): Promise<SdkworkOrderMpRechargeOrderCreated>;
  getRechargeOrder(orderId: string): Promise<SdkworkOrderMpRechargeOrder>;
  listRechargeOrders(input?: {
    page?: number;
    pageSize?: number;
  }): Promise<SdkworkOrderMpListResult<SdkworkOrderMpRechargeOrder>>;
  cancelRechargeOrder(orderId: string, cancelReason?: string): Promise<Record<string, unknown>>;
}

/**
 * Withdrawal domain service (src/services/withdrawal-service.js).
 * The order app-api has POST /withdrawals/requests and
 * GET /withdrawals/requests/{id} only (no owner list endpoint yet), so the
 * page-level records view combines the bounded device-local submission
 * history with per-id server retrieves.
 */
export interface SdkworkOrderMpWithdrawalService {
  createWithdrawalRequest(input: {
    /** Minor-unit amount string. */
    amount: string;
    currencyCode: string;
    payoutMethod?: string;
    payoutAccountRef?: string;
    reasonCode?: string;
  }): Promise<SdkworkOrderMpAccountValueRequest>;
  getWithdrawalRequest(withdrawalRequestId: string): Promise<SdkworkOrderMpAccountValueRequest>;
  listLocalHistory(): Promise<SdkworkOrderMpAccountValueRequest[]>;
}

export interface SdkworkOrderMpSessionStore {
  getToken(): string;
  setToken(token: string): void;
  clearToken(): void;
  isLoggedIn(): boolean;
}

/** Composed facade the bootstrap owns; pages consume the domain services. */
export interface SdkworkOrderMpClient {
  session: SdkworkOrderMpSessionStore;
  orders: SdkworkOrderMpOrderService;
  recharges: SdkworkOrderMpRechargeService;
  withdrawals: SdkworkOrderMpWithdrawalService;
}
