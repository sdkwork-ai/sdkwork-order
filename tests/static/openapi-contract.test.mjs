import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

test("order openapi authorities declare v3 list and command envelopes", () => {
  const appOpenApiPath = path.join(
    repoRoot,
    "apis/app-api/order/order-app-api.openapi.json",
  );
  const backendOpenApiPath = path.join(
    repoRoot,
    "apis/backend-api/order/order-backend-api.openapi.json",
  );

  const appSpec = JSON.parse(fs.readFileSync(appOpenApiPath, "utf8"));
  const backendSpec = JSON.parse(fs.readFileSync(backendOpenApiPath, "utf8"));

  assert.ok(appSpec.components?.schemas?.SdkWorkListResponse);
  assert.ok(appSpec.components?.schemas?.SdkWorkCommandResponse);
  assert.ok(appSpec.paths["/app/v3/api/orders/{orderId}/payments"]?.get);
  assert.ok(appSpec.paths["/app/v3/api/recharges/packages"]?.get?.parameters?.length > 0);
  assert.ok(
    appSpec.paths["/app/v3/api/orders/payments/webhooks/{providerCode}"]?.post,
    "PSP webhook route must be on order app-api",
  );
  assert.equal(
    appSpec.paths["/app/v3/api/orders/payments/webhooks/{providerCode}"]?.post?.["x-sdkwork-auth-mode"],
    "anonymous",
    "public PSP webhooks must suppress generated SDK credentials",
  );
  for (const catalogPath of [
    "/app/v3/api/recharges/packages",
    "/app/v3/api/recharges/plans",
    "/app/v3/api/recharges/settings",
  ]) {
    const operation = appSpec.paths[catalogPath]?.get;
    assert.deepEqual(
      operation?.security,
      [],
      `${catalogPath} must be a public catalog operation`,
    );
    assert.equal(
      operation?.["x-sdkwork-auth-mode"],
      "anonymous",
      `${catalogPath} must suppress generated SDK credentials`,
    );
    assert.equal(
      operation?.["x-sdkwork-route-auth"],
      "public",
      `${catalogPath} must declare public route auth`,
    );
  }

  assert.ok(backendSpec.components?.schemas?.SdkWorkCommandResponse);
  assert.ok(backendSpec.paths["/backend/v3/api/orders"]?.get);
  assert.ok(
    backendSpec.paths["/backend/v3/api/orders/{orderId}/payment_confirmations"]?.post,
    "manual payment confirmation must be on order backend-api",
  );

  const cancelPost =
    appSpec.paths["/app/v3/api/orders/{orderId}/cancellations"]?.post;
  assert.ok(cancelPost?.["x-sdkwork-idempotent"]);
  assert.equal(cancelPost.operationId, "orders.cancellations.create");
  assert.equal(appSpec.paths["/app/v3/api/orders"]?.post, undefined);
  assert.equal(
    appSpec.paths["/app/v3/api/orders/{orderId}/cancel"],
    undefined,
  );

  for (const [, spec] of [["app", appSpec], ["backend", backendSpec]]) {
    for (const [path, methods] of Object.entries(spec.paths ?? {})) {
      for (const [method, operation] of Object.entries(methods ?? {})) {
        if (!operation?.["x-sdkwork-idempotent"]) {
          continue;
        }
        const params = operation.parameters ?? [];
        const hasIdempotency = params.some(
          (entry) =>
            entry?.$ref === "#/components/parameters/WriteCommandIdempotencyKey" ||
            entry?.name === "Idempotency-Key",
        );
        const hasRequestHash = params.some(
          (entry) =>
            entry?.$ref === "#/components/parameters/WriteCommandRequestHash" ||
            entry?.name === "Sdkwork-Request-Hash",
        );
        const hasBodyFingerprint = params.some(
          (entry) =>
            entry?.$ref === "#/components/parameters/WriteCommandIdempotencyFingerprint" ||
            entry?.name === "X-Idempotency-Fingerprint",
        );
        assert.ok(hasIdempotency, `${method.toUpperCase()} ${path} (${operation.operationId}) must declare Idempotency-Key`);
        assert.equal(hasRequestHash, false, `${method.toUpperCase()} ${path} must not expose Sdkwork-Request-Hash`);
        assert.equal(hasBodyFingerprint, false, `${method.toUpperCase()} ${path} must not expose X-Idempotency-Fingerprint`);
      }
    }
  }

  const checkoutCreate =
    appSpec.paths["/app/v3/api/checkout/sessions"]?.post;
  assert.equal(
    checkoutCreate?.requestBody?.content?.["application/json"]?.schema?.$ref,
    "#/components/schemas/CreateCheckoutSessionRequest",
  );
  assert.equal(
    checkoutCreate?.responses?.["201"]?.content?.["application/json"]?.schema?.$ref,
    "#/components/schemas/CreateCheckoutSessionResponse",
  );
  assert.equal(
    appSpec.paths["/app/v3/api/checkout/sessions/{checkoutSessionId}/quotes"]?.post?.requestBody,
    undefined,
  );
  assert.equal(
    appSpec.paths["/app/v3/api/checkout/sessions/{checkoutSessionId}/orders"]?.post?.requestBody,
    undefined,
  );

  assert.equal(
    backendSpec.components.parameters.WriteCommandIdempotencyKey.required,
    true,
    "all backend write commands require a stable Idempotency-Key",
  );
  assert.equal(
    backendSpec.paths[
      "/backend/v3/api/orders/{orderId}/payment_confirmations"
    ].post.parameters.find((parameter) => parameter.name === "Idempotency-Key")
      ?.required,
    true,
    "payment reconciliation requires a stable Idempotency-Key",
  );
  assert.equal(
    backendSpec.paths[
      "/backend/v3/api/orders/{orderId}/payment_confirmations"
    ].post.parameters.find((parameter) => parameter.name === "Idempotency-Key")
      ?.schema?.maxLength,
    128,
  );
});

