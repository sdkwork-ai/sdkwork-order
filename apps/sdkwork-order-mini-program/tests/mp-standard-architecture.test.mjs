import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { strict as assert } from "node:assert";
import { test } from "node:test";

const appRoot = fileURLToPath(new URL("..", import.meta.url));

function read(relativePath) {
  return readFileSync(join(appRoot, relativePath), "utf8");
}

function listFiles(dir, out = []) {
  const absolute = join(appRoot, dir);
  for (const entry of readdirSync(absolute)) {
    const entryPath = join(absolute, entry);
    if (statSync(entryPath).isDirectory()) {
      listFiles(join(dir, entry), out);
    } else {
      out.push(`${dir}/${entry}`.replaceAll("\\", "/"));
    }
  }
  return out;
}

const EXPECTED_PAGES = [
  "pages/orders/index",
  "pages/recharge/index",
  "pages/profile/index",
  "pages/order-detail/index",
  "pages/cashier/index",
  "pages/payment-result/index",
  "pages/coupon/index",
  "pages/withdraw/index",
  "pages/refund/index",
  "pages/login/index",
];

test("order mini-program root is manifest-driven", () => {
  const manifest = JSON.parse(read("sdkwork.app.config.json"));
  assert.equal(manifest.app?.key, "sdkwork-order-mini-program");
  assert.equal(manifest.runtime?.family, "mini-program");
  assert.equal(manifest.runtime?.framework, "weixin-mini-program");
  assert.deepEqual(manifest.runtime?.runtimes, ["MINI_PROGRAM"]);
  assert.deepEqual(manifest.publish?.platforms, ["MP_WEIXIN"]);
  assert.equal(manifest.publish?.installPlatforms?.[0], "MP_WEIXIN");
  assert.equal(manifest.metadata?.domain, "commerce");
  assert.equal(manifest.metadata?.capability, "order");
});

test("order mini-program tab bar covers orders/recharge/profile", () => {
  const appJson = JSON.parse(read("src/app.json"));
  const tabs = appJson.tabBar.list.map((entry) => entry.pagePath);
  assert.deepEqual(tabs, [
    "pages/orders/index",
    "pages/recharge/index",
    "pages/profile/index",
  ]);
  for (const entry of appJson.tabBar.list) {
    assert.ok(entry.text, `tab ${entry.pagePath} must carry a label`);
  }
});

