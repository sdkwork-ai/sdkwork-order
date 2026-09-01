import { HttpClient, createHttpClient } from './http/client';
import type { SdkworkAppConfig } from './types/common';
import type { AuthTokenManager } from '@sdkwork/sdk-common';

import { OrderCheckoutApi, createOrderCheckoutApi } from './api/order-checkout';
import { OrderOrdersApi, createOrderOrdersApi } from './api/order-orders';
import { OrderPaymentsApi, createOrderPaymentsApi } from './api/order-payments';
import { OrderAfterSalesApi, createOrderAfterSalesApi } from './api/order-after-sales';
import { OrderFulfillmentsApi, createOrderFulfillmentsApi } from './api/order-fulfillments';
import { OrderShipmentsApi, createOrderShipmentsApi } from './api/order-shipments';
import { RechargesApi, createRechargesApi } from './api/recharges';
import { OrderMembershipsApi, createOrderMembershipsApi } from './api/order-memberships';
import { OrdersApi, createOrdersApi } from './api/orders';
import { WithdrawalsApi, createWithdrawalsApi } from './api/withdrawals';

export class SdkworkAppClient {
  private httpClient: HttpClient;

  public readonly orderCheckout: OrderCheckoutApi;
  public readonly orderOrders: OrderOrdersApi;
  public readonly orderPayments: OrderPaymentsApi;
  public readonly orderAfterSales: OrderAfterSalesApi;
  public readonly orderFulfillments: OrderFulfillmentsApi;
  public readonly orderShipments: OrderShipmentsApi;
  public readonly recharges: RechargesApi;
  public readonly orderMemberships: OrderMembershipsApi;
  public readonly orders: OrdersApi;
  public readonly withdrawals: WithdrawalsApi;

  constructor(config: SdkworkAppConfig) {
    this.httpClient = createHttpClient(config);
    this.orderCheckout = createOrderCheckoutApi(this.httpClient);

    this.orderOrders = createOrderOrdersApi(this.httpClient);

    this.orderPayments = createOrderPaymentsApi(this.httpClient);

    this.orderAfterSales = createOrderAfterSalesApi(this.httpClient);

    this.orderFulfillments = createOrderFulfillmentsApi(this.httpClient);

    this.orderShipments = createOrderShipmentsApi(this.httpClient);

    this.recharges = createRechargesApi(this.httpClient);

    this.orderMemberships = createOrderMembershipsApi(this.httpClient);

    this.orders = createOrdersApi(this.httpClient);

    this.withdrawals = createWithdrawalsApi(this.httpClient);
  }
  setAuthToken(token: string): this {
    this.httpClient.setAuthToken(token);
    return this;
  }

  setAccessToken(token: string): this {
    this.httpClient.setAccessToken(token);
    return this;
  }

  setTokenManager(manager: AuthTokenManager): this {
    this.httpClient.setTokenManager(manager);
    return this;
  }

  get http(): HttpClient {
    return this.httpClient;
  }
}

export function createClient(config: SdkworkAppConfig): SdkworkAppClient {
  return new SdkworkAppClient(config);
}

export default SdkworkAppClient;