test("sdk openapi inputs stay aligned with api authorities", () => {
  const pairs = [
    [
      "apis/app-api/order/order-app-api.openapi.json",
      "sdks/sdkwork-order-app-sdk/openapi/sdkwork-order-app-api.openapi.json",
    ],
    [
      "apis/backend-api/order/order-backend-api.openapi.json",
      "sdks/sdkwork-order-backend-sdk/openapi/sdkwork-order-backend-api.openapi.json",
    ],
  ];

  for (const [authorityPath, sdkPath] of pairs) {
    const authority = fs.readFileSync(path.join(repoRoot, authorityPath), "utf8");
    const sdkCopy = fs.readFileSync(path.join(repoRoot, sdkPath), "utf8");
    assert.equal(
      sdkCopy,
      authority,
      `${sdkPath} must match ${authorityPath}; run pnpm sync:openapi`,
    );
  }
});

test("order route manifests are exported from gateway assembly", () => {
  const assemblyLib = fs.readFileSync(
    path.join(repoRoot, "crates/sdkwork-api-order-assembly/src/lib.rs"),
    "utf8",
  );
  assert.match(assemblyLib, /order_contract_fallback_config/);
});

test("order routes do not retain client-owned request fingerprint headers", () => {
  for (const relativePath of [
    "crates/sdkwork-routes-order-app-api/src",
    "crates/sdkwork-routes-order-backend-api/src",
  ]) {
    const source = fs
      .readdirSync(path.join(repoRoot, relativePath), { recursive: true })
      .filter((entry) => String(entry).endsWith(".rs"))
      .map((entry) =>
        fs.readFileSync(path.join(repoRoot, relativePath, String(entry)), "utf8"),
      )
      .join("\n");
    assert.doesNotMatch(source, /Sdkwork-Request-Hash|X-Idempotency-Fingerprint/);
  }
});

test("payment webhook spec declares order ownership", () => {
  const spec = JSON.parse(
    fs.readFileSync(
      path.join(repoRoot, "specs/commerce-payment-webhook.spec.json"),
      "utf8",
    ),
  );
  assert.match(
    spec.ownedRoutes.app[0],
    /\/app\/v3\/api\/orders\/payments\/webhooks/,
  );
  assert.ok(spec.forbidden.some((entry) => entry.includes("sdkwork-payment")));
});

