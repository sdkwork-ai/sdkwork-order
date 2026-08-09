import { uuid } from "@sdkwork/utils";
import { formatMoneyMinorUnits } from "@sdkwork/utils/money";
import type { SdkworkAppClient } from "@sdkwork/order-app-sdk";

import { setPaymentRegionOverride, type PaymentEnvironment, type PaymentRegion } from "./PaymentEnvironment";
import type { WechatPaymentOAuthChannel } from "./WechatPaymentOAuth";

/**
 * Canonical mobile order service backed by the generated Order App SDK.
 *
 * The SDK client is injected once by the H5 host through
 * `configureOrderMobileRuntime()` (same runtime-injection pattern as
 * `CloudDriveService`). UI code must never construct raw HTTP or SDK
 * clients; it consumes this service through the composed runtime.
 */

export type OrderStatus =
  | "pending_payment"
  | "paid"
  | "fulfilled"
  | "completed"
  | "cancelled"
  | "expired"
  | "refunding"
  | "refunded";

export type OrderPaymentMethod =
  | "wechat_pay"
  | "wechat_jsapi"
  | "alipay"
  | "alipay_wap"
  | "balance";

/**
 * Methods offered on the cashier UI (shown to the user). Environment-aware
 * cashiers narrow this list via `paymentMethodsForEnvironment`.
 */
export const ORDER_PAYMENT_METHODS: readonly OrderPaymentMethod[] = [
  "wechat_pay",
  "alipay",
  "balance",
];

/**
 * Overseas cashier defaults. The order backend only accepts the CN wire
 * methods today, so overseas deployments inherit them until their gateway
 * configures paypal/card providers; hosts may override the whole matrix
 * through `configureOrderMobileRuntime({ paymentMethodsForEnvironment })`.
 */
export const ORDER_PAYMENT_METHODS_OVERSEAS: readonly OrderPaymentMethod[] = [
  "wechat_pay",
  "alipay",
  "balance",
];

/**
 * Wire methods accepted by `POST /app/v3/api/orders/{orderId}/payments`.
 * `wechat_jsapi` (needs openid) and `alipay_wap` are environment-specific
 * launch methods, not user-facing choices.
 */
export const ORDER_PAYMENT_WIRE_METHODS: readonly OrderPaymentMethod[] = [
  ...ORDER_PAYMENT_METHODS,
  "wechat_jsapi",
  "alipay_wap",
];

/**
 * Narrow the cashier method list to the current payment environment and
 * deployment region:
 * - Alipay app webview: Alipay only (WAP redirect in-app).
 * - WeChat app webview: WeChat only (JSAPI via OAuth).
 * - Browser (CN deployment): WeChat/Alipay scan + balance.
 * - Browser (overseas deployment): the channels configured by the host
 *   deployment (injected through `configureOrderMobileRuntime`); defaults
 *   to the CN channels, which are the only wire methods the order backend
 *   accepts today.
 */
export function paymentMethodsForEnvironment(
  environment: PaymentEnvironment,
  region: PaymentRegion = "cn",
): readonly OrderPaymentMethod[] {
  switch (environment) {
    case "alipay":
      return ["alipay"];
    case "wechat":
      return ["wechat_pay"];
    default:
      return region === "overseas"
        ? ORDER_PAYMENT_METHODS_OVERSEAS
        : ORDER_PAYMENT_METHODS;
  }
}

export type OrderTabId =
  | "all"
  | "pending_payment"
  | "paid"
  | "fulfilled"
  | "completed"
  | "cancelled";

export interface OrderTab {
  readonly id: OrderTabId;
  readonly labelKey: string;
}

/** Order line item from the backend `OrderItemResponse` read model. */
export interface OrderItem {
  readonly id: string;
  readonly title: string;
  readonly quantity: number;
  /** Minor-unit amount string, e.g. "6990" for CNY 69.90. */
  readonly unitPrice: string;
  /** Minor-unit amount string. */
  readonly totalAmount: string;
}

