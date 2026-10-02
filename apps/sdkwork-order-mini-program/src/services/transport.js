/**
 * Single transport seam for the mini-program.
 *
 * Every network call in the app funnels through `request()` so envelope
 * unwrapping, auth headers, idempotency headers, timeouts, and problem+json
 * error mapping live in exactly one place. Pages and services never call the
 * raw platform request API (enforced by tests and lint).
 *
 * Wire contract (API_SPEC.md section 4.5/14/15):
 * - success: SdkWorkApiResponse `{ code: 0, data, traceId }` → resolves `data`
 *   (`data.item` / `data.items + pageInfo` / `data.accepted` stay intact for
 *   the domain services to unwrap);
 * - error: HTTP 4xx/5xx `application/problem+json` → rejects an Error carrying
 *   numeric `.code`, `.traceId`, `.statusCode` and `problem.errors`.
 */
const session = require("./session");

const DEFAULT_TIMEOUT_MS = 15000;

function getBaseUrl() {
  const app = getApp();
  return (app && app.globalData && app.globalData.orderApiBaseUrl) || "https://api-dev.sdkwork.com/app/v3/api";
}

function uuid() {
  // RFC 4122 v4 shape from Math.random: sufficient for client-side
  // Idempotency-Key generation (uniqueness within one payment/recharge
  // attempt); the server treats the key as opaque text (1..128 chars).
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (char) => {
    const random = Math.floor(Math.random() * 16);
    const value = char === "x" ? random : (random & 0x3) | 0x8;
    return value.toString(16);
  });
}

function appendQuery(url, query) {
  if (!query) {
    return url;
  }
  const search = Object.keys(query)
    .filter((key) => query[key] !== undefined && query[key] !== null && query[key] !== "")
    .map((key) => `${encodeURIComponent(key)}=${encodeURIComponent(query[key])}`)
    .join("&");
  return search ? `${url}?${search}` : url;
}

/**
 * Performs one request against the order app-api.
 *
 * options:
 *   path          required, app-api path beginning with "/"
 *   method        GET (default) | POST | PATCH | PUT | DELETE
 *   body          JSON body (commands)
 *   query         query params (page/page_size/cursor/status/...)
 *   headers       extra headers; write commands pass Idempotency-Key here
 *   timeoutMs     per-attempt timeout, default 15000
 */
function request(options) {
  const { path, method = "GET", body, query, headers, timeoutMs = DEFAULT_TIMEOUT_MS } = options;
  const url = appendQuery(`${getBaseUrl()}${path}`, query);

  const header = { "content-type": "application/json" };
  const token = session.getToken();
  if (token) {
    header.authorization = `Bearer ${token}`;
  }
  if (headers) {
    Object.keys(headers).forEach((name) => {
      if (headers[name] !== undefined && headers[name] !== null && headers[name] !== "") {
        header[name.toLowerCase()] = String(headers[name]);
      }
    });
  }

  return new Promise((resolve, reject) => {
    wx.request({
      url,
      method,
      data: body,
      header,
      timeout: timeoutMs,
      success(res) {
        if (res.statusCode >= 200 && res.statusCode < 300) {
          const payload = res.data;
          if (payload && typeof payload === "object" && "code" in payload) {
            if (Number(payload.code) === 0) {
              resolve(payload.data);
              return;
            }
            // Spec-forbidden legacy shape; map it anyway so the page shows a
            // typed numeric code instead of an opaque failure.
            const error = new Error(payload.message || `请求失败（${payload.code}）`);
            error.code = payload.code;
            error.traceId = payload.traceId;
            error.statusCode = res.statusCode;
            reject(error);
            return;
          }
          resolve(payload);
          return;
        }
        const problem = res.data && typeof res.data === "object" ? res.data : {};
        const error = new Error(
          problem.detail || problem.title || `请求失败（HTTP ${res.statusCode}）`,
        );
        error.code = problem.code ?? res.statusCode;
        error.traceId = problem.traceId;
        error.statusCode = res.statusCode;
        error.errors = Array.isArray(problem.errors) ? problem.errors : [];
        reject(error);
      },
      fail(cause) {
        const error = new Error(cause && cause.errMsg ? `网络请求失败：${cause.errMsg}` : "网络请求失败");
        error.cause = cause;
        reject(error);
      },
    });
  });
}

/**
 * Unwraps a resource payload (`data.item`) or tolerates an already-unwrapped
 * record, mirroring `unwrapSdkworkOrderResource` in @sdkwork/order-service so
 * a bare `{ item }` wrapper can never be read as the record itself.
 */
function unwrapResource(payload) {
  if (payload && typeof payload === "object" && !Array.isArray(payload) && "item" in payload) {
    return payload.item;
  }
  return payload;
}

/**
 * Unwraps a list payload (`data.items + data.pageInfo`) into
 * `{ items, pageInfo }`, keeping pageInfo.totalItems as the raw int64 string.
 */
function unwrapList(payload) {
  const source = payload && typeof payload === "object" ? payload : {};
  const items = Array.isArray(source.items) ? source.items : [];
  const rawPageInfo = source.pageInfo && typeof source.pageInfo === "object" ? source.pageInfo : {};
  return {
    items,
    pageInfo: {
      mode: rawPageInfo.mode === "cursor" ? "cursor" : "offset",
      page: rawPageInfo.page,
      pageSize: rawPageInfo.pageSize,
      totalItems: rawPageInfo.totalItems !== undefined && rawPageInfo.totalItems !== null
        ? String(rawPageInfo.totalItems)
        : undefined,
      totalPages: rawPageInfo.totalPages,
      nextCursor: rawPageInfo.nextCursor ?? null,
      hasMore: rawPageInfo.hasMore,
    },
  };
}

module.exports = { request, unwrapResource, unwrapList, uuid };
