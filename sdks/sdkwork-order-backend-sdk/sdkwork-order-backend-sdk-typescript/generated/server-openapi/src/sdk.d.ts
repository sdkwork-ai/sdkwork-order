import { HttpClient } from './http/client';
import type { SdkworkBackendConfig } from './types/common';
import type { AuthTokenManager } from '@sdkwork/sdk-common';
import { OrderAdminOrdersApi } from './api/order-admin-orders';
import { OrderAdminAfterSalesApi } from './api/order-admin-after-sales';
import { OrderAdminShipmentsApi } from './api/order-admin-shipments';
import { OrderAdminBackendApi } from './api/order-admin-backend';
export declare class SdkworkOrderBackendClient {
    private httpClient;
    readonly orderAdminOrders: OrderAdminOrdersApi;
    readonly orderAdminAfterSales: OrderAdminAfterSalesApi;
    readonly orderAdminShipments: OrderAdminShipmentsApi;
    readonly orderAdminBackend: OrderAdminBackendApi;
    constructor(config: SdkworkBackendConfig);
    setAuthToken(token: string): this;
    setAccessToken(token: string): this;
    setTokenManager(manager: AuthTokenManager): this;
    get http(): HttpClient;
}
export declare function createClient(config: SdkworkBackendConfig): SdkworkOrderBackendClient;
export default SdkworkOrderBackendClient;
//# sourceMappingURL=sdk.d.ts.map