import type { AuthTokenManager } from "@sdkwork/sdk-common";
import { type SdkworkOrderBackendClient } from "@sdkwork/order-backend-sdk";
export declare function resolveOrderBackendApiOrigin(baseUrl: string): string;
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
export declare function createOrderBackendTransportClient(input: BootstrapSdkworkOrderBackendSdkInput): SdkworkOrderBackendClient;
export declare function bootstrapSdkworkOrderBackendSdk(input: BootstrapSdkworkOrderBackendSdkInput): SdkworkOrderBackendClient;
export declare function getSdkworkOrderBackendSdkClient(): SdkworkOrderBackendClient;
export declare function resetSdkworkOrderBackendSdkClient(): void;
//# sourceMappingURL=backend-transport.d.ts.map