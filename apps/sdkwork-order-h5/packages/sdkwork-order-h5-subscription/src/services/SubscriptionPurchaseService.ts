import { createClient as createOrderAppClient, type SdkworkAppClient as SdkworkOrderAppClient, type SdkworkAppConfig } from "@sdkwork/order-app-sdk";
import {
  createSdkworkIdempotencyParams,
  createSdkworkOrderAppService,
  unwrapSdkworkOrderResource,
  type SdkworkMembershipCheckoutPayment,
  type SdkworkOrderAppService,
} from "@sdkwork/order-service";

import {
  createDefaultSubscriptionCatalogPort,
  type MembershipPackage,
  type MembershipPackageGroup,
  type SubscriptionCatalogPort,
  type TokenBankPlan,
} from "./SubscriptionCatalogPort";

export interface SubscriptionPurchasePort {
  listMembershipPackages(): Promise<MembershipPackage[]>;
  listMembershipPackageGroups(): Promise<MembershipPackageGroup[]>;
  listTokenBankPlans(): Promise<TokenBankPlan[]>;
  createRechargeOrder(
    planCode: string,
    paymentMethod?: string,
    /** Stable within one purchase attempt so retries dedupe server-side. */
    idempotencyKey?: string,
  ): Promise<TokenBankPayment>;
  getRechargeStatus(orderId: string): Promise<TokenBankPayment>;
  createSubscriptionOrder(
    packageId: string,
    paymentMethod?: string,
    /** Stable within one purchase attempt so retries dedupe server-side. */
    idempotencyKey?: string,
  ): Promise<SdkworkMembershipCheckoutPayment>;
  getSubscriptionStatus(orderId: string): Promise<SdkworkMembershipCheckoutPayment>;
  /** 兑换优惠券：输入码 → Token Bank 额度 / 会员权益等。 */
  redeemCoupon(code: string): Promise<CouponRedemptionResult>;
}

/** Coupon redemption result (benefit-shaped contract). */
export interface CouponRedemptionResult {
  orderId: string;
  orderNo?: string;
  replayed?: boolean;
  status: "completed" | "pending";
  benefitKind?: string;
  grantAmount?: string;
  durationDays?: number;
  [key: string]: unknown;
}

/** Token Bank recharge payment result (order-bound cashier contract). */
export interface TokenBankPayment {
  orderId: string;
  orderNo?: string;
  status: string;
  cashierUrl?: string;
  qrCode?: string;
  expiresAt?: string;
  [key: string]: unknown;
}

export interface CreateSubscriptionPurchaseServiceOptions {
  appService?: SdkworkOrderAppService;
  catalog?: SubscriptionCatalogPort;
  appConfig?: Partial<SdkworkAppConfig>;
  orderAppSdkClient?: SdkworkOrderAppClient;
}

function resolveAppService(options: CreateSubscriptionPurchaseServiceOptions): SdkworkOrderAppService {
  if (options.appService) {
    return options.appService;
  }
  const client = options.orderAppSdkClient ?? createOrderAppClient({
    baseUrl: options.appConfig?.baseUrl ?? "/",
    tokenManager: options.appConfig?.tokenManager,
    accessToken: options.appConfig?.accessToken,
    authToken: options.appConfig?.authToken,
    tenantId: options.appConfig?.tenantId,
    organizationId: options.appConfig?.organizationId,
    platform: "h5",
    authMode: options.appConfig?.authMode ?? "dual-token",
  } as SdkworkAppConfig);
  return createSdkworkOrderAppService({ appClient: client as unknown as SdkworkOrderAppClient });
}

/**
 * Order-domain subscription purchase service: Token Bank recharge and
 * membership subscription checkout share the order-service factories, so
 * both flows use one idempotent, single-flight implementation.
 *
 * Token Bank recharge is a real `token_bank_recharge` account-value order
 * (subject + targetAsset + planCode + grantAmount); the points recharge
 * service is intentionally not reused here because it settles to the points
 * asset, not the Token Bank.
 */
