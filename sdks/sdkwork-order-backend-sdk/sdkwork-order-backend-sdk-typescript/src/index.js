import { createClient as createGeneratedBackendClient, SdkworkOrderBackendClient as GeneratedSdkworkOrderBackendClient, } from '../generated/server-openapi/src/index';
export { createGeneratedBackendClient, };
export * from '../generated/server-openapi/src/types';
export * from '../generated/server-openapi/src/api';
export * from '../generated/server-openapi/src/http';
export * from '../generated/server-openapi/src/auth';
export class SdkworkOrderBackendClient extends GeneratedSdkworkOrderBackendClient {
    afterSales;
    backend;
    orders;
    shipments;
    constructor(config) {
        super(config);
        this.afterSales = this.orderAdminAfterSales.afterSales;
        this.backend = this.orderAdminBackend.backend;
        this.orders = this.orderAdminOrders.orders;
        this.shipments = this.orderAdminShipments.shipments;
    }
}
export { SdkworkOrderBackendClient as SdkworkBackendClient };
export function createClient(config) {
    return new SdkworkOrderBackendClient(config);
}
//# sourceMappingURL=index.js.map