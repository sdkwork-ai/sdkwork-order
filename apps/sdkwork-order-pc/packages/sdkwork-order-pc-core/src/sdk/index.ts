export const SDKWORK_ORDER_PC_SDK_PACKAGES = {
  app: "@sdkwork/order-app-sdk",
  backend: "@sdkwork/order-backend-sdk",
} as const;

export type SdkworkOrderPcSdkPackageRole = keyof typeof SDKWORK_ORDER_PC_SDK_PACKAGES;

export {
  createOrderAppSdkClient,
  type SdkworkOrderAppClient,
  type SdkworkOrderAppSdkConfig,
} from "./order-app-sdk.ts";
export {
  createMembershipAppSdkClient,
  type SdkworkMembershipAppClient,
} from "./membership-app-sdk.ts";
