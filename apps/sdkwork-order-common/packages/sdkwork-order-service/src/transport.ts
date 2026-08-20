import type { AuthTokenManager } from "@sdkwork/sdk-common";
import {
  createClient,
  type SdkworkAppClient as OrderAppTransportClient,
  type SdkworkAppConfig,
} from "@sdkwork/order-app-sdk";

const APP_API_SUFFIX = "/app/v3/api";

export function resolveOrderAppApiOrigin(baseUrl: string): string {
  const trimmed = baseUrl.trim().replace(/\/+$/u, "");
  if (trimmed.endsWith(APP_API_SUFFIX)) {
    return trimmed.slice(0, -APP_API_SUFFIX.length);
  }
  return trimmed;
}

export interface BootstrapSdkworkOrderAppServiceInput {
  baseUrl: string;
  authToken?: string;
  accessToken?: string;
  tenantId?: string;
  organizationId?: string;
  platform?: string;
  tokenManager?: AuthTokenManager;
}

export function createOrderAppTransportClient(
  input: BootstrapSdkworkOrderAppServiceInput,
): OrderAppTransportClient {
  const config: SdkworkAppConfig = {
    baseUrl: resolveOrderAppApiOrigin(input.baseUrl),
    authToken: input.authToken,
    accessToken: input.accessToken,
    tenantId: input.tenantId,
    organizationId: input.organizationId,
    platform: input.platform,
    tokenManager: input.tokenManager,
  };
  return createClient(config);
}
