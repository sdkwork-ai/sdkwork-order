import { createClient as createGeneratedAppClient, SdkworkAppClient as GeneratedSdkworkAppClient, } from '../generated/server-openapi/src/index';
import { applySdkworkIdempotencyRequestFingerprint } from './idempotency-request-fingerprint';
export { createGeneratedAppClient };
export * from '../generated/server-openapi/src/types';
export * from '../generated/server-openapi/src/api';
export * from '../generated/server-openapi/src/http';
export * from '../generated/server-openapi/src/auth';
export class SdkworkAppClient extends GeneratedSdkworkAppClient {
    afterSales;
    checkout;
    fulfillments;
    memberships;
    orders;
    payments;
    recharges;
    shipments;
    withdrawals;
    constructor(config) {
        super(config);
        this.afterSales = this.orderAfterSales.afterSales;
        this.checkout = this.orderCheckout.checkout;
        this.fulfillments = this.orderFulfillments.fulfillments;
        this.memberships = this.orderMemberships.memberships;
        this.orders = this.orderOrders.orders;
        this.payments = this.orderPayments.payments;
        this.recharges = this.recharges;
        this.shipments = this.orderShipments.shipments;
        this.withdrawals = this.withdrawals;
        this.http
            .addRequestInterceptor(applySdkworkIdempotencyRequestFingerprint);
    }
}
export function createClient(config) {
    return new SdkworkAppClient(config);
}
//# sourceMappingURL=index.js.map