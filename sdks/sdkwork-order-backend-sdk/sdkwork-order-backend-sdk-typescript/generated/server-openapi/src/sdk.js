import { createHttpClient } from './http/client';
import { createOrderAdminOrdersApi } from './api/order-admin-orders';
import { createOrderAdminAfterSalesApi } from './api/order-admin-after-sales';
import { createOrderAdminShipmentsApi } from './api/order-admin-shipments';
import { createOrderAdminBackendApi } from './api/order-admin-backend';
export class SdkworkOrderBackendClient {
    httpClient;
    orderAdminOrders;
    orderAdminAfterSales;
    orderAdminShipments;
    orderAdminBackend;
    constructor(config) {
        this.httpClient = createHttpClient(config);
        this.orderAdminOrders = createOrderAdminOrdersApi(this.httpClient);
        this.orderAdminAfterSales = createOrderAdminAfterSalesApi(this.httpClient);
        this.orderAdminShipments = createOrderAdminShipmentsApi(this.httpClient);
        this.orderAdminBackend = createOrderAdminBackendApi(this.httpClient);
    }
    setAuthToken(token) {
        this.httpClient.setAuthToken(token);
        return this;
    }
    setAccessToken(token) {
        this.httpClient.setAccessToken(token);
        return this;
    }
    setTokenManager(manager) {
        this.httpClient.setTokenManager(manager);
        return this;
    }
    get http() {
        return this.httpClient;
    }
}
export function createClient(config) {
    return new SdkworkOrderBackendClient(config);
}
export default SdkworkOrderBackendClient;
//# sourceMappingURL=sdk.js.map