test("account value order specs define order orchestration boundary", () => {
  const accountValueSpecPath = path.join(
    repoRoot,
    "specs/ACCOUNT_VALUE_ORDER_SPEC.md",
  );
  assert.ok(
    fs.existsSync(accountValueSpecPath),
    "ACCOUNT_VALUE_ORDER_SPEC.md must define account-value order ownership",
  );

  const accountValueSpec = fs.readFileSync(accountValueSpecPath, "utf8");
  const rechargeSpec = JSON.parse(
    fs.readFileSync(path.join(repoRoot, "specs/commerce-recharge.spec.json"), "utf8"),
  );
  const topology = JSON.parse(
    fs.readFileSync(
      path.join(repoRoot, "specs/commerce-checkout-topology.spec.json"),
      "utf8",
    ),
  );

  assert.match(
    accountValueSpec,
    /Recharge, coupon redemption, refund, and withdrawal orchestration belong to `sdkwork-order`/,
  );
  assert.match(
    accountValueSpec,
    /`sdkwork-payment` executes provider payment and refund channels today, and owns any future provider payout executor contract/,
  );
  assert.match(
    accountValueSpec,
    /`sdkwork-account` is the ledger truth source/,
  );
  assert.match(
    accountValueSpec,
    /current default runtime is fail-closed until `sdkwork-payment` exposes a concrete provider payout executor/,
  );

  assert.deepEqual(rechargeSpec.accountValueOrder, {
    owner: "sdkwork-order",
    paymentExecutor: "sdkwork-payment",
    ledgerExecutor: "sdkwork-account",
    directPaymentToAccountDependencyAllowed: false,
    subjects: [
      "points_recharge",
      "token_bank_recharge",
      "token_bank_plan_purchase",
      "token_bank_plan_renewal",
      "account_recharge_package",
      "coupon_recharge",
      "refund_request",
      "cash_withdrawal",
    ],
  });

  for (const forbidden of [
    "direct account SQL writes",
    "payment-owned recharge routes",
    "payment-to-account ledger writes",
    "naked token account naming",
  ]) {
    assert.ok(
      rechargeSpec.forbidden.includes(forbidden) ||
        topology.forbidden.includes(forbidden),
      `${forbidden} must be forbidden by order specs`,
    );
  }

  for (const subject of [
    "token_bank_recharge",
    "token_bank_plan_purchase",
    "token_bank_plan_renewal",
    "account_recharge_package",
    "coupon_recharge",
    "refund_request",
    "cash_withdrawal",
  ]) {
    assert.ok(
      topology.subjectFulfillment.some((entry) => entry.subject === subject),
      `${subject} must declare an order-owned fulfillment path`,
    );
  }
  assert.equal(
    topology.subjectFulfillment.find((entry) => entry.subject === "cash_withdrawal")
      ?.status,
    "fail-closed-payout",
    "cash withdrawal must not be documented as completed provider payout until payment publishes a concrete payout executor",
  );
});

test("account value database and docs are aligned to implemented settlement paths", () => {
  const requiredTables = [
    "commerce_account_value_package",
    "commerce_token_bank_plan",
    "commerce_order_refund_request",
    "commerce_order_withdrawal_request",
  ];
  const tableRegistry = JSON.parse(
    fs.readFileSync(
      path.join(repoRoot, "database/contract/table-registry.json"),
      "utf8",
    ),
  );
  const schemaYaml = fs.readFileSync(
    path.join(repoRoot, "database/contract/schema.yaml"),
    "utf8",
  );

  const registeredTables = new Set(
    (tableRegistry.tables ?? []).map((entry) => entry.name ?? entry.table_name),
  );
  for (const table of requiredTables) {
    assert.ok(registeredTables.has(table), `${table} must be registered`);
    assert.match(schemaYaml, new RegExp(`name: ${table}\\b`));
  }

  const prd = fs.readFileSync(
    path.join(repoRoot, "docs/product/prd/PRD.md"),
    "utf8",
  );
  for (const subject of [
    "token_bank_recharge",
    "token_bank_plan_purchase",
    "token_bank_plan_renewal",
    "account_recharge_package",
    "coupon_recharge",
  ]) {
    assert.doesNotMatch(
      prd,
      new RegExp(`\\| \`${subject}\` \\| Planned`),
      `${subject} fulfillment is implemented through AccountValueLedgerPort and must not be documented as Planned`,
    );
  }
});

