import { createHttpClient } from './http/client';
import { createOrderCheckoutApi } from './api/order-checkout';
import { createOrderOrdersApi } from './api/order-orders';
import { createOrderPaymentsApi } from './api/order-payments';
import { createOrderAfterSalesApi } from './api/order-after-sales';
import { createOrderFulfillmentsApi } from './api/order-fulfillments';
import { createOrderShipmentsApi } from './api/order-shipments';
import { createOrderRechargesApi } from './api/order-recharges';
import { createOrderMembershipsApi } from './api/order-memberships';
import { createOrderWithdrawalsApi } from './api/order-withdrawals';
export class SdkworkAppClient {
    httpClient;
    orderCheckout;
    orderOrders;
    orderPayments;
    orderAfterSales;
    orderFulfillments;
    orderShipments;
    orderRecharges;
    orderMemberships;
    orderWithdrawals;
    constructor(config) {
        this.httpClient = createHttpClient(config);
        this.orderCheckout = createOrderCheckoutApi(this.httpClient);
        this.orderOrders = createOrderOrdersApi(this.httpClient);
        this.orderPayments = createOrderPaymentsApi(this.httpClient);
        this.orderAfterSales = createOrderAfterSalesApi(this.httpClient);
        this.orderFulfillments = createOrderFulfillmentsApi(this.httpClient);
        this.orderShipments = createOrderShipmentsApi(this.httpClient);
        this.orderRecharges = createOrderRechargesApi(this.httpClient);
        this.orderMemberships = createOrderMembershipsApi(this.httpClient);
        this.orderWithdrawals = createOrderWithdrawalsApi(this.httpClient);
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
    return new SdkworkAppClient(config);
}
export default SdkworkAppClient;
//# sourceMappingURL=sdk.js.map