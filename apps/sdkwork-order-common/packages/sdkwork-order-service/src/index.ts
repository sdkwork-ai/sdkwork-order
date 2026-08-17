import type { SdkworkAppClient } from "@sdkwork/order-app-sdk";
import { formatMoney } from "@sdkwork/utils/money";
import {
  createOrderAppTransportClient,
  type BootstrapSdkworkOrderAppServiceInput,
} from "./transport.ts";
import { createSdkworkIdempotencyParams } from "./idempotency.ts";

type PublicSdkPort<T> = {
  readonly [TKey in keyof T]: T[TKey] extends (...args: infer TArgs) => infer TResult
    ? (...args: TArgs) => TResult
    : T[TKey] extends object
      ? PublicSdkPort<T[TKey]>
      : T[TKey];
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

export type SdkworkCouponRedemptionResult =
  | SdkworkCouponTokenBankRedemptionResult
  | SdkworkCouponPointsRedemptionResult
  | SdkworkCouponCashRedemptionResult
  | SdkworkCouponSubscriptionRedemptionResult;

export type SdkworkCouponRechargeResult = SdkworkCouponRedemptionResult;

export interface SdkworkCouponRedemptionService {
  redeem(code: string): Promise<SdkworkCouponRedemptionResult>;
}

export type SdkworkCouponRechargeService = SdkworkCouponRedemptionService;

export interface CreateSdkworkCouponRechargeServiceOptions {
  appService?: SdkworkOrderAppService;
}

export type SdkworkOrderAppServiceProvider = () => SdkworkOrderAppService;

let sdkworkOrderAppServiceProvider: SdkworkOrderAppServiceProvider | null = null;

export interface SdkworkOrderSessionTokens {
  accessToken?: string;
  authToken?: string;
  refreshToken?: string;
}

export type SdkworkOrderSessionTokenProvider = () => SdkworkOrderSessionTokens;

let sdkworkOrderSessionTokenProvider: SdkworkOrderSessionTokenProvider = () => ({});

export interface CreateSdkworkOrderAppServiceInput {
  appClient: PublicSdkPort<SdkworkAppClient>;
}

export interface SdkworkOrderResponseEnvelope<T> {
  code?: number | string;
  data?: T;
  message?: string;
  msg?: string;
}

export type SdkworkMediaKind =
  | "archive"
  | "audio"
  | "document"
  | "image"
  | "model"
  | "other"
  | "video";

export type SdkworkMediaSource =
  | "data_url"
  | "external_url"
  | "generated"
  | "object_storage"
  | "provider_asset";

export interface SdkworkMediaResource {
  kind: SdkworkMediaKind;
  publicUrl?: string;
  source: SdkworkMediaSource;
  url?: string;
  [key: string]: unknown;
}

export function configureSdkworkOrderAppServiceProvider(provider: SdkworkOrderAppServiceProvider | null): void {
  sdkworkOrderAppServiceProvider = provider;
}

export function configureSdkworkOrderSessionTokenProvider(provider: SdkworkOrderSessionTokenProvider | null): void {
  sdkworkOrderSessionTokenProvider = provider ?? (() => ({}));
}

export function getSdkworkOrderService(): SdkworkOrderAppService {
  if (!sdkworkOrderAppServiceProvider) {
    throw new Error(
      "SDKWork order service provider is not configured. Call configureSdkworkOrderAppServiceProvider() from order PC bootstrap.",
    );
  }
  return sdkworkOrderAppServiceProvider();
}

export function getSdkworkOrderSessionTokens(): SdkworkOrderSessionTokens {
  const tokens = sdkworkOrderSessionTokenProvider();
  const accessToken = normalizeSessionToken(tokens.accessToken);
  const authToken = normalizeSessionToken(tokens.authToken);
  const refreshToken = normalizeSessionToken(tokens.refreshToken);
  return {
    ...(accessToken === undefined ? {} : { accessToken }),
    ...(authToken === undefined ? {} : { authToken }),
    ...(refreshToken === undefined ? {} : { refreshToken }),
  };
}

export function hasSdkworkOrderSession(): boolean {
  const tokens = getSdkworkOrderSessionTokens();
  return Boolean(normalizeSessionToken(tokens.authToken) || normalizeSessionToken(tokens.accessToken));
}

export function requireSdkworkOrderSession(message = "Authentication required"): void {
  if (!hasSdkworkOrderSession()) {
    throw new Error(message);
  }
}

export function createSdkworkOrderAppService(input: CreateSdkworkOrderAppServiceInput): SdkworkOrderAppService {
  return {
    checkout: input.appClient.checkout,
    memberships: input.appClient.memberships,
    orders: input.appClient.orders,
    recharges: input.appClient.recharges,
    withdrawals: input.appClient.withdrawals,
  };
}

export function createSdkworkPhysicalPurchaseService(
  options: CreateSdkworkPhysicalPurchaseServiceOptions = {},
): SdkworkPhysicalPurchaseService {
  const resolveAppService = () => options.appService ?? getSdkworkOrderService();

  return {
    async prepareCheckout(input) {
      requireSdkworkOrderSession();
      const items = input.items.map((item) => ({
        quantity: normalizePhysicalQuantity(item.quantity),
        skuId: requirePhysicalText("SKU", item.skuId),
      }));
      if (items.length === 0) {
        throw new Error("At least one physical SKU is required.");
      }
      if (new Set(items.map((item) => item.skuId)).size !== items.length) {
        throw new Error("Duplicate physical SKU lines are not allowed.");
      }
      const shippingAddress = normalizePhysicalShippingAddress(input.shippingAddress);
      const session = await resolveAppService().checkout.sessions.create(
        {
          currencyCode: (input.currencyCode ?? "CNY").trim().toUpperCase(),
          items: items.map((item) => ({
            quantity: String(item.quantity),
            skuId: item.skuId,
          })),
          shippingAddress,
        },
        createSdkworkIdempotencyParams(),
      );
      const checkoutSessionId = requirePhysicalText(
        "checkout session id",
        session.checkoutSessionId,
      );
      const quote = await resolveAppService().checkout.sessions.quotes.create(
        checkoutSessionId,
        createSdkworkIdempotencyParams(),
      );
      return {
        checkoutSessionId,
        currencyCode: quote.currencyCode,
        discountAmount: quote.discountAmount,
        originalAmount: quote.originalAmount,
        payableAmount: quote.payableAmount,
        quoteId: quote.quoteId,
        status: session.status,
      };
    },

    async placeOrder(checkoutSessionId) {
      requireSdkworkOrderSession();
      const normalizedSessionId = requirePhysicalText(
        "checkout session id",
        checkoutSessionId,
      );
      return resolveAppService().checkout.sessions.orders.create(
        normalizedSessionId,
        createSdkworkIdempotencyParams(),
      );
    },
  };
}

export type CreateSdkworkCouponRedemptionServiceOptions = CreateSdkworkCouponRechargeServiceOptions;

export function createSdkworkPointsRechargeService(
  options: CreateSdkworkPointsRechargeServiceOptions = {},
): SdkworkPointsRechargeService {
  const resolveAppService = () => options.appService ?? getSdkworkOrderService();

  return {
    async listPackages() {
      const response = await resolveAppService().recharges.packages.list({ page: 1, pageSize: 200 });
      const page = unwrapSdkworkOrderListPage<unknown>(response, "Unable to load recharge packages.");
      return page.items.map(normalizePointsRechargePackage).filter((item): item is SdkworkPointsRechargePackage => item !== null);
    },

    async createOrder(input) {
      const packageId = String(input.packageId).trim();
      if (!packageId) {
        throw new Error("A recharge package is required.");
      }
      const packages = await this.listPackages();
      const selectedPackage = packages.find((item) => item.id === packageId);
      if (!selectedPackage) {
        throw new Error("The selected recharge package is unavailable.");
      }

      const paymentProduct = input.paymentProduct ?? "mobile_cashier_h5";
      const body = {
        amount: selectedPackage.priceAmount,
        currencyCode: selectedPackage.currencyCode,
        packageId: selectedPackage.id,
        paymentMethod: input.paymentMethod
          ?? (paymentProduct === "alipay_native" ? "alipay" : "wechat_pay"),
        paymentProduct,
        source: input.source ?? "membership-token-plan",
        subject: "points_recharge" as const,
        targetAsset: "points" as const,
      };
      const params = createSdkworkIdempotencyParams();
      const response = await resolveAppService().recharges.orders.create(body, params);
      return normalizePointsRechargePayment(
        unwrapSdkworkOrderResource<unknown>(response, "Unable to create points recharge order."),
      );
    },

    async getOrderStatus(orderId) {
      const normalizedOrderId = orderId.trim();
      if (!normalizedOrderId) {
        throw new Error("A recharge order id is required.");
      }
      const response = await resolveAppService().recharges.orders.retrieve(normalizedOrderId);
      return normalizePointsRechargePayment(
        unwrapSdkworkOrderResource<unknown>(response, "Unable to retrieve points recharge order."),
      );
    },
  };
}

export function createSdkworkCouponRechargeService(
  options: CreateSdkworkCouponRechargeServiceOptions = {},
): SdkworkCouponRechargeService {
  const resolveAppService = () => options.appService ?? getSdkworkOrderService();

  return {
    async redeem(code) {
      requireSdkworkOrderSession();
      const couponCode = code.trim();
      if (!couponCode) {
        throw new Error("A coupon code is required.");
      }
      const params = createSdkworkIdempotencyParams();
      const response = await resolveAppService().orders.couponRedemptions.create({ couponCode }, params);
      return normalizeCouponRechargeResult(
        unwrapSdkworkOrderResource<unknown>(response, "Unable to redeem this coupon."),
      );
    },
  };
}

export function createSdkworkMembershipCheckoutService(
  options: CreateSdkworkMembershipCheckoutServiceOptions = {},
): SdkworkMembershipCheckoutService {
  const resolveAppService = () => options.appService ?? getSdkworkOrderService();
  const inFlightCheckouts = new Map<string, Promise<SdkworkMembershipCheckoutPayment>>();

  return {
    createCheckout(input) {
      requireSdkworkOrderSession();
      const isRecharge = input.action === "recharge";
      const grantQuantity = input.grantQuantity ?? 0;
      const amount = input.amountCny?.trim() ?? "";
      // 订阅期额度充值：数量与金额必填，不依赖目录套餐
      if (isRecharge) {
        if (grantQuantity <= 0 || !amount || Number.isNaN(Number(amount)) || Number(amount) <= 0) {
          throw new Error("Membership quota recharge requires a positive grantQuantity and amount.");
        }
      }
      const packageId = String(input.packageId).trim();
      if (!isRecharge && (!packageId || input.packageId <= 0)) {
        throw new Error("A valid membership package is required.");
      }

      const paymentProduct = input.paymentProduct ?? "mobile_cashier_h5";
      const paymentMethod = normalizeMembershipPaymentMethod(input.paymentMethod, paymentProduct);
      const singleFlightKey = [input.action, packageId, paymentMethod, paymentProduct, input.grantQuantity ?? "", input.amountCny ?? ""].join(":");
      const existing = inFlightCheckouts.get(singleFlightKey);
      if (existing) {
        return existing;
      }
      const body = {
        action: input.action,
        packageId,
        paymentMethod,
        paymentProduct,
        ...(isRecharge
          ? { grantQuantity: String(grantQuantity), amount }
          : {}),
      };
      const checkout = (async () => {
        const params = createSdkworkIdempotencyParams();
        const response = await resolveAppService().memberships.orders.create(body, params);
        return normalizeMembershipCheckoutPayment(
          unwrapSdkworkOrderResource<unknown>(response, "Unable to create membership order."),
          input.packageId,
          paymentProduct,
        );
      })();
      inFlightCheckouts.set(singleFlightKey, checkout);
      void checkout.then(
        () => inFlightCheckouts.delete(singleFlightKey),
        () => inFlightCheckouts.delete(singleFlightKey),
      );
      return checkout;
    },

    async getCheckoutStatus(orderId) {
      requireSdkworkOrderSession();
      const normalizedOrderId = orderId.trim();
      if (!normalizedOrderId) {
        throw new Error("A membership order id is required.");
      }
      const response = await resolveAppService().orders.paymentSuccess.retrieve(normalizedOrderId);
      const record = unwrapSdkworkOrderResource<Record<string, unknown>>(
        response,
        "Unable to retrieve membership order status.",
      );
      return {
        amountCny: null,
        durationDays: null,
        orderId: normalizedOrderId,
        packageId: null,
        status: record?.paid === true ? "completed" : normalizePointsRechargeStatus(record?.status),
      };
    },
  };
}

export function unwrapSdkworkOrderResource<T>(
  value: unknown,
  fallbackMessage = "Request failed.",
): T {
  const data = unwrapSdkworkOrderResponse<{ item?: T } | T>(value, fallbackMessage);
  if (data && typeof data === "object" && "item" in (data as Record<string, unknown>)) {
    return (data as { item?: T }).item as T;
  }
  return data as T;
}

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

export function unwrapSdkworkOrderPage<T>(
  value: unknown,
  fallbackMessage = "Request failed.",
): T[] {
  return unwrapSdkworkOrderListPage<T>(value, fallbackMessage).items;
}

export function unwrapSdkworkOrderListPage<T>(
  value: unknown,
  fallbackMessage = "Request failed.",
): SdkworkOffsetListPage<T> {
  const data = unwrapSdkworkOrderResponse<SdkworkOffsetListPage<T> | T[]>(value, fallbackMessage);
  if (Array.isArray(data)) {
    return { items: data };
  }
  return {
    items: Array.isArray(data?.items) ? data.items : [],
    ...(data?.pageInfo === undefined ? {} : { pageInfo: data.pageInfo }),
  };
}

export function resolveSdkworkOffsetPagination(
  pageInfo: SdkworkOffsetPageInfo | null | undefined,
  fallbackPage: number,
  fallbackPageSize: number,
): {
  hasMore: boolean;
  page: number;
  pageSize: number;
  total: number;
  totalPages: number;
} {
  const pageSize = toSdkworkOrderNumber(pageInfo?.pageSize, fallbackPageSize) || fallbackPageSize;
  const page = toSdkworkOrderNumber(pageInfo?.page, fallbackPage) || fallbackPage;
  const total = toSdkworkOrderNumber(pageInfo?.totalItems);
  const totalPages = pageInfo?.totalPages === undefined
    ? (pageSize > 0 ? Math.ceil(total / pageSize) : 0)
    : toSdkworkOrderNumber(pageInfo?.totalPages);
  return {
    page,
    pageSize,
    total,
    hasMore: Boolean(pageInfo?.hasMore ?? page * pageSize < total),
    totalPages,
  };
}

/** Maps UI kebab-case order status filters to API snake_case wire values. */
export function toApiOrderStatusWire(status: string): string {
  const normalized = status.trim();
  if (!normalized || normalized === "all") {
    return normalized;
  }
  return normalized.replace(/-/g, "_").toLowerCase();
}

export function unwrapSdkworkOrderResponse<T>(value: unknown, fallbackMessage = "Request failed."): T {
  if (!value || typeof value !== "object") {
    return value as T;
  }
  if (!("data" in value) && !("code" in value)) {
    return value as T;
  }
  const envelope = value as SdkworkOrderResponseEnvelope<T>;
  if (!isSuccessCode(envelope.code)) {
    throw new Error(String(envelope.message || envelope.msg || fallbackMessage).trim());
  }
  return (envelope.data ?? null) as T;
}

export function toSdkworkOrderOptionalString(value: unknown): string | undefined {
  const normalized = typeof value === "string" ? value.trim() : String(value ?? "").trim();
  return normalized || undefined;
}

export function toNullableSdkworkOrderNumber(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === "string" && value.trim()) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

export function toSdkworkOrderNumber(value: unknown, fallback = 0): number {
  return toNullableSdkworkOrderNumber(value) ?? fallback;
}

export function formatSdkworkOrderCurrencyCny(value: number | null | undefined, language = "en-US"): string {
  return formatMoney(value, { currency: "CNY", locale: language, mode: "symbol" }) ?? "--";
}

export function readSdkworkMediaResource(value: unknown): SdkworkMediaResource | undefined {
  if (!value || typeof value !== "object") {
    return undefined;
  }
  const record = value as Record<string, unknown>;
  const kind = typeof record.kind === "string" ? record.kind : undefined;
  const source = typeof record.source === "string" ? record.source : undefined;
  if (!kind || !source) {
    return undefined;
  }
  return { ...record, kind, source } as SdkworkMediaResource;
}

function normalizePointsRechargePackage(value: unknown): SdkworkPointsRechargePackage | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const record = value as Record<string, unknown>;
  const id = toSdkworkOrderOptionalString(record.id ?? record.packageId);
  if (!id) {
    return null;
  }
  return {
    id,
    priceAmount: toSdkworkOrderNumber(record.priceAmount ?? record.price),
    currencyCode: toSdkworkOrderOptionalString(record.currencyCode) ?? "CNY",
    bonusPoints: toSdkworkOrderNumber(record.bonusPoints),
    grantAmount: toSdkworkOrderNumber(record.grantAmount),
    points: toSdkworkOrderNumber(record.points ?? record.grantAmount ?? record.bonusPoints),
  };
}

