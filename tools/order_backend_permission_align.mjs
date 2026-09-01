#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HTTP_METHODS = new Set(["get", "post", "put", "patch", "delete"]);

/**
 * Operations whose enforced permission is narrower than the coarse
 * read/manage rule enforced by the backend router permission constants
 * (see `crates/sdkwork-routes-order-backend-api`). These are pinned by
 * operationId and must stay aligned with the Rust permission constants and
 * the product specs; the blanket rule below must never overwrite them.
 */
const PERMISSION_OVERRIDES = new Map([
  ["orders.paymentConfirmations.create", "commerce.orders.fulfill"],
]);

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sourcePath = path.join(root, "apis/backend-api/order/order-backend-api.openapi.json");
const authorityPath = path.join(root, "sdks/sdkwork-order-backend-sdk/openapi/sdkwork-order-backend-api.openapi.json");
const checkMode = process.argv.includes("--check");

const openapi = JSON.parse(readFileSync(sourcePath, "utf8").replace(/^\uFEFF/u, ""));
for (const pathItem of Object.values(openapi.paths ?? {})) {
  for (const [method, operation] of Object.entries(pathItem ?? {})) {
    if (!HTTP_METHODS.has(method)) continue;
    operation["x-sdkwork-owner"] = "sdkwork-order";
    operation["x-sdkwork-api-authority"] = "sdkwork-order-backend-api";
    operation["x-sdkwork-permission"] =
      PERMISSION_OVERRIDES.get(operation.operationId) ??
      (method === "get" ? "commerce.orders.read" : "commerce.orders.manage");
  }
}

const expected = `${JSON.stringify(openapi, null, 2)}\n`;
for (const target of [sourcePath, authorityPath]) {
  const current = readFileSync(target, "utf8").replace(/^\uFEFF/u, "");
  if (checkMode && current !== expected) {
    throw new Error(`${path.relative(root, target)} is not aligned with the Order backend permission contract`);
  }
  if (!checkMode && current !== expected) {
    writeFileSync(target, expected, "utf8");
  }
}

console.log(`[order_backend_permission_align] ${checkMode ? "check passed" : "aligned"}`);
