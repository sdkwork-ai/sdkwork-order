/**
 * Subscription catalog port (thin re-export).
 *
 * The implementation lives in `@sdkwork/order-h5-core` (capability packages
 * must not import generated SDK modules directly; SDK wiring belongs in the
 * core public exports). This module keeps the package's public export
 * surface stable for existing consumers.
 */
export {
  createDefaultSubscriptionCatalogPort,
  formatCatalogPrice,
} from "@sdkwork/order-h5-core/sdk";
export type {
  CreateSubscriptionCatalogPortOptions,
  MembershipPackage,
  MembershipPackageGroup,
  SubscriptionCatalogPort,
  TokenBankPlan,
} from "@sdkwork/order-h5-core/sdk";