function normalizePointsRechargePayment(value: unknown): SdkworkPointsRechargePayment {
  const record = value && typeof value === "object" ? value as Record<string, unknown> : {};
  const status = normalizePointsRechargeStatus(
    record.status ?? record.rechargeStatus ?? record.paymentStatus ?? record.orderStatus,
  );
  const cashierUrl = toSdkworkOrderOptionalString(record.cashierUrl);
  const paymentProduct = toSdkworkOrderOptionalString(record.paymentProduct);
  const providerQrCode = toSdkworkOrderOptionalString(
    record.qrCode ?? record.qrCodePayload ?? record.providerQrCode,
  );
  const expiresAt = toSdkworkOrderOptionalString(record.expiresAt);
  const orderId = toSdkworkOrderOptionalString(record.orderId ?? record.id);
  const orderNo = toSdkworkOrderOptionalString(record.orderNo ?? record.outTradeNo);
  const qrCode = paymentProduct === "mobile_cashier_h5" ? cashierUrl : providerQrCode ?? cashierUrl;
  return {
    amountCny: toNullableSdkworkOrderNumber(record.amountCny ?? record.amount),
    ...(cashierUrl === undefined ? {} : { cashierUrl }),
    ...(expiresAt === undefined ? {} : { expiresAt }),
    ...(orderId === undefined ? {} : { orderId }),
    ...(orderNo === undefined ? {} : { orderNo }),
    points: toSdkworkOrderNumber(record.points ?? record.grantAmount),
    ...(qrCode === undefined ? {} : { qrCode }),
    status,
  };
}

