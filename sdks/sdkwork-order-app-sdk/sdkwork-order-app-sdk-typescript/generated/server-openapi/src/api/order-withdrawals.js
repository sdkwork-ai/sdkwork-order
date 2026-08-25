import { appApiPath } from './paths';
export class OrderWithdrawalsWithdrawalsRequestsApi {
    client;
    constructor(client) {
        this.client = client;
    }
    /** Withdrawal requests create. */
    async create(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/withdrawals/requests`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    /** Withdrawal requests retrieve. */
    async retrieve(withdrawalRequestId, requestOptions) {
        return this.client.request(appApiPath(`/withdrawals/requests/${serializePathParameter(withdrawalRequestId, { name: 'withdrawalRequestId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'item' });
    }
}
export class OrderWithdrawalsWithdrawalsApi {
    requests;
    constructor(client) {
        this.requests = new OrderWithdrawalsWithdrawalsRequestsApi(client);
    }
}
export class OrderWithdrawalsApi {
    withdrawals;
    constructor(client) {
        this.withdrawals = new OrderWithdrawalsWithdrawalsApi(client);
    }
}
export function createOrderWithdrawalsApi(client) {
    return new OrderWithdrawalsApi(client);
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
//# sourceMappingURL=order-withdrawals.js.map