import { createClient as createAppbaseAppClient, type SdkworkAppClient } from "@sdkwork/iam-app-sdk";
import {
  createSdkworkAppbasePcAuthRuntime,
  type SdkworkAppbasePcAuthRuntimeComposition,
  type SdkworkAppbasePcAuthRuntimeSdkClient,
} from "@sdkwork/auth-runtime-pc-react";
import type { IamAppContext, IamDeploymentMode, IamEnvironment } from "@sdkwork/iam-contracts";
import type { IamRuntime } from "@sdkwork/iam-runtime";
import { normalizeSdkworkApiBaseUrl } from "@sdkwork/runtime-bootstrap";
import { createClient as createOrderAppClient, type SdkworkAppClient as SdkworkOrderAppClient } from "@sdkwork/order-app-sdk";

import type { SdkworkOrderH5RuntimeConfig } from "./environment";
import {
  createSdkworkOrderH5SessionStore,
  SDKWORK_ORDER_H5_SESSION_STORAGE_KEY,
  type SdkworkOrderH5SessionSnapshot,
  type SdkworkOrderH5SessionStore,
} from "@sdkwork/order-h5-shell";
import { createSdkworkOrderH5SessionTokenManager } from "./sessionTokenManager";
import type { SdkworkOrderH5SdkClientInventory } from "./sdkClients";

const APPBASE_APP_SDK_FAMILY_ID = "sdkwork-iam-app-sdk";
const DOMAIN_APP_SDK_FAMILY_IDS = {
  order: "sdkwork-order-app-sdk",
} as const;
const APP_API_PREFIX = "/app/v3/api";
const BACKEND_API_PREFIX = "/backend/v3/api";

export type SdkworkOrderH5IamRuntime = IamRuntime & {
  composition: SdkworkAppbasePcAuthRuntimeComposition;
  session: SdkworkOrderH5SessionStore;
};

export interface CreateSdkworkOrderH5IamRuntimeOptions {
  config: SdkworkOrderH5RuntimeConfig;
  sdkClients: SdkworkOrderH5SdkClientInventory;
  session?: SdkworkOrderH5SessionStore;
}

interface OrderH5IamSessionLike {
  accessToken?: string;
  authToken?: string;
  refreshToken?: string;
  sessionId?: string;
  context?: IamAppContext;
}

export function createSdkworkOrderH5IamRuntime(
  options: CreateSdkworkOrderH5IamRuntimeOptions,
): SdkworkOrderH5IamRuntime {
  const session = options.session ?? createSdkworkOrderH5SessionStore(resolveSessionStorage());
  const tokenManager = createSdkworkOrderH5SessionTokenManager(session);
  const appbaseAppClient = createAppbaseGeneratedAppClient(options.config, tokenManager);
  const composition = createSdkworkAppbasePcAuthRuntime({
    app: {
      appId: options.config.appKey,
      deploymentMode: toIamDeploymentMode(options.config.deploymentMode),
      environment: toIamEnvironment(options.config.environment),
      platform: "pc",
    },
    baseUrls: {
      appbaseAppApiBaseUrl: resolveAppbaseAppApiBaseUrl(options.config),
    },
    createAppbaseAppClient: () => appbaseAppClient,
    localeProvider: () => options.config.i18n.defaultLocale,
    sdkClients: [
      options.sdkClients.orderAppClient,
    ] as SdkworkAppbasePcAuthRuntimeSdkClient[],
    sessionBridge: {
      clearSession: () => {
        session.clearSession();
      },
      commitSession: (nextSession) => commitOrderH5IamRuntimeSession(session, nextSession as OrderH5IamSessionLike),
      readSession: () => toOrderH5IamBridgeSession(session.getSnapshot()),
    },
    tokenManager,
  });

  return {
    ...composition.runtime,
    composition,
    session,
  };
}

export function createSdkworkOrderH5SdkClientsWithTokenManager(
  config: SdkworkOrderH5RuntimeConfig,
  tokenManager: ReturnType<typeof createSdkworkOrderH5SessionTokenManager>,
): SdkworkOrderH5SdkClientInventory {
  const orderAppClient = createOrderAppClient({
    authMode: "dual-token",
    baseUrl: normalizeGeneratedSdkBaseUrl(
      resolveDependencyAppApiBaseUrl(config, DOMAIN_APP_SDK_FAMILY_IDS.order),
      APP_API_PREFIX,
    ),
    platform: "h5",
    tokenManager,
  });
  orderAppClient.setTokenManager(tokenManager);

  return {
    appApiBaseUrl: normalizeSdkworkApiBaseUrl(config.appApiBaseUrl, "app"),
    orderAppClient,
    sdkFamilies: {
      app: [
        "sdkwork-iam-app-sdk",
        "sdkwork-order-app-sdk",
      ],
      backendAdmin: [],
    },
  };
}

