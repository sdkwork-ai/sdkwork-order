import {
  resolveSdkworkOrderH5RuntimeConfig,
  type SdkworkOrderH5RuntimeConfig,
} from "./environment";
import { configureSdkworkOrderH5OrderRuntime, type SdkworkOrderH5OrderRuntime } from "./orderRuntime";
import {
  createSdkworkOrderH5IamRuntime,
  createSdkworkOrderH5SdkClientsWithTokenManager,
  type SdkworkOrderH5IamRuntime,
} from "./iamRuntime";
import {
  createSdkworkOrderH5SessionStore,
  registerSdkworkOrderH5SessionStoreLocator,
  type SdkworkOrderH5SessionStore,
} from "@sdkwork/order-h5-shell";
import { createSdkworkOrderH5SessionTokenManager } from "./sessionTokenManager";
import type { SdkworkOrderH5SdkClientInventory } from "./sdkClients";


export interface SdkworkOrderH5Runtime {
  config: SdkworkOrderH5RuntimeConfig;
  iamRuntime: SdkworkOrderH5IamRuntime;
  orderRuntime: SdkworkOrderH5OrderRuntime;
  sdkClients: SdkworkOrderH5SdkClientInventory;
  session: SdkworkOrderH5SessionStore;
}

export function createSdkworkOrderH5Runtime(): SdkworkOrderH5Runtime {
  const config = resolveSdkworkOrderH5RuntimeConfig();
  const session = createSdkworkOrderH5SessionStore(
    typeof window === "undefined" ? undefined : window.localStorage,
  );
  registerSdkworkOrderH5SessionStoreLocator(() => session);
  const tokenManager = createSdkworkOrderH5SessionTokenManager(session);
  const sdkClients = createSdkworkOrderH5SdkClientsWithTokenManager(config, tokenManager);
  const iamRuntime = createSdkworkOrderH5IamRuntime({
    config,
    sdkClients,
    session,
  });
  const orderRuntime = configureSdkworkOrderH5OrderRuntime({
    config,
    sdkClients,
  });

  return {
    config,
    iamRuntime,
    orderRuntime,
    sdkClients,
    session,
  };
}
