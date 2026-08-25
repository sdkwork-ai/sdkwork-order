import { sha256Hash } from '@sdkwork/utils';
const CONTENT_SHA256_HEADER = 'X-Content-SHA256';
const IDEMPOTENCY_FINGERPRINT_HEADER = 'X-Idempotency-Fingerprint';
const IDEMPOTENCY_KEY_HEADER = 'Idempotency-Key';
export async function applySdkworkIdempotencyRequestFingerprint(config) {
    if (config.body == null
        || !hasNonEmptyHeader(config.headers, IDEMPOTENCY_KEY_HEADER)
        || hasNonEmptyHeader(config.headers, CONTENT_SHA256_HEADER)
        || hasNonEmptyHeader(config.headers, IDEMPOTENCY_FINGERPRINT_HEADER)) {
        return config;
    }
    const fingerprint = await createRequestBodyFingerprint(config.body);
    if (!fingerprint) {
        return config;
    }
    return {
        ...config,
        headers: {
            ...withoutHeaders(config.headers, [CONTENT_SHA256_HEADER, IDEMPOTENCY_FINGERPRINT_HEADER]),
            [fingerprint.header]: fingerprint.value,
        },
    };
}
function hasNonEmptyHeader(headers, name) {
    const normalizedName = name.toLowerCase();
    return Object.entries(headers ?? {}).some(([headerName, value]) => headerName.toLowerCase() === normalizedName && value.trim().length > 0);
}
function withoutHeaders(headers, names) {
    const normalizedNames = new Set(names.map((name) => name.toLowerCase()));
    return Object.fromEntries(Object.entries(headers ?? {}).filter(([headerName]) => !normalizedNames.has(headerName.toLowerCase())));
}
async function createRequestBodyFingerprint(body) {
    if (typeof FormData !== 'undefined' && body instanceof FormData) {
        return {
            header: IDEMPOTENCY_FINGERPRINT_HEADER,
            value: await sha256Hex(new TextEncoder().encode(await serializeFormData(body))),
        };
    }
    const bytes = await serializeRequestBodyBytes(body);
    if (!bytes) {
        return undefined;
    }
    return {
        header: CONTENT_SHA256_HEADER,
        value: await sha256Hex(bytes),
    };
}
async function serializeRequestBodyBytes(body) {
    if (typeof URLSearchParams !== 'undefined' && body instanceof URLSearchParams) {
        return new TextEncoder().encode(body.toString());
    }
    if (typeof Blob !== 'undefined' && body instanceof Blob) {
        return new Uint8Array(await body.arrayBuffer());
    }
    if (typeof ArrayBuffer !== 'undefined' && body instanceof ArrayBuffer) {
        return new Uint8Array(body.slice(0));
    }
    if (typeof ArrayBuffer !== 'undefined' && ArrayBuffer.isView(body)) {
        const bytes = new Uint8Array(body.byteLength);
        bytes.set(new Uint8Array(body.buffer, body.byteOffset, body.byteLength));
        return bytes;
    }
    if (typeof body === 'string') {
        return new TextEncoder().encode(body);
    }
    const serialized = JSON.stringify(body);
    return serialized === undefined ? undefined : new TextEncoder().encode(serialized);
}
async function serializeFormData(body) {
    const parts = [];
    for (const [name, value] of body.entries()) {
        if (typeof value === 'string') {
            parts.push({ kind: 'field', name, value });
            continue;
        }
        parts.push({
            kind: 'file',
            name,
            fileName: 'name' in value ? String(value.name) : '',
            contentType: value.type,
            size: value.size,
            contentSha256: await sha256Hex(new Uint8Array(await value.arrayBuffer())),
        });
    }
    return JSON.stringify(parts);
}
async function sha256Hex(bytes) {
    return sha256Hash(bytes);
}
//# sourceMappingURL=idempotency-request-fingerprint.js.map