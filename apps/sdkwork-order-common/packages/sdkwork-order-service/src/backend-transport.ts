import type { AuthTokenManager } from "@sdkwork/sdk-common";
import {
  createClient,
  type SdkworkOrderBackendClient,
  type SdkworkBackendConfig,
} from "@sdkwork/order-backend-sdk";

const BACKEND_API_SUFFIX = "/backend/v3/api";

export function resolveOrderBackendApiOrigin(baseUrl: string): string {
  const trimmed = baseUrl.trim().replace(/\/+$/u, "");
  if (trimmed.endsWith(BACKEND_API_SUFFIX)) {
    return trimmed.slice(0, -BACKEND_API_SUFFIX.length);
  }
  return trimmed;
}

export interface BootstrapSdkworkOrderBackendSdkInput {
  baseUrl: string;
  authToken?: string;
  accessToken?: string;
  tenantId?: string;
  organizationId?: string;
  platform?: string;
  /**
   * Live token manager injected by embedding hosts (e.g. the cloudrouter
   * portal). When present it supplies per-request session tokens and takes
   * precedence over the static auth/access token pair.
   */
  tokenManager?: AuthTokenManager;
}

export function createOrderBackendTransportClient(
  input: BootstrapSdkworkOrderBackendSdkInput,
): SdkworkOrderBackendClient {
  const config: SdkworkBackendConfig = {
    authMode: "dual-token",
    baseUrl: resolveOrderBackendApiOrigin(input.baseUrl),
    ...(input.authToken === undefined ? {} : { authToken: input.authToken }),
    ...(input.accessToken === undefined ? {} : { accessToken: input.accessToken }),
    ...(input.tenantId === undefined ? {} : { tenantId: input.tenantId }),
    ...(input.organizationId === undefined ? {} : { organizationId: input.organizationId }),
    platform: input.platform ?? "pc",
    ...(input.tokenManager === undefined ? {} : { tokenManager: input.tokenManager }),
  };
  return createClient(config);
}

let backendClient: SdkworkOrderBackendClient | null = null;

export function bootstrapSdkworkOrderBackendSdk(
  input: BootstrapSdkworkOrderBackendSdkInput,
): SdkworkOrderBackendClient {
  backendClient = createOrderBackendTransportClient(input);
  return backendClient;
}

export function getSdkworkOrderBackendSdkClient(): SdkworkOrderBackendClient {
  if (!backendClient) {
    throw new Error(
      "SDKWork order backend SDK is not configured. Call bootstrapSdkworkOrderBackendSdk() from order PC bootstrap.",
    );
  }
  return backendClient;
}

export function resetSdkworkOrderBackendSdkClient(): void {
  backendClient = null;
}
