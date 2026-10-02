import manifest from "../../sdkwork.app.config.json";
import { resolveSharedSdkApiBaseUrl } from "./resolveSdkApiBaseUrl";

export type SdkworkOrderH5Environment =
  | "development"
  | "test"
  | "staging"
  | "demo"
  | "production";

export type SdkworkOrderH5ConfigProfile =
  | "dev"
  | "test"
  | "staging"
  | "demo"
  | "prod";

export type SdkworkOrderH5DeploymentMode = "web";
export type SdkworkOrderH5RuntimeTarget = "browser";
export type SdkworkOrderH5BuildMode = SdkworkOrderH5Environment;

export interface SdkworkOrderH5AuthRuntimeConfig {
  accessTokenHeader: "Access-Token";
  authTokenHeader: "Authorization";
  refreshEnabled: boolean;
  tokenManagerMode: "appbase-global";
  tokenStorage: "browser-session";
}

export interface SdkworkOrderH5I18nRuntimeConfig {
  defaultLocale: string;
  fallbackLocale: string;
  supportedLocales: string[];
}

export interface SdkworkOrderH5DependencySdkBaseUrls {
  appApiBaseUrl?: string;
  backendApiBaseUrl?: string;
}

export interface SdkworkOrderH5SdkBaseUrls {
  appApiBaseUrl?: string;
  backendApiBaseUrl?: string;
  dependencySdkBaseUrls?: Record<string, SdkworkOrderH5DependencySdkBaseUrls>;
  sdkBaseUrl?: string;
}

export interface SdkworkOrderH5RuntimeConfig {
  appApiBaseUrl: string;
  appDisplayName: string;
  appKey: string;
  auth: SdkworkOrderH5AuthRuntimeConfig;
  backendApiBaseUrl?: string;
  buildMode: SdkworkOrderH5BuildMode;
  configProfile: SdkworkOrderH5ConfigProfile;
  deploymentMode: SdkworkOrderH5DeploymentMode;
  environment: SdkworkOrderH5Environment;
  i18n: SdkworkOrderH5I18nRuntimeConfig;
  runtimeTarget: SdkworkOrderH5RuntimeTarget;
  sdkBaseUrl?: string;
  sdkBaseUrls?: SdkworkOrderH5SdkBaseUrls;
  version: string;
}

const environmentByMode: Record<string, SdkworkOrderH5Environment> = {
  development: "development",
  dev: "development",
  demo: "demo",
  production: "production",
  prod: "production",
  staging: "staging",
  test: "test",
};

const profileByEnvironment: Record<SdkworkOrderH5Environment, SdkworkOrderH5ConfigProfile> = {
  demo: "demo",
  development: "dev",
  production: "prod",
  staging: "staging",
  test: "test",
};

function envValue(key: string): string | undefined {
  const value = import.meta.env[key];
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function resolveEnvironment(mode: string): SdkworkOrderH5Environment {
  return environmentByMode[mode] ?? "development";
}

function parseSdkBaseUrls(
  sdkBaseUrl?: string,
): SdkworkOrderH5SdkBaseUrls | undefined {
  const raw = envValue("VITE_SDKWORK_ORDER_H5_SDK_BASE_URLS_JSON");
  if (raw) {
    try {
      return JSON.parse(raw) as SdkworkOrderH5SdkBaseUrls;
    } catch {
      return undefined;
    }
  }

  if (!sdkBaseUrl) {
    return undefined;
  }

  const normalizedSdkBaseUrl = sdkBaseUrl.replace(/\/+$/u, "");
  return {
    appApiBaseUrl: `${normalizedSdkBaseUrl}/app/v3/api`,
    backendApiBaseUrl: `${normalizedSdkBaseUrl}/backend/v3/api`,
    dependencySdkBaseUrls: {
      "sdkwork-iam-app-sdk": {
        appApiBaseUrl: `${normalizedSdkBaseUrl}/app/v3/api`,
      },
      "sdkwork-iam-backend-sdk": {
        backendApiBaseUrl: `${normalizedSdkBaseUrl}/backend/v3/api`,
      },
    },
    sdkBaseUrl: normalizedSdkBaseUrl,
  };
}

export function resolveSdkworkOrderH5RuntimeConfig(
  mode = import.meta.env.MODE,
): SdkworkOrderH5RuntimeConfig {
  const environment = resolveEnvironment(mode);
  const sdkBaseUrl = envValue("VITE_SDKWORK_ORDER_H5_SDK_BASE_URL");
  const sdkBaseUrls = parseSdkBaseUrls(sdkBaseUrl);

  return {
    appApiBaseUrl: (resolveSharedSdkApiBaseUrl() !== undefined
      ? `${resolveSharedSdkApiBaseUrl()}/app/v3/api`
      : undefined)
      ?? envValue("VITE_SDKWORK_ORDER_H5_APP_API_BASE_URL")
      ?? sdkBaseUrls?.appApiBaseUrl
      ?? (sdkBaseUrl ? `${sdkBaseUrl.replace(/\/+$/u, "")}/app/v3/api` : "/app/v3/api"),
    appDisplayName: manifest.app.displayName,
    appKey: manifest.app.key,
    auth: {
      accessTokenHeader: "Access-Token",
      authTokenHeader: "Authorization",
      refreshEnabled: true,
      tokenManagerMode: "appbase-global",
      tokenStorage: "browser-session",
    },
    backendApiBaseUrl: (resolveSharedSdkApiBaseUrl() !== undefined
      ? `${resolveSharedSdkApiBaseUrl()}/backend/v3/api`
      : undefined)
      ?? envValue("VITE_SDKWORK_ORDER_H5_BACKEND_API_BASE_URL")
      ?? sdkBaseUrls?.backendApiBaseUrl,
    buildMode: environment,
    configProfile: profileByEnvironment[environment],
    deploymentMode: "web",
    environment,
    i18n: {
      defaultLocale: envValue("VITE_SDKWORK_ORDER_H5_DEFAULT_LOCALE") ?? "zh-CN",
      fallbackLocale: "en-US",
      supportedLocales: ["zh-CN", "en-US"],
    },
    runtimeTarget: "browser",
    sdkBaseUrl,
    sdkBaseUrls,
    version: manifest.release.currentVersion,
  };
}