export const createSdkworkCouponRedemptionService = createSdkworkCouponRechargeService;

function normalizeCouponRechargeResult(value: unknown): SdkworkCouponRedemptionResult {
  const record = value && typeof value === "object" ? value as Record<string, unknown> : {};
  const orderId = toSdkworkOrderOptionalString(record.orderId ?? record.id);
  if (!orderId) {
    throw new Error("Coupon redemption did not return an order id.");
  }
  const status = normalizePointsRechargeStatus(
    record.status ?? record.fulfillmentStatus ?? record.orderStatus,
  );
  const orderNo = toSdkworkOrderOptionalString(record.orderNo ?? record.outTradeNo);
  const common = {
    orderId,
    ...(orderNo === undefined ? {} : { orderNo }),
    replayed: record.replayed === true,
    status: status === "completed" ? "completed" as const : "pending" as const,
  };
  const benefit = record.benefit && typeof record.benefit === "object"
    ? record.benefit as Record<string, unknown>
    : {};
  if (benefit.kind === "token_bank_credit") {
    const grantAmount = toSdkworkOrderNumber(benefit.grantAmount);
    if (grantAmount <= 0) {
      throw new Error("Coupon redemption did not return a Token Bank grant.");
    }
    return {
      ...common,
      benefitKind: "token_bank_credit",
      grantAmount,
      targetAsset: "token_bank",
    };
  }
  if (benefit.kind === "points_credit") {
    const grantPoints = toSdkworkOrderNumber(benefit.grantPoints);
    if (grantPoints <= 0) {
      throw new Error("Coupon redemption did not return a points grant.");
    }
    return {
      ...common,
      benefitKind: "points_credit",
      grantPoints,
    };
  }
  if (benefit.kind === "cash_credit") {
    const grantAmount = toSdkworkOrderNumber(benefit.grantAmount);
    if (grantAmount <= 0) {
      throw new Error("Coupon redemption did not return a cash grant.");
    }
    return {
      ...common,
      benefitKind: "cash_credit",
      grantAmount,
    };
  }
  if (benefit.kind === "subscription") {
    const productId = toSdkworkOrderOptionalString(benefit.productId);
    const skuId = toSdkworkOrderOptionalString(benefit.skuId);
    const packageId = toSdkworkOrderOptionalString(benefit.packageId);
    const subscriptionId = toSdkworkOrderOptionalString(benefit.subscriptionId);
    const startsAt = toSdkworkOrderOptionalString(benefit.startsAt);
    const expiresAt = toSdkworkOrderOptionalString(benefit.expiresAt);
    const period = toSdkworkOrderOptionalString(benefit.period) as SdkworkCouponSubscriptionPeriod | undefined;
    const durationDays = toSdkworkOrderNumber(benefit.durationDays);
    const dailyQuota = toSdkworkOrderNumber(benefit.dailyQuota);
    const totalQuota = toSdkworkOrderNumber(benefit.totalQuota);
    if (!productId || !skuId || !packageId || !subscriptionId || !startsAt || !expiresAt
      || !period || !["day", "week", "month", "year"].includes(period)
      || durationDays <= 0 || dailyQuota <= 0 || totalQuota < dailyQuota) {
      throw new Error("Coupon redemption returned an invalid subscription benefit.");
    }
    return {
      ...common,
      benefitKind: "subscription",
      dailyQuota,
      durationDays,
      expiresAt,
      packageId,
      period,
      productId,
      skuId,
      startsAt,
      subscriptionId,
      totalQuota,
    };
  }
  throw new Error("Coupon redemption returned an unsupported benefit.");
}