/**
 * Order model aligned with the backend `OrderSummaryResponse` /
 * `OrderDetailResponse` wire format. Amounts are minor-unit strings.
 */
export interface Order {
  readonly id: string;
  readonly orderSn: string;
  readonly status: OrderStatus | string;
  readonly statusText: string;
  readonly subject: string;
  /** Minor-unit amount string. */
  readonly totalAmount: string;
  /** Minor-unit amount string, present once paid. */
  readonly paidAmount?: string;
  /** Minor-unit amount string. */
  readonly discountAmount?: string;
  readonly currencyCode: string;
  readonly quantity: number;
  readonly createdAt: string;
  readonly payTime?: string;
  readonly expireTime?: string;
  readonly paymentMethod?: string;
  readonly items: readonly OrderItem[];
  readonly outTradeNo?: string;
  readonly transactionId?: string;
}

/** Result of `POST /app/v3/api/orders/{orderId}/payments`. */
export interface PaymentSession {
  readonly amount: string;
  readonly orderId: string;
  readonly outTradeNo: string;
  readonly paymentId: string;
  readonly paymentMethod: string;
  readonly paymentParams: Readonly<Record<string, string>>;
}

/** Result of `GET /app/v3/api/orders/{orderId}/payment_success`. */
export interface PaymentStatus {
  readonly paid: boolean;
  readonly status: string;
  readonly statusName: string;
}

export interface OrderStatistics {
  readonly totalOrders: number;
  readonly pendingPayment: number;
  readonly pendingShipment: number;
  readonly pendingReceipt: number;
  readonly completed: number;
  readonly totalAmount: string;
}

export interface CreateOrderItem {
  readonly quantity: number;
  readonly skuId: string;
}

export interface CreateOrderShippingAddress {
  readonly receiverName: string;
  readonly receiverPhone: string;
  readonly countryCode: string;
  readonly province: string;
  readonly city: string;
  readonly district?: string;
  readonly detailAddress: string;
  readonly postalCode?: string;
}

export interface CreateOrderInput {
  readonly currencyCode?: string;
  readonly items: readonly CreateOrderItem[];
  readonly shippingAddress: CreateOrderShippingAddress;
}

export interface VoucherRedemptionResult {
  readonly success: boolean;
  readonly message: string;
  readonly orderId?: string;
  readonly orderNo?: string;
}

export interface OrderRuntime {
  readonly client: SdkworkAppClient;
  /**
   * Optional WeChat payment OAuth channel composed by the host. When absent,
   * WeChat JSAPI payment is unavailable inside the WeChat app.
   */
  readonly wechatPaymentOAuth?: WechatPaymentOAuthChannel;
  /**
   * Deployment region for the cashier method matrix. When absent, the
   * payer language selects the region (`zh*` → CN, otherwise overseas).
   */
  readonly paymentRegion?: PaymentRegion;
  /**
   * Optional per-deployment method matrix override (e.g. overseas hosts
   * injecting paypal/card channels). Defaults to `paymentMethodsForEnvironment`.
   */
  readonly paymentMethodsForEnvironment?: (
    environment: PaymentEnvironment,
    region: PaymentRegion,
  ) => readonly OrderPaymentMethod[];
}

export class OrderCapabilityUnavailableError extends Error {
  constructor() {
    super("Orders are unavailable because the Order owner SDK is not composed.");
    this.name = "OrderCapabilityUnavailableError";
  }
}

let runtime: OrderRuntime | null = null;

function requireOrderRuntime(): OrderRuntime {
  if (!runtime) {
    throw new OrderCapabilityUnavailableError();
  }
  return runtime;
}

export function configureOrderMobileRuntime(nextRuntime: OrderRuntime): void {
  runtime = nextRuntime;
  setPaymentRegionOverride(nextRuntime.paymentRegion ?? null);
}

export function resetOrderMobileRuntime(): void {
  runtime = null;
  setPaymentRegionOverride(null);
}

export function getOrderMobileRuntime(): OrderRuntime | null {
  return runtime;
}