test("order mini-program registers the full order route catalog 1:1 with disk", () => {
  const appJson = JSON.parse(read("src/app.json"));
  assert.deepEqual(appJson.pages, EXPECTED_PAGES);
  for (const page of appJson.pages) {
    for (const extension of [".js", ".json", ".wxml", ".wxss"]) {
      assert.ok(
        statSync(join(appRoot, "src", `${page}${extension}`)).isFile(),
        `page ${page} must ship ${extension}`,
      );
    }
  }
  const diskPages = new Set(
    listFiles("src/pages")
      .filter((file) => file.endsWith(".js"))
      .map((file) => file.replace(/^src\//u, "").replace(/\.js$/u, "")),
  );
  assert.deepEqual(
    [...diskPages].sort(),
    [...appJson.pages].sort(),
    "app.json pages and src/pages directories must match exactly",
  );
});

test("order mini-program funnels all traffic through the single transport seam", () => {
  const transportSource = read("src/services/transport.js");
  const seamCalls = transportSource.match(/wx\.request\(/gu) ?? [];
  assert.equal(seamCalls.length, 1, "services/transport.js must hold exactly one wx.request");

  for (const file of listFiles("src").filter((file) => file.endsWith(".js"))) {
    if (file === "src/services/transport.js") {
      continue;
    }
    assert.ok(
      !read(file).includes("wx.request("),
      `${file} must go through the transport seam`,
    );
  }
});

test("transport maps the SdkWork envelope and problem+json errors", () => {
  const transportSource = read("src/services/transport.js");
  assert.match(transportSource, /payload\.code\)\s*===\s*0/u, "must unwrap code===0 envelopes");
  assert.match(transportSource, /problem\+json|problem\.code/u, "must map ProblemDetail errors");
  assert.match(transportSource, /traceId/u, "must surface traceId on errors");
  assert.match(transportSource, /Bearer /u, "must attach the bearer token");
  assert.match(transportSource, /Idempotency-Key/iu, "must support idempotency write headers");
  assert.match(transportSource, /15000/u, "must default to a 15s timeout");
});

test("runtime-env is generated with the required SDKWORK keys", () => {
  let runtimeEnv;
  try {
    runtimeEnv = JSON.parse(read("src/runtime-env.json"));
  } catch {
    assert.fail("src/runtime-env.json is missing; run `node scripts/build-runtime.mjs` first");
  }
  assert.equal(runtimeEnv.runtimeTarget, "mini-program");
  assert.ok(runtimeEnv.appApiBaseUrl?.includes("/app/v3/api"), "runtime env must carry the app-api base");
  assert.ok(runtimeEnv.environment, "runtime env must carry the environment");
  assert.ok(runtimeEnv.deploymentProfile, "runtime env must carry the deployment profile");
  assert.ok(runtimeEnv.profileId, "runtime env must carry the profile id");
});

test("config templates carry the native WeChat runtime-env keys and stay secret-free", () => {
  const configDir = join(appRoot, "config", "mini-program");
  const templates = readdirSync(configDir).filter((name) => name.endsWith(".json"));
  assert.ok(templates.length >= 6, "must ship standalone dev/test/staging/prod + cloud dev/prod templates");
  for (const name of templates) {
    const template = JSON.parse(readFileSync(join(configDir, name), "utf8"));
    assert.equal(template.SDKWORK_RUNTIME_TARGET, "mini-program", `${name} runtime target`);
    assert.ok(template.SDKWORK_ENVIRONMENT, `${name} environment`);
    assert.ok(template.SDKWORK_DEPLOYMENT_PROFILE, `${name} deployment profile`);
    assert.ok(template.SDKWORK_PROFILE_ID, `${name} profile id`);
    assert.match(template.appApiBaseUrl, /\/app\/v3\/api$/u, `${name} app-api base`);
  }
  const hostDir = join(appRoot, "config", "host");
  for (const name of readdirSync(hostDir).filter((entry) => entry.endsWith(".json"))) {
    const source = readFileSync(join(hostDir, name), "utf8").toLowerCase();
    for (const forbidden of ["secret", "privatekey", "private_key", "password", "accesstoken", "access_token"]) {
      assert.ok(!source.includes(forbidden), `${name} must not embed ${forbidden}`);
    }
  }
});

test("order facade composes the domain services", () => {
  const facade = read("src/bootstrap/sdkClients.ts");
  for (const serviceType of [
    "SdkworkOrderMpOrderService",
    "SdkworkOrderMpRechargeService",
    "SdkworkOrderMpWithdrawalService",
    "SdkworkOrderMpSessionStore",
    "SdkworkOrderMpClient",
  ]) {
    assert.match(facade, new RegExp(serviceType, "u"), `facade must type ${serviceType}`);
  }
  for (const [file, exports] of [
    ["src/services/order-service.js", ["listOrders", "getOrderDetail", "createPayment", "pollPaymentSuccess", "redeemCoupon", "createRefundRequest"]],
    ["src/services/recharge-service.js", ["listPlans", "listPackages", "createRechargeOrder", "cancelRechargeOrder"]],
    ["src/services/withdrawal-service.js", ["createWithdrawalRequest", "getWithdrawalRequest", "listLocalHistory"]],
    ["src/services/session.js", ["getToken", "setToken", "clearToken", "isLoggedIn"]],
  ]) {
    const source = read(file);
    assert.match(source, /module\.exports/u, `${file} must export its domain surface`);
    for (const name of exports) {
      assert.ok(source.includes(name), `${file} must export ${name}`);
    }
  }
});

test("route contributions align 1:1 with app.json pages", () => {
  const routesSource = read("src/bootstrap/routes.ts");
  const appJson = JSON.parse(read("src/app.json"));
  const routePagePaths = [...routesSource.matchAll(/pagePath:\s*"([^"]+)"/gu)].map((match) => match[1]);
  assert.deepEqual(
    [...new Set(routePagePaths)].sort(),
    [...appJson.pages].sort(),
    "routes.ts placements must cover exactly the registered pages",
  );
  const routeIds = [...routesSource.matchAll(/id:\s*"([^"]+)"/gu)].map((match) => match[1]);
  for (const id of routeIds) {
    assert.match(id, /^app\.commerce\.[a-z_]+\.[a-z_]+$/u, `route id ${id} must follow surface.domain.capability.screen`);
  }
  for (const tab of appJson.tabBar.list) {
    assert.ok(
      routePagePaths.includes(tab.pagePath),
      `tab page ${tab.pagePath} must have a route contribution`,
    );
  }
});

test("amounts stay minor-unit strings and ids stay strings end to end", () => {
  const formatSource = read("src/utils/format.js");
  assert.match(formatSource, /formatMinor/u, "format.js must expose the minor-unit formatter");
  assert.match(formatSource, /yuanInputToMinor/u, "format.js must expose yuan-to-minor input conversion");
  const orderServiceSource = read("src/services/order-service.js");
  assert.match(orderServiceSource, /String\(record\.orderId/u, "orderId must map through String()");
  assert.match(orderServiceSource, /Idempotency-Key/u, "write commands must send Idempotency-Key");
  const rechargeServiceSource = read("src/services/recharge-service.js");
  assert.match(rechargeServiceSource, /planPriceToMinor/u, "recharge prices must convert major->minor");
  assert.ok(
    !/Number\([^)]*orderId/u.test(orderServiceSource),
    "orderId must never be converted to number",
  );
});
