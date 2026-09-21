import {
  createClient as createGeneratedAppClient,
  SdkworkAppClient as GeneratedSdkworkAppClient,
} from '../generated/server-openapi/src/index';
import type { SdkworkAppConfig } from '../generated/server-openapi/src/types/common';
import { applySdkworkIdempotencyRequestFingerprint } from './idempotency-request-fingerprint';

interface RequestInterceptorRegistrar {
  addRequestInterceptor(
    interceptor: typeof applySdkworkIdempotencyRequestFingerprint,
  ): () => void;
}

export { createGeneratedAppClient };
export type { SdkworkAppConfig };
export * from '../generated/server-openapi/src/types';
export * from '../generated/server-openapi/src/api';
export * from '../generated/server-openapi/src/http';
export * from '../generated/server-openapi/src/auth';

// NOTE: `recharges` and `withdrawals` are intentionally NOT re-declared here.
// The generated base class already exposes both under exactly these names, so a
// subclass field declaration would only add a bare (undefined) own property that
// shadows the base value after `super()` runs — `target: ES2022` implies
// `useDefineForClassFields: true`, which defines such a field as `undefined`.
// A `this.recharges = this.recharges` initializer then re-assigns that
// `undefined`, so `client.recharges.packages` threw
// "Cannot read properties of undefined (reading 'packages')" on the wallet page.
export class SdkworkAppClient extends GeneratedSdkworkAppClient {
  public readonly afterSales: GeneratedSdkworkAppClient["orderAfterSales"]["afterSales"];
  public readonly checkout: GeneratedSdkworkAppClient["orderCheckout"]["checkout"];
  public readonly fulfillments: GeneratedSdkworkAppClient["orderFulfillments"]["fulfillments"];
  public readonly memberships: GeneratedSdkworkAppClient["orderMemberships"]["memberships"];
  public readonly orders: GeneratedSdkworkAppClient["orderOrders"]["orders"];
  public readonly payments: GeneratedSdkworkAppClient["orderPayments"]["payments"];
  public readonly shipments: GeneratedSdkworkAppClient["orderShipments"]["shipments"];

  constructor(config: SdkworkAppConfig) {
    super(config);
    this.afterSales = this.orderAfterSales.afterSales;
    this.checkout = this.orderCheckout.checkout;
    this.fulfillments = this.orderFulfillments.fulfillments;
    this.memberships = this.orderMemberships.memberships;
    this.orders = this.orderOrders.orders;
    this.payments = this.orderPayments.payments;
    this.shipments = this.orderShipments.shipments;
    (this.http as unknown as RequestInterceptorRegistrar)
      .addRequestInterceptor(applySdkworkIdempotencyRequestFingerprint);
  }
}

export function createClient(config: SdkworkAppConfig): SdkworkAppClient {
  return new SdkworkAppClient(config);
}
