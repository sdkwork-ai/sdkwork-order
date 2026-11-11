/**
 * Subscription purchase service (thin re-export).
 *
 * The implementation lives in `@sdkwork/order-h5-core` (capability packages
 * must not import generated SDK modules directly; SDK wiring belongs in the
 * core public exports). This module keeps the package's public export
 * surface stable for existing consumers.
 */
export {
  COUPON_BENEFIT_DELIVERY_FAILED_I18N_KEY,
  classifyCouponRedemptionFailure,
  createSubscriptionPurchaseService,
} from "@sdkwork/order-h5-core/sdk";
export type {
  CouponRedemptionFailure,
  CouponRedemptionFailureKind,
  CreateSubscriptionPurchaseServiceOptions,
  CouponRedemptionResult,
  SubscriptionPurchasePort,
  TokenBankPayment,
} from "@sdkwork/order-h5-core/sdk";
