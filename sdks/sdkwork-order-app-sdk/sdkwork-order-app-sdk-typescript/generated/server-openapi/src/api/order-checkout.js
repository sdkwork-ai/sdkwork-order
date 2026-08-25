import { appApiPath } from './paths';
export class OrderCheckoutCheckoutSessionsOrdersApi {
    client;
    constructor(client) {
        this.client = client;
    }
    /** Checkout sessions orders create. */
    async create(checkoutSessionId, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/checkout/sessions/${serializePathParameter(checkoutSessionId, { name: 'checkoutSessionId', style: 'simple', explode: false })}/orders`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
}
export class OrderCheckoutCheckoutSessionsQuotesApi {
    client;
    constructor(client) {
        this.client = client;
    }
    /** Checkout sessions quotes create. */
    async create(checkoutSessionId, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/checkout/sessions/${serializePathParameter(checkoutSessionId, { name: 'checkoutSessionId', style: 'simple', explode: false })}/quotes`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
}
export class OrderCheckoutCheckoutSessionsApi {
    client;
    quotes;
    orders;
    constructor(client) {
        this.client = client;
        this.quotes = new OrderCheckoutCheckoutSessionsQuotesApi(client);
        this.orders = new OrderCheckoutCheckoutSessionsOrdersApi(client);
    }
    /** Checkout sessions create. */
    async create(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/checkout/sessions`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    /** Checkout sessions retrieve. */
    async retrieve(checkoutSessionId, requestOptions) {
        return this.client.request(appApiPath(`/checkout/sessions/${serializePathParameter(checkoutSessionId, { name: 'checkoutSessionId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'item' });
    }
}
export class OrderCheckoutCheckoutApi {
    sessions;
    constructor(client) {
        this.sessions = new OrderCheckoutCheckoutSessionsApi(client);
    }
}
export class OrderCheckoutApi {
    checkout;
    constructor(client) {
        this.checkout = new OrderCheckoutCheckoutApi(client);
    }
}
export function createOrderCheckoutApi(client) {
    return new OrderCheckoutApi(client);
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
//# sourceMappingURL=order-checkout.js.map