function createAppbaseGeneratedAppClient(
  config: SdkworkOrderH5RuntimeConfig,
  tokenManager: ReturnType<typeof createSdkworkOrderH5SessionTokenManager>,
): SdkworkAppClient {
  return createAppbaseAppClient({
    authMode: "dual-token",
    baseUrl: normalizeGeneratedSdkBaseUrl(resolveAppbaseAppApiBaseUrl(config), APP_API_PREFIX),
    platform: "pc",
    tokenManager,
  });
}

function resolveAppbaseAppApiBaseUrl(config: SdkworkOrderH5RuntimeConfig): string {
  return config.sdkBaseUrls?.dependencySdkBaseUrls?.[APPBASE_APP_SDK_FAMILY_ID]?.appApiBaseUrl
    ?? config.appApiBaseUrl;
}

function normalizeGeneratedSdkBaseUrl(baseUrl: string, apiPrefix: string): string {
  const normalizedBaseUrl = baseUrl.replace(/\/+$/u, "");
  const normalizedApiPrefix = apiPrefix.replace(/\/+$/u, "");
  if (normalizedBaseUrl.endsWith(normalizedApiPrefix)) {
    return normalizedBaseUrl.slice(0, -normalizedApiPrefix.length).replace(/\/+$/u, "");
  }
  return normalizedBaseUrl;
}

function commitOrderH5IamRuntimeSession(
  session: SdkworkOrderH5SessionStore,
  iamSession: OrderH5IamSessionLike,
): OrderH5IamSessionLike | undefined {
  const nextSession: SdkworkOrderH5SessionSnapshot = {
    ...session.getSnapshot(),
    accessToken: iamSession.accessToken,
    authToken: iamSession.authToken,
    refreshToken: iamSession.refreshToken,
    sessionId: iamSession.sessionId ?? iamSession.context?.sessionId,
    context: iamSession.context
      ? {
          tenantId: iamSession.context.tenantId,
          userId: iamSession.context.userId,
          organizationId: iamSession.context.organizationId,
          sessionId: iamSession.context.sessionId,
          appId: iamSession.context.appId,
          environment: iamSession.context.environment,
          deploymentMode: iamSession.context.deploymentMode,
        }
      : undefined,
  };

  if (!nextSession.context) {
    delete nextSession.context;
  }

  session.setSession(nextSession);
  return toOrderH5IamBridgeSession(session.getSnapshot()) ?? undefined;
}

function toOrderH5IamBridgeSession(
  snapshot: SdkworkOrderH5SessionSnapshot,
): OrderH5IamSessionLike | null {
  if (!snapshot.authToken && !snapshot.accessToken && !snapshot.refreshToken) {
    return null;
  }

  return {
    ...(snapshot.accessToken ? { accessToken: snapshot.accessToken } : {}),
    ...(snapshot.authToken ? { authToken: snapshot.authToken } : {}),
    ...(snapshot.refreshToken ? { refreshToken: snapshot.refreshToken } : {}),
    ...(snapshot.sessionId ? { sessionId: snapshot.sessionId } : {}),
    ...(snapshot.context?.tenantId && snapshot.context.userId
      ? {
          context: {
            tenantId: snapshot.context.tenantId,
            userId: snapshot.context.userId,
            organizationId: snapshot.context.organizationId,
            sessionId: snapshot.context.sessionId,
            appId: snapshot.context.appId,
            environment: snapshot.context.environment,
            deploymentMode: snapshot.context.deploymentMode,
          } as IamAppContext,
        }
      : {}),
  };
}

function resolveSessionStorage(): Storage | undefined {
  if (typeof window === "undefined") {
    return undefined;
  }
  migrateLegacySessionStorage(SDKWORK_ORDER_H5_SESSION_STORAGE_KEY);
  return window.localStorage;
}

function resolveDependencyAppApiBaseUrl(
  config: SdkworkOrderH5RuntimeConfig,
  sdkFamily: string,
): string {
  return config.sdkBaseUrls?.dependencySdkBaseUrls?.[sdkFamily]?.appApiBaseUrl
    ?? config.appApiBaseUrl;
}

function migrateLegacySessionStorage(storageKey: string): void {
  const legacySession = window.sessionStorage.getItem(storageKey);
  if (legacySession && !window.localStorage.getItem(storageKey)) {
    window.localStorage.setItem(storageKey, legacySession);
  }
  if (legacySession) {
    window.sessionStorage.removeItem(storageKey);
  }
}

function toIamDeploymentMode(value: SdkworkOrderH5RuntimeConfig["deploymentMode"]): IamDeploymentMode {
  return value === "web" ? "saas" : value;
}

function toIamEnvironment(value: SdkworkOrderH5RuntimeConfig["environment"]): IamEnvironment {
  if (value === "development") {
    return "dev";
  }
  if (value === "production") {
    return "prod";
  }
  return "test";
}
