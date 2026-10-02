import {
  configureOrderMobileRuntime,
  type PaymentRegion,
} from "@sdkwork/order-mobile-react-orders";
import {
  createSubscriptionPurchaseService,
  type SubscriptionPurchasePort,
} from "@sdkwork/order-h5-subscription";
import {
  createWithdrawalRequestService,
  type WithdrawalRequestPort,
} from "@sdkwork/order-h5-withdraw";

import type { SdkworkOrderH5RuntimeConfig } from "./environment";
import type { SdkworkOrderH5SdkClientInventory } from "./sdkClients";

/**
 * Order mobile runtime composition (bootstrap-owned SDK construction).
 *
 * `configureOrderMobileRuntime` injects the bootstrap-built Order App SDK
 * client (dual-token + token manager) into `OrderService` and the cashier
 * pages. UI packages never construct SDK clients themselves.
 *
 * Payment environment wiring:
 * - The cashier detects the in-app environment (WeChat / Alipay / browser)
 *   from the user agent at runtime (`detectPaymentEnvironment`).
 * - The deployment region is fixed to `cn` for this deployment: the order
 *   backend accepts the CN wire methods only today, so the locale fallback
 *   must not flip overseas browsers onto paypal/stripe channels that would
 *   fail server-side.
 * - The WeChat JSAPI OAuth channel is not composed yet; WeChat in-app JSAPI
 *   payment stays unavailable until the IAM payment OAuth endpoint is wired
 *   (see docs/decisions.md).
 */
const ORDER_PAYMENT_REGION: PaymentRegion = "cn";

export interface SdkworkOrderH5OrderRuntime {
  paymentRegion: PaymentRegion;
  /** Lazily resolves the shared subscription purchase service (VIP/Token Bank/coupon). */
  resolveSubscriptionPurchaseService(): SubscriptionPurchasePort;
  /** Lazily resolves the withdraw request service. */
  resolveWithdrawalRequestService(): WithdrawalRequestPort;
}

export function configureSdkworkOrderH5OrderRuntime(input: {
  config: SdkworkOrderH5RuntimeConfig;
  sdkClients: SdkworkOrderH5SdkClientInventory;
}): SdkworkOrderH5OrderRuntime {
  void input.config;

  configureOrderMobileRuntime({
    client: input.sdkClients.orderAppClient,
    paymentRegion: ORDER_PAYMENT_REGION,
  });

  return {
    paymentRegion: ORDER_PAYMENT_REGION,
    // Lazy resolution keeps the services on the bootstrap-initialized client:
    // bootstrap runs before the first render, so every render-time resolution
    // observes the token-manager-bound instance (im-h5 module pattern).
    resolveSubscriptionPurchaseService: () =>
      createSubscriptionPurchaseService({
        orderAppSdkClient: input.sdkClients.orderAppClient,
      }),
    resolveWithdrawalRequestService: () =>
      createWithdrawalRequestService({
        orderAppSdkClient: input.sdkClients.orderAppClient,
      }),
  };
}
