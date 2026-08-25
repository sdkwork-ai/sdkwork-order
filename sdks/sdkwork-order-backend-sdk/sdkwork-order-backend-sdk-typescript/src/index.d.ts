import { createClient as createGeneratedBackendClient, SdkworkOrderBackendClient as GeneratedSdkworkOrderBackendClient } from '../generated/server-openapi/src/index';
import type { SdkworkBackendConfig } from '../generated/server-openapi/src/types/common';
export { createGeneratedBackendClient, };
export type { SdkworkBackendConfig };
export * from '../generated/server-openapi/src/types';
export * from '../generated/server-openapi/src/api';
export * from '../generated/server-openapi/src/http';
export * from '../generated/server-openapi/src/auth';
export declare class SdkworkOrderBackendClient extends GeneratedSdkworkOrderBackendClient {
    readonly afterSales: GeneratedSdkworkOrderBackendClient["orderAdminAfterSales"]["afterSales"];
    readonly backend: GeneratedSdkworkOrderBackendClient["orderAdminBackend"]["backend"];
    readonly orders: GeneratedSdkworkOrderBackendClient["orderAdminOrders"]["orders"];
    readonly shipments: GeneratedSdkworkOrderBackendClient["orderAdminShipments"]["shipments"];
    constructor(config: SdkworkBackendConfig);
}
export { SdkworkOrderBackendClient as SdkworkBackendClient };
export declare function createClient(config: SdkworkBackendConfig): SdkworkOrderBackendClient;
//# sourceMappingURL=index.d.ts.map