export function createSubscriptionPurchaseService(
  options: CreateSubscriptionPurchaseServiceOptions = {},
): SubscriptionPurchasePort {
  const appService = resolveAppService(options);
  const catalog = options.catalog ?? createDefaultSubscriptionCatalogPort({
    orderAppSdkClient: options.orderAppSdkClient,
  });

  return {
    listMembershipPackages: () => catalog.listMembershipPackages(),
    listMembershipPackageGroups: () => catalog.listMembershipPackageGroups(),
    listTokenBankPlans: () => catalog.listTokenBankPlans(),
    createRechargeOrder: async (planCode, paymentMethod, idempotencyKey) => {
      const plans = await catalog.listTokenBankPlans();
      const plan = plans.find((item) => item.planCode === planCode);
      if (!plan) {
        throw new Error("The selected Token Bank plan is unavailable.");
      }
      const params = createSdkworkIdempotencyParams(idempotencyKey);
      const response = await appService.recharges.orders.create(
        {
          subject: "points_recharge",
          targetAsset: "points",
          packageId: plan.planCode,
          amount: plan.priceAmount,
          currencyCode: plan.currencyCode,
          paymentMethod: paymentMethod ?? "wechat_pay",
          paymentProduct: "mobile_cashier_h5",
          source: "h5-token-bank",
        },
        params,
      );
      return normalizeTokenBankPayment(response);
    },
    getRechargeStatus: async (orderId) => {
      const response = await appService.recharges.orders.retrieve(orderId);
      return normalizeTokenBankPayment(response);
    },
    createSubscriptionOrder: async (packageId, paymentMethod, idempotencyKey) => {
      const params = createSdkworkIdempotencyParams(idempotencyKey);
      const response = await appService.memberships.orders.create(
        {
          action: "purchase",
          packageId: String(Number(packageId)),
          paymentMethod: paymentMethod ?? "wechat_pay",
          paymentProduct: "mobile_cashier_h5",
        },
        params,
      );
      return normalizeMembershipCheckoutPayment(response);
    },
    getSubscriptionStatus: async (orderId) => {
      const response = await appService.orders.paymentSuccess.retrieve(orderId);
      return normalizeMembershipCheckoutPayment(response);
    },
    redeemCoupon: async (code) => {
      const couponCode = code.trim();
      if (!couponCode) {
        throw new Error("A coupon code is required.");
      }
      const params = createSdkworkIdempotencyParams();
      const response = await appService.orders.couponRedemptions.create(
        { couponCode },
        params,
      );
      return normalizeCouponRedemptionResult(response);
    },
  };
}

function normalizeCouponRedemptionResult(response: unknown): CouponRedemptionResult {
  // 共享信封解包:data.item 与已解包载荷二者兼容(与 withdraw 服务同因,
  // 裸读 {item} 包装层会把订单号/状态静默归空)。
  const record = unwrapSdkworkOrderResource<Record<string, unknown>>(
    response,
    "Coupon redemption response is invalid.",
  );
  const source = (record ?? {}) as Record<string, unknown>;
  const benefit = source.benefit && typeof source.benefit === "object"
    ? (source.benefit as Record<string, unknown>)
    : {};
  const rawStatus = String(source.status ?? source.orderStatus ?? "pending");
  const completed = rawStatus === "completed" || rawStatus === "succeeded";
  return {
    ...source,
    // 归一化字段放在展开之后:原始 status("succeeded")等不得覆盖契约字段。
    orderId: String(source.orderId ?? source.id ?? ""),
    orderNo: source.orderNo != null ? String(source.orderNo) : undefined,
    replayed: source.replayed === true,
    status: completed ? "completed" : "pending",
    benefitKind: benefit.kind != null ? String(benefit.kind) : undefined,
    grantAmount: benefit.grantAmount != null || benefit.grantPoints != null
      ? String(benefit.grantAmount ?? benefit.grantPoints)
      : undefined,
    durationDays: benefit.durationDays != null ? Number(benefit.durationDays) : undefined,
  };
}


function normalizeMembershipCheckoutPayment(response: unknown): SdkworkMembershipCheckoutPayment {
  const record = unwrapSdkworkOrderResource<Record<string, unknown>>(
    response,
    "Membership checkout response is invalid.",
  );
  const paid = record.paid === true;
  const rawStatus = String(record.status ?? "pending");
  return {
    amountCny: record.amount != null ? Number(record.amount) : null,
    durationDays: record.durationDays != null ? Number(record.durationDays) : null,
    orderId: String(record.orderId ?? record.id ?? ""),
    packageId: record.packageId != null ? Number(record.packageId) : null,
    status: paid || rawStatus === "completed"
      ? "completed"
      : rawStatus === "failed" || rawStatus === "cancelled"
        ? "failed"
        : "pending",
    ...(record.cashierUrl != null ? { cashierUrl: String(record.cashierUrl) } : {}),
    ...(record.qrCode != null ? { qrCode: String(record.qrCode) } : {}),
    ...(record.expiresAt != null ? { expiresAt: String(record.expiresAt) } : {}),
    ...(record.packageName != null ? { packageName: String(record.packageName) } : {}),
  };
}

function normalizeTokenBankPayment(response: unknown): TokenBankPayment {
  const record = unwrapSdkworkOrderResource<Record<string, unknown>>(
    response,
    "Token Bank payment response is invalid.",
  );
  return {
    orderId: String(record.orderId ?? record.id ?? ""),
    orderNo: record.orderNo != null ? String(record.orderNo) : undefined,
    status: String(record.status ?? "pending"),
    cashierUrl: record.cashierUrl != null ? String(record.cashierUrl) : undefined,
    ...record,
  };
}
