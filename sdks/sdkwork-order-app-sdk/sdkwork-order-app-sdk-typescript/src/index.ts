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

export class SdkworkAppClient extends GeneratedSdkworkAppClient {
  public readonly afterSales: GeneratedSdkworkAppClient["orderAfterSales"]["afterSales"];
  public readonly checkout: GeneratedSdkworkAppClient["orderCheckout"]["checkout"];
  public readonly fulfillments: GeneratedSdkworkAppClient["orderFulfillments"]["fulfillments"];
  public readonly memberships: GeneratedSdkworkAppClient["orderMemberships"]["memberships"];
  public readonly orders: GeneratedSdkworkAppClient["orderOrders"]["orders"];
  public readonly payments: GeneratedSdkworkAppClient["orderPayments"]["payments"];
  public readonly recharges: GeneratedSdkworkAppClient["recharges"];
  public readonly shipments: GeneratedSdkworkAppClient["orderShipments"]["shipments"];
  public readonly withdrawals: GeneratedSdkworkAppClient["withdrawals"];

  constructor(config: SdkworkAppConfig) {
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
    (this.http as unknown as RequestInterceptorRegistrar)
      .addRequestInterceptor(applySdkworkIdempotencyRequestFingerprint);
  }
}

export function createClient(config: SdkworkAppConfig): SdkworkAppClient {
  return new SdkworkAppClient(config);
}
