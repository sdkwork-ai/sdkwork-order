import type { SdkworkAppClient as SdkworkOrderAppClient } from "@sdkwork/order-app-sdk";

import type { SdkworkOrderH5RuntimeConfig } from "./environment";

export interface SdkworkOrderH5SdkClientInventory {
  appApiBaseUrl: string;
  orderAppClient: SdkworkOrderAppClient;
  sdkFamilies: {
    app: string[];
    backendAdmin: string[];
  };
}

/**
 * H5 ships no backend-admin SDK clients (APP_H5_ARCHITECTURE_SPEC section 7):
 * app packages use generated app SDK clients only.
 */
export function listSdkworkOrderH5RegisteredSdkFamilies(
  config: SdkworkOrderH5RuntimeConfig,
): SdkworkOrderH5SdkClientInventory["sdkFamilies"] {
  void config;
  return {
    app: [
      "sdkwork-iam-app-sdk",
      "sdkwork-order-app-sdk",
    ],
    backendAdmin: [],
  };
}
