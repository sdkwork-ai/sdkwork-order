import { createClient, } from "@sdkwork/order-backend-sdk";
const BACKEND_API_SUFFIX = "/backend/v3/api";
export function resolveOrderBackendApiOrigin(baseUrl) {
    const trimmed = baseUrl.trim().replace(/\/+$/u, "");
    if (trimmed.endsWith(BACKEND_API_SUFFIX)) {
        return trimmed.slice(0, -BACKEND_API_SUFFIX.length);
    }
    return trimmed;
}
export function createOrderBackendTransportClient(input) {
    const config = {
        authMode: "dual-token",
        baseUrl: resolveOrderBackendApiOrigin(input.baseUrl),
        authToken: input.authToken,
        accessToken: input.accessToken,
        tenantId: input.tenantId,
        organizationId: input.organizationId,
        platform: input.platform ?? "pc",
        tokenManager: input.tokenManager,
    };
    return createClient(config);
}
let backendClient = null;
export function bootstrapSdkworkOrderBackendSdk(input) {
    backendClient = createOrderBackendTransportClient(input);
    return backendClient;
}
export function getSdkworkOrderBackendSdkClient() {
    if (!backendClient) {
        throw new Error("SDKWork order backend SDK is not configured. Call bootstrapSdkworkOrderBackendSdk() from order PC bootstrap.");
    }
    return backendClient;
}
export function resetSdkworkOrderBackendSdkClient() {
    backendClient = null;
}
//# sourceMappingURL=backend-transport.js.map