test("account value app and backend APIs expose complete order-owned workflows", () => {
  const appSpec = JSON.parse(
    fs.readFileSync(
      path.join(repoRoot, "apis/app-api/order/order-app-api.openapi.json"),
      "utf8",
    ),
  );
  const backendSpec = JSON.parse(
    fs.readFileSync(
      path.join(repoRoot, "apis/backend-api/order/order-backend-api.openapi.json"),
      "utf8",
    ),
  );

  const requiredAppOperations = [
    ["/app/v3/api/recharges/plans", "get", "recharges.plans.list"],
    [
      "/app/v3/api/orders/refund_requests",
      "get",
      "orders.refundRequests.list",
    ],
    [
      "/app/v3/api/orders/refund_requests",
      "post",
      "orders.refundRequests.create",
    ],
    [
      "/app/v3/api/orders/refund_requests/{refundRequestId}",
      "get",
      "orders.refundRequests.retrieve",
    ],
    [
      "/app/v3/api/withdrawals/requests",
      "post",
      "withdrawals.requests.create",
    ],
    [
      "/app/v3/api/withdrawals/requests/{withdrawalRequestId}",
      "get",
      "withdrawals.requests.retrieve",
    ],
  ];

  const requiredBackendOperations = [
    [
      "/backend/v3/api/account_value_packages",
      "get",
      "backend.accountValuePackages.list",
    ],
    [
      "/backend/v3/api/account_value_packages",
      "post",
      "backend.accountValuePackages.create",
    ],
    [
      "/backend/v3/api/account_value_packages/{packageId}",
      "patch",
      "backend.accountValuePackages.update",
    ],
    [
      "/backend/v3/api/account_value_packages/{packageId}/retire",
      "post",
      "backend.accountValuePackages.retire",
    ],
    [
      "/backend/v3/api/token_bank_plans",
      "get",
      "backend.tokenBankPlans.list",
    ],
    [
      "/backend/v3/api/token_bank_plans",
      "post",
      "backend.tokenBankPlans.create",
    ],
    [
      "/backend/v3/api/token_bank_plans/{planCode}",
      "patch",
      "backend.tokenBankPlans.update",
    ],
    [
      "/backend/v3/api/token_bank_plans/{planCode}/retire",
      "post",
      "backend.tokenBankPlans.retire",
    ],
    [
      "/backend/v3/api/refund_requests",
      "get",
      "backend.refundRequests.list",
    ],
    [
      "/backend/v3/api/refund_requests/{refundRequestId}/approve",
      "post",
      "backend.refundRequests.approve",
    ],
    [
      "/backend/v3/api/refund_requests/{refundRequestId}/reject",
      "post",
      "backend.refundRequests.reject",
    ],
    [
      "/backend/v3/api/refund_requests/{refundRequestId}/retry",
      "post",
      "backend.refundRequests.retry",
    ],
    [
      "/backend/v3/api/withdrawal_requests",
      "get",
      "backend.withdrawalRequests.list",
    ],
    [
      "/backend/v3/api/withdrawal_requests/{withdrawalRequestId}/approve",
      "post",
      "backend.withdrawalRequests.approve",
    ],
    [
      "/backend/v3/api/withdrawal_requests/{withdrawalRequestId}/reject",
      "post",
      "backend.withdrawalRequests.reject",
    ],
    [
      "/backend/v3/api/withdrawal_requests/{withdrawalRequestId}/retry",
      "post",
      "backend.withdrawalRequests.retry",
    ],
  ];

  for (const [apiPath, method, operationId] of requiredAppOperations) {
    const operation = appSpec.paths?.[apiPath]?.[method];
    assert.equal(
      operation?.operationId,
      operationId,
      `${method.toUpperCase()} ${apiPath} must expose ${operationId}`,
    );
  }

  for (const [apiPath, method, operationId] of requiredBackendOperations) {
    const operation = backendSpec.paths?.[apiPath]?.[method];
    assert.equal(
      operation?.operationId,
      operationId,
      `${method.toUpperCase()} ${apiPath} must expose ${operationId}`,
    );
  }

  for (const [apiPath, method] of requiredBackendOperations.filter(
    ([, method, operationId]) =>
      method === "get" && operationId.startsWith("backend."),
  )) {
    const schemaRef =
      backendSpec.paths?.[apiPath]?.[method]?.responses?.["200"]?.content?.[
        "application/json"
      ]?.schema?.$ref;
    assert.equal(
      schemaRef,
      "#/components/schemas/SdkWorkListResponse",
      `${method.toUpperCase()} ${apiPath} must use the standard list response envelope`,
    );
    const listSchema = backendSpec.components?.schemas?.SdkWorkListResponse;
    assert.ok(
      listSchema?.allOf?.some(
        (entry) => entry?.$ref === "#/components/schemas/SdkWorkApiResponse",
      ),
      "backend SdkWorkListResponse must extend SdkWorkApiResponse",
    );
  }

  const createRechargeSchema =
    appSpec.components?.schemas?.RechargeOrderCreateCommand?.properties ?? {};
  for (const field of [
    "subject",
    "targetAsset",
    "grantAmount",
    "planCode",
    "planPeriod",
    "couponCode",
    "paymentProduct",
  ]) {
    assert.ok(
      createRechargeSchema[field],
      `RechargeOrderCreateCommand must include ${field}`,
    );
  }
  assert.equal(
    createRechargeSchema.paymentProduct?.default,
    "mobile_cashier_h5",
    "RechargeOrderCreateCommand must default QR checkout to the mobile H5 cashier",
  );

  for (const [label, spec] of [
    ["app", appSpec],
    ["backend", backendSpec],
  ]) {
    const accountValueRequest =
      spec.components?.schemas?.AccountValueRequestResponse?.properties ?? {};
    assert.ok(
      accountValueRequest.accountValueRequestId,
      `${label} AccountValueRequestResponse must expose accountValueRequestId`,
    );
    assert.equal(
      accountValueRequest.requestId,
      undefined,
      `${label} AccountValueRequestResponse must not expose forbidden requestId`,
    );
  }
});
