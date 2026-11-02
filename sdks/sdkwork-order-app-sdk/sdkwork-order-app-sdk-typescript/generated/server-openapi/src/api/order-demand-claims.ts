import { appApiPath } from './paths';
import type { ApiRequestOptions, HttpClient } from '../http/client';

import type { DemandClaim, DemandClaimActionCommand, DemandClaimCreateCommand, DemandClaimEvent, DemandClaimStatistics, PageInfo } from '../types';


export class OrderDemandClaimsDemandClaimsEventsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List a demand claim timeline */
  async list(claimId: string, requestOptions?: ApiRequestOptions): Promise<{ items: DemandClaimEvent[]; pageInfo: PageInfo; }> {
    return this.client.request<{ items: DemandClaimEvent[]; pageInfo: PageInfo; }>(appApiPath(`/orders/demand_claims/${serializePathParameter(claimId, { name: 'claimId', style: 'simple', explode: false })}/events`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }
}

export class OrderDemandClaimsDemandClaimsStatisticsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Demand claim tab counters */
  async retrieve(requestOptions?: ApiRequestOptions): Promise<DemandClaimStatistics> {
    return this.client.request<DemandClaimStatistics>(appApiPath(`/orders/demand_claims/statistics`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export interface OrderDemandClaimsDemandClaimsListParams {
  status?: 'submitted' | 'shortlisted' | 'accepted' | 'in_delivery' | 'delivered' | 'completed' | 'rejected' | 'withdrawn' | 'cancelled';
  statusGroup?: 'in_progress' | 'completed' | 'closed';
  demandId?: string;
  role?: 'provider' | 'publisher';
  page?: number;
  pageSize?: number;
}

export class OrderDemandClaimsDemandClaimsApi {
  private client: HttpClient;
  public readonly statistics: OrderDemandClaimsDemandClaimsStatisticsApi;
  public readonly events: OrderDemandClaimsDemandClaimsEventsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.statistics = new OrderDemandClaimsDemandClaimsStatisticsApi(client);
    this.events = new OrderDemandClaimsDemandClaimsEventsApi(client);
  }


/** List my demand claims */
  async list(params?: OrderDemandClaimsDemandClaimsListParams, requestOptions?: ApiRequestOptions): Promise<{ items: DemandClaim[]; pageInfo: PageInfo; }> {
    const query = buildQueryString([
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
      { name: 'status_group', value: params?.statusGroup, style: 'form', explode: true, allowReserved: false },
      { name: 'demand_id', value: params?.demandId, style: 'form', explode: true, allowReserved: false },
      { name: 'role', value: params?.role, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<{ items: DemandClaim[]; pageInfo: PageInfo; }>(appendQueryString(appApiPath(`/orders/demand_claims`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Submit a demand claim */
  async create(body: DemandClaimCreateCommand, requestOptions?: ApiRequestOptions): Promise<DemandClaim> {
    return this.client.request<DemandClaim>(appApiPath(`/orders/demand_claims`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve a demand claim */
  async retrieve(claimId: string, requestOptions?: ApiRequestOptions): Promise<DemandClaim> {
    return this.client.request<DemandClaim>(appApiPath(`/orders/demand_claims/${serializePathParameter(claimId, { name: 'claimId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Withdraw a claim */
  async withdraw(claimId: string, body: DemandClaimActionCommand, requestOptions?: ApiRequestOptions): Promise<DemandClaim> {
    return this.client.request<DemandClaim>(appApiPath(`/orders/demand_claims/${serializePathParameter(claimId, { name: 'claimId', style: 'simple', explode: false })}/withdraw`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Start claim delivery */
  async start(claimId: string, body: DemandClaimActionCommand, requestOptions?: ApiRequestOptions): Promise<DemandClaim> {
    return this.client.request<DemandClaim>(appApiPath(`/orders/demand_claims/${serializePathParameter(claimId, { name: 'claimId', style: 'simple', explode: false })}/start`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Submit claim delivery */
  async deliver(claimId: string, body: DemandClaimActionCommand, requestOptions?: ApiRequestOptions): Promise<DemandClaim> {
    return this.client.request<DemandClaim>(appApiPath(`/orders/demand_claims/${serializePathParameter(claimId, { name: 'claimId', style: 'simple', explode: false })}/deliver`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Shortlist a claim */
  async shortlist(claimId: string, body: DemandClaimActionCommand, requestOptions?: ApiRequestOptions): Promise<DemandClaim> {
    return this.client.request<DemandClaim>(appApiPath(`/orders/demand_claims/${serializePathParameter(claimId, { name: 'claimId', style: 'simple', explode: false })}/shortlist`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Award a claim */
  async accept(claimId: string, body: DemandClaimActionCommand, requestOptions?: ApiRequestOptions): Promise<DemandClaim> {
    return this.client.request<DemandClaim>(appApiPath(`/orders/demand_claims/${serializePathParameter(claimId, { name: 'claimId', style: 'simple', explode: false })}/accept`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Reject a claim */
  async reject(claimId: string, body: DemandClaimActionCommand, requestOptions?: ApiRequestOptions): Promise<DemandClaim> {
    return this.client.request<DemandClaim>(appApiPath(`/orders/demand_claims/${serializePathParameter(claimId, { name: 'claimId', style: 'simple', explode: false })}/reject`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Accept claim delivery */
  async complete(claimId: string, body: DemandClaimActionCommand, requestOptions?: ApiRequestOptions): Promise<DemandClaim> {
    return this.client.request<DemandClaim>(appApiPath(`/orders/demand_claims/${serializePathParameter(claimId, { name: 'claimId', style: 'simple', explode: false })}/complete`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Request claim rework */
  async rework(claimId: string, body: DemandClaimActionCommand, requestOptions?: ApiRequestOptions): Promise<DemandClaim> {
    return this.client.request<DemandClaim>(appApiPath(`/orders/demand_claims/${serializePathParameter(claimId, { name: 'claimId', style: 'simple', explode: false })}/rework`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Cancel an awarded claim */
  async cancel(claimId: string, body: DemandClaimActionCommand, requestOptions?: ApiRequestOptions): Promise<DemandClaim> {
    return this.client.request<DemandClaim>(appApiPath(`/orders/demand_claims/${serializePathParameter(claimId, { name: 'claimId', style: 'simple', explode: false })}/cancel`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class OrderDemandClaimsApi {
  public readonly demandClaims: OrderDemandClaimsDemandClaimsApi;

  constructor(client: HttpClient) {
    this.demandClaims = new OrderDemandClaimsDemandClaimsApi(client);
  }

}

export function createOrderDemandClaimsApi(client: HttpClient): OrderDemandClaimsApi {
  return new OrderDemandClaimsApi(client);
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
