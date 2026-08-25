import { HttpClient } from './http/client';
import type { SdkworkAppConfig } from './types/common';
import type { AuthTokenManager } from '@sdkwork/sdk-common';
import { OrderCheckoutApi } from './api/order-checkout';
import { OrderOrdersApi } from './api/order-orders';
import { OrderPaymentsApi } from './api/order-payments';
import { OrderAfterSalesApi } from './api/order-after-sales';
import { OrderFulfillmentsApi } from './api/order-fulfillments';
import { OrderShipmentsApi } from './api/order-shipments';
import { OrderRechargesApi } from './api/order-recharges';
import { OrderMembershipsApi } from './api/order-memberships';
import { OrderWithdrawalsApi } from './api/order-withdrawals';
export declare class SdkworkAppClient {
    private httpClient;
    readonly orderCheckout: OrderCheckoutApi;
    readonly orderOrders: OrderOrdersApi;
    readonly orderPayments: OrderPaymentsApi;
    readonly orderAfterSales: OrderAfterSalesApi;
    readonly orderFulfillments: OrderFulfillmentsApi;
    readonly orderShipments: OrderShipmentsApi;
    readonly orderRecharges: OrderRechargesApi;
    readonly orderMemberships: OrderMembershipsApi;
    readonly orderWithdrawals: OrderWithdrawalsApi;
    constructor(config: SdkworkAppConfig);
    setAuthToken(token: string): this;
    setAccessToken(token: string): this;
    setTokenManager(manager: AuthTokenManager): this;
    get http(): HttpClient;
}
export declare function createClient(config: SdkworkAppConfig): SdkworkAppClient;
export default SdkworkAppClient;
//# sourceMappingURL=sdk.d.ts.map