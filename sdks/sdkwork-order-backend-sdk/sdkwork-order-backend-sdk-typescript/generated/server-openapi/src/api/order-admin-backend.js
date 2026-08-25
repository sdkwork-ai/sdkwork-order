import { backendApiPath } from './paths';
export class OrderAdminBackendBackendWithdrawalRequestsApi {
    client;
    constructor(client) {
        this.client = client;
    }
    /** Withdrawal requests list. */
    async list(params, requestOptions) {
        const query = buildQueryString([
            { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
            { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
            { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
        ]);
        return this.client.request(appendQueryString(backendApiPath(`/withdrawal_requests`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'page' });
    }
    /** Withdrawal requests approve. */
    async approve(withdrawalRequestId, params, body, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(backendApiPath(`/withdrawal_requests/${serializePathParameter(withdrawalRequestId, { name: 'withdrawalRequestId', style: 'simple', explode: false })}/approve`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', ...(body !== undefined ? { body, contentType: 'application/json' } : {}), ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'command' });
    }
    /** Withdrawal requests reject. */
    async reject(withdrawalRequestId, params, body, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(backendApiPath(`/withdrawal_requests/${serializePathParameter(withdrawalRequestId, { name: 'withdrawalRequestId', style: 'simple', explode: false })}/reject`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', ...(body !== undefined ? { body, contentType: 'application/json' } : {}), ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'command' });
    }
    /** Withdrawal requests retry. */
    async retry(withdrawalRequestId, params, body, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(backendApiPath(`/withdrawal_requests/${serializePathParameter(withdrawalRequestId, { name: 'withdrawalRequestId', style: 'simple', explode: false })}/retry`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', ...(body !== undefined ? { body, contentType: 'application/json' } : {}), ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'command' });
    }
}
export class OrderAdminBackendBackendRefundRequestsApi {
    client;
    constructor(client) {
        this.client = client;
    }
    /** Refund requests list. */
    async list(params, requestOptions) {
        const query = buildQueryString([
            { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
            { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
            { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
        ]);
        return this.client.request(appendQueryString(backendApiPath(`/refund_requests`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'page' });
    }
    /** Refund requests approve. */
    async approve(refundRequestId, params, body, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(backendApiPath(`/refund_requests/${serializePathParameter(refundRequestId, { name: 'refundRequestId', style: 'simple', explode: false })}/approve`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', ...(body !== undefined ? { body, contentType: 'application/json' } : {}), ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'command' });
    }
    /** Refund requests reject. */
    async reject(refundRequestId, params, body, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(backendApiPath(`/refund_requests/${serializePathParameter(refundRequestId, { name: 'refundRequestId', style: 'simple', explode: false })}/reject`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', ...(body !== undefined ? { body, contentType: 'application/json' } : {}), ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'command' });
    }
    /** Refund requests retry. */
    async retry(refundRequestId, params, body, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(backendApiPath(`/refund_requests/${serializePathParameter(refundRequestId, { name: 'refundRequestId', style: 'simple', explode: false })}/retry`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', ...(body !== undefined ? { body, contentType: 'application/json' } : {}), ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'command' });
    }
}
export class OrderAdminBackendBackendTokenBankPlansApi {
    client;
    constructor(client) {
        this.client = client;
    }
    /** Token Bank plans list. */
    async list(params, requestOptions) {
        const query = buildQueryString([
            { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
            { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
            { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
        ]);
        return this.client.request(appendQueryString(backendApiPath(`/token_bank_plans`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'page' });
    }
    /** Token Bank plans create. */
    async create(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(backendApiPath(`/token_bank_plans`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    /** Token Bank plans update. */
    async update(planCode, body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(backendApiPath(`/token_bank_plans/${serializePathParameter(planCode, { name: 'planCode', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'PATCH', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    /** Token Bank plans retire. */
    async retire(planCode, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(backendApiPath(`/token_bank_plans/${serializePathParameter(planCode, { name: 'planCode', style: 'simple', explode: false })}/retire`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'command' });
    }
}
export class OrderAdminBackendBackendAccountValuePackagesApi {
    client;
    constructor(client) {
        this.client = client;
    }
    /** Account value packages list. */
    async list(params, requestOptions) {
        const query = buildQueryString([
            { name: 'target_asset', value: params?.targetAsset, style: 'form', explode: true, allowReserved: false },
            { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
            { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
            { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
        ]);
        return this.client.request(appendQueryString(backendApiPath(`/account_value_packages`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'page' });
    }
    /** Account value packages create. */
    async create(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(backendApiPath(`/account_value_packages`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    /** Account value packages update. */
    async update(packageId, body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(backendApiPath(`/account_value_packages/${serializePathParameter(packageId, { name: 'packageId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'PATCH', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    /** Account value packages retire. */
    async retire(packageId, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(backendApiPath(`/account_value_packages/${serializePathParameter(packageId, { name: 'packageId', style: 'simple', explode: false })}/retire`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'command' });
    }
}
export class OrderAdminBackendBackendApi {
    accountValuePackages;
    tokenBankPlans;
    refundRequests;
    withdrawalRequests;
    constructor(client) {
        this.accountValuePackages = new OrderAdminBackendBackendAccountValuePackagesApi(client);
        this.tokenBankPlans = new OrderAdminBackendBackendTokenBankPlansApi(client);
        this.refundRequests = new OrderAdminBackendBackendRefundRequestsApi(client);
        this.withdrawalRequests = new OrderAdminBackendBackendWithdrawalRequestsApi(client);
    }
}
export class OrderAdminBackendApi {
    backend;
    constructor(client) {
        this.backend = new OrderAdminBackendBackendApi(client);
    }
}
export function createOrderAdminBackendApi(client) {
    return new OrderAdminBackendApi(client);
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
//# sourceMappingURL=order-admin-backend.js.map