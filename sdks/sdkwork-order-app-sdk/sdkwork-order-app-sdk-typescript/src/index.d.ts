import { createClient as createGeneratedAppClient, SdkworkAppClient as GeneratedSdkworkAppClient } from '../generated/server-openapi/src/index';
import type { SdkworkAppConfig } from '../generated/server-openapi/src/types/common';
export { createGeneratedAppClient };
export type { SdkworkAppConfig };
export * from '../generated/server-openapi/src/types';
export * from '../generated/server-openapi/src/api';
export * from '../generated/server-openapi/src/http';
export * from '../generated/server-openapi/src/auth';
export declare class SdkworkAppClient extends GeneratedSdkworkAppClient {
    readonly afterSales: GeneratedSdkworkAppClient["orderAfterSales"]["afterSales"];
    readonly checkout: GeneratedSdkworkAppClient["orderCheckout"]["checkout"];
    readonly fulfillments: GeneratedSdkworkAppClient["orderFulfillments"]["fulfillments"];
    readonly memberships: GeneratedSdkworkAppClient["orderMemberships"]["memberships"];
    readonly orders: GeneratedSdkworkAppClient["orderOrders"]["orders"];
    readonly payments: GeneratedSdkworkAppClient["orderPayments"]["payments"];
    readonly recharges: GeneratedSdkworkAppClient["orderRecharges"]["recharges"];
    readonly shipments: GeneratedSdkworkAppClient["orderShipments"]["shipments"];
    readonly withdrawals: GeneratedSdkworkAppClient["orderWithdrawals"]["withdrawals"];
    constructor(config: SdkworkAppConfig);
}
export declare function createClient(config: SdkworkAppConfig): SdkworkAppClient;
//# sourceMappingURL=index.d.ts.map