/**
 * Resolves the cashier method list for the current environment and region,
 * honouring the host-injected matrix override from the runtime.
 */
export function resolveAvailablePaymentMethods(
  environment: PaymentEnvironment,
  region: PaymentRegion,
): readonly OrderPaymentMethod[] {
  const composed = requireOrderRuntimeOrNull();
  const resolver = composed?.paymentMethodsForEnvironment;
  if (resolver) {
    return resolver(environment, region);
  }
  return paymentMethodsForEnvironment(environment, region);
}

function requireOrderRuntimeOrNull(): OrderRuntime | null {
  return runtime;
}

/* ------------------------------------------------------------------ */
/* Wire helpers                                                       */
/* ------------------------------------------------------------------ */

function toOptionalString(value: unknown): string | undefined {
  if (typeof value !== "string") {
    return undefined;
  }
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

function toNumber(value: unknown, fallback = 0): number {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === "string" && value.trim()) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
  }
  return fallback;
}

/**
 * Maps an API status wire value onto the mobile `OrderStatus` union.
 * Unknown values pass through unchanged so future backend statuses keep
 * rendering with the backend-provided `status_name`.
 */
export function toOrderStatusWire(status: string): OrderStatus | string {
  const normalized = status.trim();
  if (!normalized) {
    return "pending_payment";
  }
  const lowered = normalized.toLowerCase();
  if (lowered === "unpaid" || lowered === "wait_pay" || lowered === "created" || lowered === "draft") {
    return "pending_payment";
  }
  if (lowered === "canceled" || lowered === "closed" || lowered === "timeout") {
    return lowered === "timeout" ? "expired" : "cancelled";
  }
  if (lowered === "finished" || lowered === "shipped" || lowered === "delivered") {
    return lowered === "finished" ? "completed" : "fulfilled";
  }
  return normalized.toLowerCase() as OrderStatus;
}

/** Normalizes the tab id to a backend `status` query value. */
export function toOrderListStatusWire(tabId: OrderTabId | string): string | undefined {
  const normalized = tabId.trim().toLowerCase();
  if (!normalized || normalized === "all") {
    return undefined;
  }
  return normalized.replace(/-/g, "_");
}

/**
 * Converts a minor-unit amount string ("6990") into a display string
 * ("¥69.90") using the shared currency formatter. Unknown currencies or
 * malformed amounts fall back to the raw minor string.
 */
export function formatAmountCny(
  minorAmount: string | number | undefined | null,
  currencyCode = "CNY",
  locale = "zh-CN",
): string {
  if (minorAmount === undefined || minorAmount === null || minorAmount === "") {
    return "--";
  }
  const minor = toNumber(minorAmount, Number.NaN);
  if (!Number.isFinite(minor)) {
    return String(minorAmount);
  }
  return formatMoneyMinorUnits(minor, currencyCode, locale, "symbol") ?? String(minorAmount);
}

function mapOrderItem(value: Record<string, unknown>): OrderItem {
  return {
    id: toOptionalString(value.id) ?? "",
    title: toOptionalString(value.productName) ?? toOptionalString(value.title) ?? "",
    quantity: toNumber(value.quantity),
    unitPrice: toOptionalString(value.unitPrice) ?? "0",
    totalAmount: toOptionalString(value.totalAmount) ?? "0",
  };
}

