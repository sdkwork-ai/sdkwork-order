import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import http from "node:http";
import https from "node:https";

const require = createRequire(import.meta.url);

/**
 * Live-gateway transport integration tests.
 *
 * Skipped unless `SDKWORK_ORDER_GATEWAY_BASE_URL` points at a running order
 * standalone gateway (for example `http://127.0.0.1:3901`). A minimal `wx`
 * shim bridges `wx.request` onto node http/https so the production
 * transport seam — the app's single request seam — runs against the real
 * wire: `{code,data,traceId}` unwrapping, seeded public reads, and
 * problem+json mapping for auth failures.
 */

const gatewayBaseUrl = (process.env.SDKWORK_ORDER_GATEWAY_BASE_URL || "").trim();

if (!gatewayBaseUrl) {
  test("gateway integration skipped without SDKWORK_ORDER_GATEWAY_BASE_URL", () => {
    // A named skip keeps `npm test` green in offline runs.
    assert.ok(true, "set SDKWORK_ORDER_GATEWAY_BASE_URL to run against a live gateway");
  });
} else {
  const storage = new Map();
  globalThis.wx = {
    getStorageSync(key) {
      return storage.get(key) ?? "";
    },
    setStorageSync(key, value) {
      storage.set(key, value);
    },
    removeStorageSync(key) {
      storage.delete(key);
    },
    request(options) {
      const { url, method = "GET", data, header = {}, timeout = 15000 } = options;
      const target = new URL(url);
      const client = target.protocol === "https:" ? https : http;
      const payload = data === undefined || data === null ? null : JSON.stringify(data);
      const request = client.request(
        target,
        {
          method,
          headers: {
            ...header,
            ...(payload !== null ? { "content-length": Buffer.byteLength(payload) } : {}),
          },
          timeout,
        },
        (response) => {
          const chunks = [];
          response.on("data", (chunk) => chunks.push(chunk));
          response.on("end", () => {
            const text = Buffer.concat(chunks).toString("utf8");
            let parsed = text;
            try {
              parsed = text ? JSON.parse(text) : "";
            } catch {
              // non-JSON body: pass the raw text through like wx would
            }
            options.success({ statusCode: response.statusCode || 0, data: parsed });
          });
        },
      );
      request.on("timeout", () => {
        request.destroy(new Error("request timed out"));
      });
      request.on("error", (cause) => {
        options.fail({ errMsg: cause && cause.message ? cause.message : String(cause) });
      });
      if (payload !== null) {
        request.write(payload);
      }
      request.end();
    },
  };
  globalThis.getApp = () => ({ globalData: { orderApiBaseUrl: `${gatewayBaseUrl}/app/v3/api` } });

  const transport = require("../src/services/transport.js");

  test("public recharge plans read returns seeded items over the real wire", async () => {
    const data = await transport.request({
      path: "/recharges/plans",
      query: { page: 1, page_size: 3 },
    });
    assert.ok(Array.isArray(data.items), "data.items is a list");
    assert.ok(data.items.length > 0, "seeded plans are present");
    const first = data.items[0];
    assert.equal(typeof first.planCode, "string");
    // Money/points cross the wire as strings (int64-safe), never numbers.
    assert.equal(typeof first.priceAmount, "string");
  });

  test("unauthenticated order list is rejected as problem+json 401", async () => {
    await assert.rejects(
      transport.request({ path: "/orders", query: { page: 1, page_size: 1 } }),
      (error) => {
        assert.equal(error.statusCode, 401);
        assert.ok(error.code !== undefined && error.code !== null, "numeric problem code present");
        return true;
      },
    );
  });
}
