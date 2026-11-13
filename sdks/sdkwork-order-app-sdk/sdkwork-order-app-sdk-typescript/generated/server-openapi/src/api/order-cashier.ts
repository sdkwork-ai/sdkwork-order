import { appApiPath } from './paths';
import type { ApiRequestOptions, HttpClient } from '../http/client';

import type { CashierOrder, CashierPayment, CashierPaymentCreateCommand, CashierStatus } from '../types';


export class OrderCashierCashierStatusApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Anonymous cashier payment status. */
  async retrieve(token: string, requestOptions?: ApiRequestOptions): Promise<CashierStatus> {
    return this.client.request<CashierStatus>(appApiPath(`/cashier/${serializePathParameter(token, { name: 'token', style: 'simple', explode: false })}/status`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, skipAuth: true, sdkworkUnwrapKind: 'item' });
  }
}

export class OrderCashierCashierPaymentsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a payment from an anonymous cashier link. */
  async create(token: string, body: CashierPaymentCreateCommand, requestOptions?: ApiRequestOptions): Promise<CashierPayment> {
    return this.client.request<CashierPayment>(appApiPath(`/cashier/${serializePathParameter(token, { name: 'token', style: 'simple', explode: false })}/payments`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', skipAuth: true, sdkworkUnwrapKind: 'item' });
  }
}

export class OrderCashierCashierApi {
  private client: HttpClient;
  public readonly payments: OrderCashierCashierPaymentsApi;
  public readonly status: OrderCashierCashierStatusApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.payments = new OrderCashierCashierPaymentsApi(client);
    this.status = new OrderCashierCashierStatusApi(client);
  }


/** Anonymous cashier order view. */
  async retrieve(token: string, requestOptions?: ApiRequestOptions): Promise<CashierOrder> {
    return this.client.request<CashierOrder>(appApiPath(`/cashier/${serializePathParameter(token, { name: 'token', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, skipAuth: true, sdkworkUnwrapKind: 'item' });
  }
}

export class OrderCashierApi {
  public readonly cashier: OrderCashierCashierApi;

  constructor(client: HttpClient) {
    this.cashier = new OrderCashierCashierApi(client);
  }

}

export function createOrderCashierApi(client: HttpClient): OrderCashierApi {
  return new OrderCashierApi(client);
}



interface PathParameterSpec {
  name: string;
  style: string;
  explode: boolean;
}

function serializePathParameter(value: unknown, spec: PathParameterSpec): string {
  if (value === undefined || value === null) {
    return '';
  }

  const style = spec.style || 'simple';
  if (Array.isArray(value)) {
    return serializePathArray(spec.name, value, style, spec.explode);
  }
  if (typeof value === 'object') {
    return serializePathObject(spec.name, value as Record<string, unknown>, style, spec.explode);
  }
  return pathPrefix(spec.name, style, false) + encodePathValue(serializePathPrimitive(value));
}

function serializePathArray(name: string, values: unknown[], style: string, explode: boolean): string {
  const serialized = values
    .filter((item) => item !== undefined && item !== null)
    .map((item) => encodePathValue(serializePathPrimitive(item)));
  if (serialized.length === 0) {
    return pathPrefix(name, style, false);
  }
  if (style === 'matrix') {
    return explode
      ? serialized.map((item) => `;${name}=${item}`).join('')
      : `;${name}=${serialized.join(',')}`;
  }
  return pathPrefix(name, style, false) + serialized.join(explode ? '.' : ',');
}

function serializePathObject(name: string, value: Record<string, unknown>, style: string, explode: boolean): string {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (entries.length === 0) {
    return pathPrefix(name, style, true);
  }
  if (style === 'matrix') {
    return explode
      ? entries.map(([key, entryValue]) => `;${encodePathValue(key)}=${encodePathValue(serializePathPrimitive(entryValue))}`).join('')
      : `;${name}=${entries.flatMap(([key, entryValue]) => [encodePathValue(key), encodePathValue(serializePathPrimitive(entryValue))]).join(',')}`;
  }
  const serialized = explode
    ? entries.map(([key, entryValue]) => `${encodePathValue(key)}=${encodePathValue(serializePathPrimitive(entryValue))}`).join(style === 'label' ? '.' : ',')
    : entries.flatMap(([key, entryValue]) => [encodePathValue(key), encodePathValue(serializePathPrimitive(entryValue))]).join(',');
  return pathPrefix(name, style, true) + serialized;
}

function pathPrefix(name: string, style: string, _objectValue: boolean): string {
  if (style === 'label') return '.';
  if (style === 'matrix') return `;${name}`;
  return '';
}

function encodePathValue(value: string): string {
  return encodeURIComponent(value);
}

function serializePathPrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}
