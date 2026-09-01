import { appApiPath } from './paths';
import type { ApiRequestOptions, HttpClient } from '../http/client';

import type { CommerceOperationCommand, CouponRedemptionCreateCommand, CouponRedemptionResult, OrderPaymentSuccess, OrdersPaymentsWebhooksReceiveRequest, OrdersRefundsWebhooksReceiveRequest, SdkWorkCommandData, SdkWorkPageData } from '../types';


export class OrderOrdersOrdersRefundsWebhooksApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Receive PSP refund webhook */
  async receive(providerCode: string, body: OrdersRefundsWebhooksReceiveRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData> {
    return this.client.request<SdkWorkCommandData>(appApiPath(`/orders/refunds/webhooks/${serializePathParameter(providerCode, { name: 'providerCode', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', skipAuth: true, sdkworkUnwrapKind: 'command' });
  }
}

export class OrderOrdersOrdersRefundsApi {
  public readonly webhooks: OrderOrdersOrdersRefundsWebhooksApi;

  constructor(client: HttpClient) {
    this.webhooks = new OrderOrdersOrdersRefundsWebhooksApi(client);
  }

}

export interface OrderOrdersOrdersReceiptsCreateParams {
  idempotencyKey: string;
}

export class OrderOrdersOrdersReceiptsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Orders receipt confirmations create. */
  async create(orderId: string, params: OrderOrdersOrdersReceiptsCreateParams, body?: CommerceOperationCommand, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<SdkWorkCommandData>(appApiPath(`/orders/${serializePathParameter(orderId, { name: 'orderId', style: 'simple', explode: false })}/receipt_confirmations`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, ...(body !== undefined ? { body, contentType: 'application/json' } : {}), ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'command' });
  }
}

export interface OrderOrdersOrdersCouponRedemptionsCreateParams {
  idempotencyKey: string;
}

export class OrderOrdersOrdersCouponRedemptionsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Redeem a coupon into Token Bank credit or a quota-limited subscription. */
  async create(body: CouponRedemptionCreateCommand, params: OrderOrdersOrdersCouponRedemptionsCreateParams, requestOptions?: ApiRequestOptions): Promise<CouponRedemptionResult> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<CouponRedemptionResult>(appApiPath(`/orders/coupon_redemptions`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
  }
}

export class OrderOrdersOrdersStatusApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Orders status retrieve. */
  async retrieve(orderId: string, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>> {
    return this.client.request<Record<string, unknown>>(appApiPath(`/orders/${serializePathParameter(orderId, { name: 'orderId', style: 'simple', explode: false })}/status`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class OrderOrdersOrdersStatisticsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Orders statistics retrieve. */
  async retrieve(requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>> {
    return this.client.request<Record<string, unknown>>(appApiPath(`/orders/statistics`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class OrderOrdersOrdersPaymentSuccessApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Orders payment Success retrieve. */
  async retrieve(orderId: string, requestOptions?: ApiRequestOptions): Promise<OrderPaymentSuccess> {
    return this.client.request<OrderPaymentSuccess>(appApiPath(`/orders/${serializePathParameter(orderId, { name: 'orderId', style: 'simple', explode: false })}/payment_success`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export interface OrderOrdersOrdersCancellationsCreateParams {
  idempotencyKey: string;
}

export class OrderOrdersOrdersCancellationsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Orders cancellations create. */
  async create(orderId: string, params: OrderOrdersOrdersCancellationsCreateParams, body?: CommerceOperationCommand, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<SdkWorkCommandData>(appApiPath(`/orders/${serializePathParameter(orderId, { name: 'orderId', style: 'simple', explode: false })}/cancellations`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, ...(body !== undefined ? { body, contentType: 'application/json' } : {}), ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'command' });
  }
}

export interface OrderOrdersOrdersEventsListParams {
  page?: number;
  pageSize?: number;
}

export class OrderOrdersOrdersEventsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Orders events list. */
  async list(orderId: string, params?: OrderOrdersOrdersEventsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<SdkWorkPageData>(appendQueryString(appApiPath(`/orders/${serializePathParameter(orderId, { name: 'orderId', style: 'simple', explode: false })}/events`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }
}

export class OrderOrdersOrdersPaymentsWebhooksApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Receive PSP payment webhook */
  async receive(providerCode: string, body: OrdersPaymentsWebhooksReceiveRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData> {
    return this.client.request<SdkWorkCommandData>(appApiPath(`/orders/payments/webhooks/${serializePathParameter(providerCode, { name: 'providerCode', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', skipAuth: true, sdkworkUnwrapKind: 'command' });
  }
}

export interface OrderOrdersOrdersPaymentsCreateParams {
  idempotencyKey: string;
}

export class OrderOrdersOrdersPaymentsApi {
  private client: HttpClient;
  public readonly webhooks: OrderOrdersOrdersPaymentsWebhooksApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.webhooks = new OrderOrdersOrdersPaymentsWebhooksApi(client);
  }


/** Orders payments create. */
  async create(orderId: string, body: CommerceOperationCommand, params: OrderOrdersOrdersPaymentsCreateParams, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<Record<string, unknown>>(appApiPath(`/orders/${serializePathParameter(orderId, { name: 'orderId', style: 'simple', explode: false })}/payments`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
  }
}

export interface OrderOrdersOrdersListParams {
  status?: string;
  page?: number;
  pageSize?: number;
}

export class OrderOrdersOrdersApi {
  private client: HttpClient;
  public readonly payments: OrderOrdersOrdersPaymentsApi;
  public readonly events: OrderOrdersOrdersEventsApi;
  public readonly cancellations: OrderOrdersOrdersCancellationsApi;
  public readonly paymentSuccess: OrderOrdersOrdersPaymentSuccessApi;
  public readonly statistics: OrderOrdersOrdersStatisticsApi;
  public readonly status: OrderOrdersOrdersStatusApi;
  public readonly couponRedemptions: OrderOrdersOrdersCouponRedemptionsApi;
  public readonly receipts: OrderOrdersOrdersReceiptsApi;
  public readonly refunds: OrderOrdersOrdersRefundsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.payments = new OrderOrdersOrdersPaymentsApi(client);
    this.events = new OrderOrdersOrdersEventsApi(client);
    this.cancellations = new OrderOrdersOrdersCancellationsApi(client);
    this.paymentSuccess = new OrderOrdersOrdersPaymentSuccessApi(client);
    this.statistics = new OrderOrdersOrdersStatisticsApi(client);
    this.status = new OrderOrdersOrdersStatusApi(client);
    this.couponRedemptions = new OrderOrdersOrdersCouponRedemptionsApi(client);
    this.receipts = new OrderOrdersOrdersReceiptsApi(client);
    this.refunds = new OrderOrdersOrdersRefundsApi(client);
  }


/** Orders list. */
  async list(params?: OrderOrdersOrdersListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData> {
    const query = buildQueryString([
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<SdkWorkPageData>(appendQueryString(appApiPath(`/orders`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Orders retrieve. */
  async retrieve(orderId: string, requestOptions?: ApiRequestOptions): Promise<Record<string, unknown>> {
    return this.client.request<Record<string, unknown>>(appApiPath(`/orders/${serializePathParameter(orderId, { name: 'orderId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class OrderOrdersApi {
  public readonly orders: OrderOrdersOrdersApi;

  constructor(client: HttpClient) {
    this.orders = new OrderOrdersOrdersApi(client);
  }

}

export function createOrderOrdersApi(client: HttpClient): OrderOrdersApi {
  return new OrderOrdersApi(client);
}

function appendQueryString(path: string, rawQueryString: string): string {
  const query = rawQueryString.replace(/^\?+/, '');
  if (!query) {
    return path;
  }
  return path.includes('?') ? `${path}&${query}` : `${path}?${query}`;
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
interface QueryParameterSpec {
  name: string;
  value: unknown;
  style: string;
  explode: boolean;
  allowReserved: boolean;
  contentType?: string;
}

function buildQueryString(parameters: QueryParameterSpec[]): string {
  const pairs: string[] = [];
  for (const parameter of parameters) {
    appendSerializedParameter(pairs, parameter);
  }
  return pairs.join('&');
}

function appendSerializedParameter(pairs: string[], parameter: QueryParameterSpec): void {
  if (parameter.value === undefined || parameter.value === null) {
    return;
  }

  if (parameter.contentType) {
    pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(JSON.stringify(parameter.value), parameter.allowReserved)}`);
    return;
  }

  const style = parameter.style || 'form';
  if (style === 'deepObject') {
    appendDeepObjectParameter(pairs, parameter.name, parameter.value, parameter.allowReserved);
    return;
  }

  if (Array.isArray(parameter.value)) {
    appendArrayParameter(pairs, parameter.name, parameter.value, style, parameter.explode, parameter.allowReserved);
    return;
  }

  if (typeof parameter.value === 'object') {
    appendObjectParameter(pairs, parameter.name, parameter.value as Record<string, unknown>, style, parameter.explode, parameter.allowReserved);
    return;
  }

  pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(serializePrimitive(parameter.value), parameter.allowReserved)}`);
}

function appendArrayParameter(
  pairs: string[],
  name: string,
  value: unknown[],
  style: string,
  explode: boolean,
  allowReserved: boolean,
): void {
  const values = value
    .filter((item) => item !== undefined && item !== null)
    .map((item) => serializePrimitive(item));
  if (values.length === 0) {
    return;
  }

  if (style === 'form' && explode) {
    for (const item of values) {
      pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(item, allowReserved)}`);
    }
    return;
  }

  pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(values.join(','), allowReserved)}`);
}

function appendObjectParameter(
  pairs: string[],
  name: string,
  value: Record<string, unknown>,
  style: string,
  explode: boolean,
  allowReserved: boolean,
): void {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (entries.length === 0) {
    return;
  }

  if (style === 'form' && explode) {
    for (const [key, entryValue] of entries) {
      pairs.push(`${encodeQueryComponent(key)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
    }
    return;
  }

  const serialized = entries.flatMap(([key, entryValue]) => [key, serializePrimitive(entryValue)]).join(',');
  pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serialized, allowReserved)}`);
}

function appendDeepObjectParameter(
  pairs: string[],
  name: string,
  value: unknown,
  allowReserved: boolean,
): void {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serializePrimitive(value), allowReserved)}`);
    return;
  }

  for (const [key, entryValue] of Object.entries(value as Record<string, unknown>)) {
    if (entryValue === undefined || entryValue === null) {
      continue;
    }
    pairs.push(`${encodeQueryComponent(`${name}[${key}]`)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
  }
}

function serializePrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}

function encodeQueryComponent(value: string): string {
  return encodeURIComponent(value);
}

function encodeQueryValue(value: string, allowReserved: boolean): string {
  const encoded = encodeURIComponent(value);
  if (!allowReserved) {
    return encoded;
  }
  return encoded.replace(/%3A/gi, ':')
    .replace(/%2F/gi, '/')
    .replace(/%3F/gi, '?')
    .replace(/%23/gi, '#')
    .replace(/%5B/gi, '[')
    .replace(/%5D/gi, ']')
    .replace(/%40/gi, '@')
    .replace(/%21/gi, '!')
    .replace(/%24/gi, '$')
    .replace(/%26/gi, '&')
    .replace(/%27/gi, "'")
    .replace(/%28/gi, '(')
    .replace(/%29/gi, ')')
    .replace(/%2A/gi, '*')
    .replace(/%2B/gi, '+')
    .replace(/%2C/gi, ',')
    .replace(/%3B/gi, ';')
    .replace(/%3D/gi, '=');
}
function buildRequestHeaders(
  headers: Record<string, HeaderParameterSpec | undefined>,
  cookies: Record<string, HeaderParameterSpec | undefined> = {},
): Record<string, string> | undefined {
  const requestHeaders: Record<string, string> = {};

  for (const [name, parameter] of Object.entries(headers)) {
    const serialized = serializeParameterValue(parameter);
    if (serialized !== undefined) {
      requestHeaders[name] = serialized;
    }
  }

  const cookieHeader = buildCookieHeader(cookies);
  if (cookieHeader) {
    requestHeaders.Cookie = requestHeaders.Cookie
      ? `${requestHeaders.Cookie}; ${cookieHeader}`
      : cookieHeader;
  }

  return Object.keys(requestHeaders).length > 0 ? requestHeaders : undefined;
}

interface HeaderParameterSpec {
  value: unknown;
  style: string;
  explode: boolean;
  contentType?: string;
}

function buildCookieHeader(cookies: Record<string, HeaderParameterSpec | undefined>): string | undefined {
  const pairs: string[] = [];
  for (const [name, parameter] of Object.entries(cookies)) {
    const serialized = serializeParameterValue(parameter);
    if (serialized !== undefined) {
      pairs.push(`${encodeURIComponent(name)}=${encodeURIComponent(serialized)}`);
    }
  }
  return pairs.length > 0 ? pairs.join('; ') : undefined;
}

function serializeParameterValue(parameter: HeaderParameterSpec | undefined): string | undefined {
  const value = parameter?.value;
  if (value === undefined || value === null) {
    return undefined;
  }
  if (parameter?.contentType) {
    return JSON.stringify(value);
  }
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (Array.isArray(value)) {
    return value.map((item) => serializeHeaderPrimitive(item)).join(',');
  }
  if (typeof value === 'object' && value !== null) {
    return serializeHeaderObject(value as Record<string, unknown>, parameter?.explode === true);
  }
  return serializeHeaderPrimitive(value);
}

function serializeHeaderObject(value: Record<string, unknown>, explode: boolean): string {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (explode) {
    return entries.map(([key, entryValue]) => `${key}=${serializeHeaderPrimitive(entryValue)}`).join(',');
  }
  return entries.flatMap(([key, entryValue]) => [key, serializeHeaderPrimitive(entryValue)]).join(',');
}

function serializeHeaderPrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  return String(value);
}