function normalizeMembershipCheckoutPayment(
  value: unknown,
  fallbackPackageId: number,
  paymentProduct: SdkworkMembershipCheckoutInput["paymentProduct"],
): SdkworkMembershipCheckoutPayment {
  const record = value && typeof value === "object" ? value as Record<string, unknown> : {};
  const paymentParams = record.paymentParams && typeof record.paymentParams === "object"
    ? record.paymentParams as Record<string, unknown>
    : {};
  const cashierUrl = toSdkworkOrderOptionalString(record.cashierUrl ?? paymentParams.cashierUrl);
  const providerQrCode = toSdkworkOrderOptionalString(
    paymentParams.qrCodeUrl
      ?? paymentParams.qrCode
      ?? paymentParams.qrCodePayload
      ?? paymentParams.codeUrl
      ?? record.qrCode
      ?? record.qrCodePayload
      ?? record.codeUrl,
  );
  const action = toSdkworkOrderOptionalString(record.action) as SdkworkMembershipCheckoutAction | undefined;
  const expiresAt = toSdkworkOrderOptionalString(record.expiresAt);
  const orderId = toSdkworkOrderOptionalString(record.orderId ?? record.id);
  const packageName = toSdkworkOrderOptionalString(record.packageName);
  const qrCode = paymentProduct === "mobile_cashier_h5" ? cashierUrl : providerQrCode ?? cashierUrl;
  const targetLevelName = toSdkworkOrderOptionalString(record.targetLevelName ?? record.targetPlanName);
  return {
    ...(action === undefined ? {} : { action }),
    amountCny: toNullableSdkworkOrderNumber(record.amountCny ?? record.amount),
    ...(cashierUrl === undefined ? {} : { cashierUrl }),
    durationDays: toNullableSdkworkOrderNumber(record.durationDays),
    ...(expiresAt === undefined ? {} : { expiresAt }),
    ...(orderId === undefined ? {} : { orderId }),
    packageId: toNullableSdkworkOrderNumber(record.packageId) ?? fallbackPackageId,
    ...(packageName === undefined ? {} : { packageName }),
    ...(qrCode === undefined ? {} : { qrCode }),
    reused: record.reused === true,
    status: normalizePointsRechargeStatus(record.status ?? record.paymentStatus ?? record.orderStatus),
    ...(targetLevelName === undefined ? {} : { targetLevelName }),
  };
}