function mapOrderSummary(value: Record<string, unknown>): Order {
  const id = toOptionalString(value.orderId);
  if (!id) {
    throw new Error("Order response is missing orderId.");
  }
  const itemsValue = Array.isArray(value.items) ? value.items : [];
  return {
    id,
    orderSn: toOptionalString(value.orderSn) ?? id,
    status: toOrderStatusWire(toOptionalString(value.status) ?? "pending_payment"),
    statusText: toOptionalString(value.statusName) ?? toOptionalString(value.status) ?? "pending_payment",
    subject: toOptionalString(value.subject) ?? "订单",
    totalAmount: toOptionalString(value.totalAmount) ?? "0",
    paidAmount: toOptionalString(value.paidAmount),
    discountAmount: toOptionalString(value.discountAmount),
    currencyCode: toOptionalString(value.currencyCode) ?? "CNY",
    quantity: toNumber(value.quantity),
    createdAt: toOptionalString(value.createdAt) ?? "",
    payTime: toOptionalString(value.payTime),
    expireTime: toOptionalString(value.expireTime),
    paymentMethod: toOptionalString(value.paymentMethod),
    items: itemsValue.map((item) => mapOrderItem(item as Record<string, unknown>)),
    outTradeNo: toOptionalString(value.outTradeNo),
    transactionId: toOptionalString(value.transactionId),
  };
}

/* ------------------------------------------------------------------ */
/* Service                                                            */
/* ------------------------------------------------------------------ */

export class OrderService {
  static async getOrderTabs(): Promise<readonly OrderTab[]> {
    return [
      { id: "all", labelKey: "orders.tab_all" },
      { id: "pending_payment", labelKey: "orders.tab_pending_payment" },
      { id: "paid", labelKey: "orders.tab_paid" },
      { id: "fulfilled", labelKey: "orders.tab_fulfilled" },
      { id: "completed", labelKey: "orders.tab_completed" },
      { id: "cancelled", labelKey: "orders.tab_cancelled" },
    ];
  }

  static async getOrders(tabId: OrderTabId | "all" = "all"): Promise<Order[]> {
    const { client } = requireOrderRuntime();
    const page = await client.orderOrders.orders.list({
      status: toOrderListStatusWire(tabId),
      page: 1,
      pageSize: 50,
    });
    return (page.items ?? []).map((item) => mapOrderSummary(item as Record<string, unknown>));
  }

  static async getOrderById(orderId: string): Promise<Order | null> {
    const { client } = requireOrderRuntime();
    const value = await client.orderOrders.orders.retrieve(orderId);
    if (!value || typeof value !== "object") {
      return null;
    }
    return mapOrderSummary(value as Record<string, unknown>);
  }

  static async getOrderStatistics(): Promise<OrderStatistics> {
    const { client } = requireOrderRuntime();
    const value = (await client.orderOrders.orders.statistics.retrieve()) as Record<string, unknown>;
    return {
      totalOrders: toNumber(value.totalOrders),
      pendingPayment: toNumber(value.pendingPayment),
      pendingShipment: toNumber(value.pendingShipment),
      pendingReceipt: toNumber(value.pendingReceipt),
      completed: toNumber(value.completed),
      totalAmount: toOptionalString(value.totalAmount) ?? "0",
    };
  }

  static async payOrder(
    orderId: string,
    paymentMethod: OrderPaymentMethod = "wechat_pay",
    options: { readonly openid?: string } = {},
  ): Promise<PaymentSession> {
    if (!ORDER_PAYMENT_WIRE_METHODS.includes(paymentMethod)) {
      throw new Error(`Unsupported payment method ${paymentMethod}.`);
    }
    const { client } = requireOrderRuntime();
    const body: Record<string, unknown> = { paymentMethod };
    const openid = options.openid?.trim();
    if (openid) {
      // wechat_jsapi adapter requires the payer openid in metadata.
      body.openid = openid;
    }
    const value = (await client.orderOrders.orders.payments.create(
      orderId,
      body,
      { idempotencyKey: uuid() },
    )) as Record<string, unknown>;
    const paymentId = toOptionalString(value.paymentId);
    if (!paymentId) {
      throw new Error("Payment creation did not return a payment id.");
    }
    return {
      amount: toOptionalString(value.amount) ?? "0",
      orderId: toOptionalString(value.orderId) ?? orderId,
      outTradeNo: toOptionalString(value.outTradeNo) ?? "",
      paymentId,
      paymentMethod: toOptionalString(value.paymentMethod) ?? paymentMethod,
      paymentParams: (value.paymentParams as Record<string, string> | undefined) ?? {},
    };
  }

