import type { AuthTokenManager } from "@sdkwork/sdk-common";
import { type SdkworkAppClient as OrderAppTransportClient } from "@sdkwork/order-app-sdk";
export declare function resolveOrderAppApiOrigin(baseUrl: string): string;
export interface BootstrapSdkworkOrderAppServiceInput {
    baseUrl: string;
    authToken?: string;
    accessToken?: string;
    tenantId?: string;
    organizationId?: string;
    platform?: string;
    tokenManager?: AuthTokenManager;
}
export declare function createOrderAppTransportClient(input: BootstrapSdkworkOrderAppServiceInput): OrderAppTransportClient;
//# sourceMappingURL=transport.d.ts.map