function normalizeMembershipPaymentMethod(
  value: string | undefined,
  paymentProduct: NonNullable<SdkworkMembershipCheckoutInput["paymentProduct"]>,
): string {
  const normalized = value?.trim().toLowerCase().replace(/-/gu, "_");
  if (normalized) {
    return normalized === "wechat" ? "wechat_pay" : normalized;
  }
  return paymentProduct === "alipay_native" ? "alipay" : "wechat_pay";
}

function normalizePhysicalQuantity(value: number): number {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error("Physical SKU quantity must be a positive integer.");
  }
  return value;
}

function normalizePhysicalShippingAddress(
  address: SdkworkPhysicalShippingAddress,
): SdkworkPhysicalShippingAddress {
  const district = toSdkworkOrderOptionalString(address.district);
  const postalCode = toSdkworkOrderOptionalString(address.postalCode);
  return {
    city: requirePhysicalText("shipping city", address.city),
    countryCode: requirePhysicalText("shipping country code", address.countryCode).toUpperCase(),
    detailAddress: requirePhysicalText("shipping detail address", address.detailAddress),
    ...(district === undefined ? {} : { district }),
    ...(postalCode === undefined ? {} : { postalCode }),
    province: requirePhysicalText("shipping province", address.province),
    receiverName: requirePhysicalText("shipping receiver name", address.receiverName),
    receiverPhone: requirePhysicalText("shipping receiver phone", address.receiverPhone),
  };
}