  /**
   * Fetches the WeChat authorize URL for the cashier redirect path through
   * the host-composed OAuth channel (backed by the IAM payment OAuth
   * endpoint). The cashier redirects the payer there to obtain the openid
   * needed by `wechat_jsapi`.
   */
  static async fetchWechatOAuthAuthorizeUrl(redirect: string): Promise<string> {
    const { wechatPaymentOAuth } = requireOrderRuntime();
    if (!wechatPaymentOAuth) {
      throw new Error("WeChat payment OAuth is not composed in the runtime.");
    }
    const authorizeUrl = await wechatPaymentOAuth.fetchAuthorizeUrl(redirect);
    if (!authorizeUrl.trim()) {
      throw new Error("WeChat payment OAuth did not return an authorize URL.");
    }
    return authorizeUrl;
  }

  static async getPaymentStatus(orderId: string): Promise<PaymentStatus> {
    const { client } = requireOrderRuntime();
    const value = await client.orderOrders.orders.paymentSuccess.retrieve(orderId);
    return {
      paid: Boolean(value.paid),
      status: value.status,
      statusName: value.statusName,
    };
  }

  static async cancelOrder(orderId: string): Promise<void> {
    const { client } = requireOrderRuntime();
    await client.orderOrders.orders.cancellations.create(
      orderId,
      { idempotencyKey: uuid() },
    );
  }

  static async redeemVoucher(code: string): Promise<VoucherRedemptionResult> {
    const { client } = requireOrderRuntime();
    try {
      const result = await client.orderOrders.orders.couponRedemptions.create(
        { couponCode: code.trim().toUpperCase() },
        { idempotencyKey: uuid() },
      );
      const completed = (result as { status?: string }).status === "completed";
      return {
        success: completed,
        message: completed ? "核销成功" : "券码无效",
        orderId: (result as { orderId?: string }).orderId,
        orderNo: (result as { orderNo?: string }).orderNo,
      };
    } catch (error) {
      return {
        success: false,
        message: error instanceof Error ? error.message : "核销失败",
      };
    }
  }

  /**
   * Creates a physical-goods order through the canonical checkout session
   * flow (`sessions.create` → `sessions.quotes.create` →
   * `sessions.orders.create`) and returns the placed order. Hosts of the
   * shop domain use this as the order-creation port.
   */
  static async createOrder(input: CreateOrderInput): Promise<Order> {
    const { client } = requireOrderRuntime();
    const items = input.items.map((item) => ({
      quantity: String(item.quantity),
      skuId: item.skuId,
    }));
    if (items.length === 0) {
      throw new Error("At least one SKU is required.");
    }
    const session = await client.orderCheckout.checkout.sessions.create(
      {
        currencyCode: (input.currencyCode ?? "CNY").trim().toUpperCase(),
        items,
        shippingAddress: {
          receiverName: input.shippingAddress.receiverName,
          receiverPhone: input.shippingAddress.receiverPhone,
          countryCode: input.shippingAddress.countryCode,
          province: input.shippingAddress.province,
          city: input.shippingAddress.city,
          district: input.shippingAddress.district,
          detailAddress: input.shippingAddress.detailAddress,
          postalCode: input.shippingAddress.postalCode,
        },
      },
      { idempotencyKey: uuid() },
    );
    const checkoutSessionId = session.checkoutSessionId;
    if (!checkoutSessionId) {
      throw new Error("Checkout session creation did not return a session id.");
    }
    await client.orderCheckout.checkout.sessions.quotes.create(
      checkoutSessionId,
      { idempotencyKey: uuid() },
    );
    const order = await client.orderCheckout.checkout.sessions.orders.create(
      checkoutSessionId,
      { idempotencyKey: uuid() },
    );
    const orderId = order.orderId;
    if (!orderId) {
      throw new Error("Order placement did not return an order id.");
    }
    const placed = await OrderService.getOrderById(orderId);
    if (!placed) {
      throw new Error("Placed order could not be retrieved.");
    }
    return placed;
  }
}
