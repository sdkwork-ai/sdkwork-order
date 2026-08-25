import { appApiPath } from './paths';
export class OrderAfterSalesAfterSalesEventsApi {
    client;
    constructor(client) {
        this.client = client;
    }
    /** After Sales events list. */
    async list(afterSalesRequestId, params, requestOptions) {
        const query = buildQueryString([
            { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
            { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
        ]);
        return this.client.request(appendQueryString(appApiPath(`/after_sales/requests/${serializePathParameter(afterSalesRequestId, { name: 'afterSalesRequestId', style: 'simple', explode: false })}/events`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'page' });
    }
}
export class OrderAfterSalesAfterSalesReturnShipmentsApi {
    client;
    constructor(client) {
        this.client = client;
    }
    /** After Sales return Shipments list. */
    async list(afterSalesRequestId, params, requestOptions) {
        const query = buildQueryString([
            { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
            { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
            { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
        ]);
        return this.client.request(appendQueryString(appApiPath(`/after_sales/requests/${serializePathParameter(afterSalesRequestId, { name: 'afterSalesRequestId', style: 'simple', explode: false })}/return_shipments`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'page' });
    }
    /** After Sales return Shipments create. */
    async create(afterSalesRequestId, body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/after_sales/requests/${serializePathParameter(afterSalesRequestId, { name: 'afterSalesRequestId', style: 'simple', explode: false })}/return_shipments`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
}
export class OrderAfterSalesAfterSalesRequestsApi {
    client;
    constructor(client) {
        this.client = client;
    }
    /** After Sales requests list. */
    async list(params, requestOptions) {
        const query = buildQueryString([
            { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
            { name: 'order_id', value: params?.orderId, style: 'form', explode: true, allowReserved: false },
            { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
            { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
        ]);
        return this.client.request(appendQueryString(appApiPath(`/after_sales/requests`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'page' });
    }
    /** After Sales requests create. */
    async create(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/after_sales/requests`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    /** After Sales requests retrieve. */
    async retrieve(afterSalesRequestId, requestOptions) {
        return this.client.request(appApiPath(`/after_sales/requests/${serializePathParameter(afterSalesRequestId, { name: 'afterSalesRequestId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'item' });
    }
    /** After Sales requests update. */
    async update(afterSalesRequestId, body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/after_sales/requests/${serializePathParameter(afterSalesRequestId, { name: 'afterSalesRequestId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'PATCH', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
}
export class OrderAfterSalesAfterSalesApi {
    requests;
    returnShipments;
    events;
    constructor(client) {
        this.requests = new OrderAfterSalesAfterSalesRequestsApi(client);
        this.returnShipments = new OrderAfterSalesAfterSalesReturnShipmentsApi(client);
        this.events = new OrderAfterSalesAfterSalesEventsApi(client);
    }
}
export class OrderAfterSalesApi {
    afterSales;
    constructor(client) {
        this.afterSales = new OrderAfterSalesAfterSalesApi(client);
    }
}
export function createOrderAfterSalesApi(client) {
    return new OrderAfterSalesApi(client);
}
function appendQueryString(path, rawQueryString) {
    const query = rawQueryString.replace(/^\?+/, '');
    if (!query) {
        return path;
    }
    return path.includes('?') ? `${path}&${query}` : `${path}?${query}`;
}
function serializePathParameter(value, spec) {
    if (value === undefined || value === null) {
        return '';
    }
    const style = spec.style || 'simple';
    if (Array.isArray(value)) {
        return serializePathArray(spec.name, value, style, spec.explode);
    }
    if (typeof value === 'object') {
        return serializePathObject(spec.name, value, style, spec.explode);
    }
    return pathPrefix(spec.name, style, false) + encodePathValue(serializePathPrimitive(value));
}
function serializePathArray(name, values, style, explode) {
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
function serializePathObject(name, value, style, explode) {
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
function pathPrefix(name, style, _objectValue) {
    if (style === 'label')
        return '.';
    if (style === 'matrix')
        return `;${name}`;
    return '';
}
function encodePathValue(value) {
    return encodeURIComponent(value);
}
function serializePathPrimitive(value) {
    if (value instanceof Date) {
        return value.toISOString();
    }
    if (typeof value === 'object') {
        return JSON.stringify(value);
    }
    return String(value);
}
function buildQueryString(parameters) {
    const pairs = [];
    for (const parameter of parameters) {
        appendSerializedParameter(pairs, parameter);
    }
    return pairs.join('&');
}
function appendSerializedParameter(pairs, parameter) {
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
        appendObjectParameter(pairs, parameter.name, parameter.value, style, parameter.explode, parameter.allowReserved);
        return;
    }
    pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(serializePrimitive(parameter.value), parameter.allowReserved)}`);
}
function appendArrayParameter(pairs, name, value, style, explode, allowReserved) {
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
function appendObjectParameter(pairs, name, value, style, explode, allowReserved) {
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
function appendDeepObjectParameter(pairs, name, value, allowReserved) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) {
        pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serializePrimitive(value), allowReserved)}`);
        return;
    }
    for (const [key, entryValue] of Object.entries(value)) {
        if (entryValue === undefined || entryValue === null) {
            continue;
        }
        pairs.push(`${encodeQueryComponent(`${name}[${key}]`)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
    }
}
function serializePrimitive(value) {
    if (value instanceof Date) {
        return value.toISOString();
    }
    if (typeof value === 'object') {
        return JSON.stringify(value);
    }
    return String(value);
}
function encodeQueryComponent(value) {
    return encodeURIComponent(value);
}
function encodeQueryValue(value, allowReserved) {
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
function buildRequestHeaders(headers, cookies = {}) {
    const requestHeaders = {};
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
function buildCookieHeader(cookies) {
    const pairs = [];
    for (const [name, parameter] of Object.entries(cookies)) {
        const serialized = serializeParameterValue(parameter);
        if (serialized !== undefined) {
            pairs.push(`${encodeURIComponent(name)}=${encodeURIComponent(serialized)}`);
        }
    }
    return pairs.length > 0 ? pairs.join('; ') : undefined;
}
function serializeParameterValue(parameter) {
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
        return serializeHeaderObject(value, parameter?.explode === true);
    }
    return serializeHeaderPrimitive(value);
}
function serializeHeaderObject(value, explode) {
    const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
    if (explode) {
        return entries.map(([key, entryValue]) => `${key}=${serializeHeaderPrimitive(entryValue)}`).join(',');
    }
    return entries.flatMap(([key, entryValue]) => [key, serializeHeaderPrimitive(entryValue)]).join(',');
}
function serializeHeaderPrimitive(value) {
    if (value instanceof Date) {
        return value.toISOString();
    }
    return String(value);
}
//# sourceMappingURL=order-after-sales.js.map