function requirePhysicalText(field: string, value: string): string {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`A valid ${field} is required.`);
  }
  return normalized;
}

function normalizePointsRechargeStatus(value: unknown): SdkworkPointsRechargePayment["status"] {
  const status = String(value ?? "").trim().toLowerCase();
  if (["completed", "complete", "paid", "success", "succeeded", "fulfilled"].includes(status)) {
    return "completed";
  }
  if (["failed", "cancelled", "canceled", "closed", "expired", "rejected"].includes(status)) {
    return "failed";
  }
  return "pending";
}

function normalizeSessionToken(value: unknown): string | undefined {
  const normalized = typeof value === "string" ? value.trim() : "";
  return normalized || undefined;
}

function isSuccessCode(code: number | string | undefined): boolean {
  if (code === undefined || code === null || code === "") {
    return true;
  }
  if (typeof code === "number") {
    return code === 0;
  }
  return String(code).trim() === "0";
}

export function bootstrapSdkworkOrderAppService(
  input: BootstrapSdkworkOrderAppServiceInput,
): SdkworkOrderAppService {
  const transport = createOrderAppTransportClient(input);
  const service = createSdkworkOrderAppService({
    appClient: transport,
  });
  configureSdkworkOrderAppServiceProvider(() => service);
  configureSdkworkOrderSessionTokenProvider(() => {
    if (input.tokenManager) {
      return input.tokenManager.getTokens();
    }
    return {
      ...(input.accessToken === undefined ? {} : { accessToken: input.accessToken }),
      ...(input.authToken === undefined ? {} : { authToken: input.authToken }),
    };
  });
  return service;
}

export {
  createOrderAppTransportClient,
  resolveOrderAppApiOrigin,
  type BootstrapSdkworkOrderAppServiceInput,
} from "./transport.ts";

export {
  bootstrapSdkworkOrderBackendSdk,
  createOrderBackendTransportClient,
  getSdkworkOrderBackendSdkClient,
  resetSdkworkOrderBackendSdkClient,
  resolveOrderBackendApiOrigin,
  type BootstrapSdkworkOrderBackendSdkInput,
} from "./backend-transport.ts";

export {
  createSdkworkIdempotencyParams,
  type SdkworkIdempotencyParams,
} from "./idempotency.ts";
