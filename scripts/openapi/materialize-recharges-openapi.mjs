#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const orderRoot = path.resolve(__dirname, "../..");

const appAuthorityPath = path.join(
  orderRoot,
  "apis/app-api/order/order-app-api.openapi.json",
);
const appSdkOpenApiPath = path.join(
  orderRoot,
  "sdks/sdkwork-order-app-sdk/openapi/sdkwork-order-app-api.openapi.json",
);
const appSdkGenPath = path.join(
  orderRoot,
  "sdks/sdkwork-order-app-sdk/openapi/sdkwork-order-app-api.sdkgen.json",
);
const backendAuthorityPath = path.join(
  orderRoot,
  "apis/backend-api/order/order-backend-api.openapi.json",
);
const backendSdkOpenApiPath = path.join(
  orderRoot,
  "sdks/sdkwork-order-backend-sdk/openapi/sdkwork-order-backend-api.openapi.json",
);

const appSecurity = [{ AuthToken: [], AccessToken: [] }];
const backendSecurity = [{ AuthToken: [], AccessToken: [] }];

const problemResponses = {
  400: problemResponse("Bad request"),
  401: problemResponse("Unauthorized"),
  403: problemResponse("Forbidden"),
  404: problemResponse("Not found"),
  409: problemResponse("Conflict"),
  500: problemResponse("Internal server error"),
};

const offsetPaginationParameters = [
  queryParameter("page", { type: "integer", minimum: 1, default: 1 }),
  queryParameter("page_size", {
    type: "integer",
    minimum: 1,
    maximum: 200,
    default: 20,
  }),
];

const writeCommandParameters = [
  { $ref: "#/components/parameters/WriteCommandIdempotencyKey" },
  { $ref: "#/components/parameters/WriteCommandRequestHash" },
  { $ref: "#/components/parameters/WriteCommandIdempotencyFingerprint" },
];

function problemResponse(description) {
  return {
    description,
    content: {
      "application/problem+json": {
        schema: { $ref: "#/components/schemas/ProblemDetail" },
      },
    },
  };
}

function queryParameter(name, schema) {
  return { name, in: "query", required: false, schema };
}

function pathParameter(name) {
  return { name, in: "path", required: true, schema: { type: "string" } };
}

function requestBody(schemaRef, required = true) {
  return {
    required,
    content: {
      "application/json": {
        schema: { $ref: schemaRef },
      },
    },
  };
}

function responses(schemaRef, successStatus = 200) {
  return {
    [successStatus]: {
      description: successStatus === 201 ? "Created" : "Success",
      content: {
        "application/json": {
          schema: { $ref: schemaRef },
        },
      },
    },
    ...problemResponses,
  };
}

function extension(surface, resource) {
  return {
    "x-sdkwork-owner": "sdkwork-order",
    "x-sdkwork-api-authority": `sdkwork-order-${surface}`,
    "x-sdkwork-domain": "commerce",
    "x-sdkwork-resource": resource,
    "x-sdkwork-request-context": "WebRequestContext",
    "x-sdkwork-api-surface": surface,
  };
}

function operation({
  tags,
  summary,
  operationId,
  resource,
  surface,
  security,
  responseSchema,
  successStatus,
  parameters = [],
  body,
  idempotent = false,
}) {
  return {
    tags,
    summary,
    operationId,
    responses: responses(responseSchema, successStatus),
    security,
    parameters: idempotent
      ? [...parameters, ...writeCommandParameters]
      : parameters,
    ...(body ? { requestBody: body } : {}),
    ...(idempotent ? { "x-sdkwork-idempotent": true } : {}),
    ...extension(surface, resource),
  };
}

function appOperation(input) {
  return operation({
    surface: "app-api",
    security: appSecurity,
    ...input,
  });
}

function backendOperation(input) {
  return operation({
    surface: "backend-api",
    security: backendSecurity,
    ...input,
  });
}

const appPaths = {
  "/app/v3/api/recharges/packages": {
    get: appOperation({
      tags: ["recharges"],
      summary: "Recharges packages list.",
      operationId: "recharges.packages.list",
      resource: "recharges.packages",
      responseSchema: "#/components/schemas/SdkWorkListResponse",
      parameters: offsetPaginationParameters,
    }),
  },
  "/app/v3/api/recharges/plans": {
    get: appOperation({
      tags: ["recharges"],
      summary: "Token Bank plans list.",
      operationId: "recharges.plans.list",
      resource: "recharges.plans",
      responseSchema: "#/components/schemas/SdkWorkListResponse",
      parameters: [
        queryParameter("status", { type: "string" }),
        ...offsetPaginationParameters,
      ],
    }),
  },
  "/app/v3/api/recharges/settings": {
    get: appOperation({
      tags: ["recharges"],
      summary: "Recharges settings retrieve.",
      operationId: "recharges.settings.retrieve",
      resource: "recharges.settings",
      responseSchema: "#/components/schemas/SdkWorkResourceResponse",
    }),
  },
  "/app/v3/api/recharges/orders": {
    get: appOperation({
      tags: ["recharges"],
      summary: "Recharges orders list.",
      operationId: "recharges.orders.list",
      resource: "recharges.orders",
      responseSchema: "#/components/schemas/SdkWorkListResponse",
      parameters: [
        queryParameter("subject", { type: "string" }),
        queryParameter("status", { type: "string" }),
        ...offsetPaginationParameters,
      ],
    }),
    post: appOperation({
      tags: ["recharges"],
      summary: "Recharges orders create.",
      operationId: "recharges.orders.create",
      resource: "recharges.orders",
      responseSchema: "#/components/schemas/SdkWorkResourceResponse",
      successStatus: 201,
      body: requestBody("#/components/schemas/RechargeOrderCreateCommand"),
      idempotent: true,
    }),
  },
  "/app/v3/api/recharges/orders/{orderId}": {
    get: appOperation({
      tags: ["recharges"],
      summary: "Recharges orders retrieve.",
      operationId: "recharges.orders.retrieve",
      resource: "recharges.orders",
      responseSchema: "#/components/schemas/SdkWorkResourceResponse",
      parameters: [pathParameter("orderId")],
    }),
  },
  "/app/v3/api/recharges/orders/{orderId}/cancel": {
    post: appOperation({
      tags: ["recharges"],
      summary: "Recharges orders cancel.",
      operationId: "recharges.orders.cancel",
      resource: "recharges.orders",
      responseSchema: "#/components/schemas/SdkWorkCommandResponse",
      parameters: [pathParameter("orderId")],
      body: requestBody("#/components/schemas/CommerceOperationCommand", false),
      idempotent: true,
    }),
  },
  "/app/v3/api/orders/refund_requests": {
    get: appOperation({
      tags: ["orders"],
      summary: "Order refund requests list.",
      operationId: "orders.refundRequests.list",
      resource: "orders.refundRequests",
      responseSchema: "#/components/schemas/SdkWorkListResponse",
      parameters: [
        queryParameter("status", { type: "string" }),
        ...offsetPaginationParameters,
      ],
    }),
    post: appOperation({
      tags: ["orders"],
      summary: "Order refund requests create.",
      operationId: "orders.refundRequests.create",
      resource: "orders.refundRequests",
      responseSchema: "#/components/schemas/SdkWorkResourceResponse",
      successStatus: 201,
      body: requestBody("#/components/schemas/RefundRequestCreateCommand"),
      idempotent: true,
    }),
  },
  "/app/v3/api/orders/refund_requests/{refundRequestId}": {
    get: appOperation({
      tags: ["orders"],
      summary: "Order refund requests retrieve.",
      operationId: "orders.refundRequests.retrieve",
      resource: "orders.refundRequests",
      responseSchema: "#/components/schemas/SdkWorkResourceResponse",
      parameters: [pathParameter("refundRequestId")],
    }),
  },
  "/app/v3/api/withdrawals/requests": {
    post: appOperation({
      tags: ["withdrawals"],
      summary: "Withdrawal requests create.",
      operationId: "withdrawals.requests.create",
      resource: "withdrawals.requests",
      responseSchema: "#/components/schemas/SdkWorkResourceResponse",
      successStatus: 201,
      body: requestBody("#/components/schemas/WithdrawalRequestCreateCommand"),
      idempotent: true,
    }),
  },
  "/app/v3/api/withdrawals/requests/{withdrawalRequestId}": {
    get: appOperation({
      tags: ["withdrawals"],
      summary: "Withdrawal requests retrieve.",
      operationId: "withdrawals.requests.retrieve",
      resource: "withdrawals.requests",
      responseSchema: "#/components/schemas/SdkWorkResourceResponse",
      parameters: [pathParameter("withdrawalRequestId")],
    }),
  },
};

const backendPaths = {
  "/backend/v3/api/account_value_packages": {
    get: backendOperation({
      tags: ["backend"],
      summary: "Account value packages list.",
      operationId: "backend.accountValuePackages.list",
      resource: "backend.accountValuePackages",
      responseSchema: "#/components/schemas/SdkWorkListResponse",
      parameters: [
        queryParameter("target_asset", { type: "string" }),
        queryParameter("status", { type: "string" }),
        ...offsetPaginationParameters,
      ],
    }),
    post: backendOperation({
      tags: ["backend"],
      summary: "Account value packages create.",
      operationId: "backend.accountValuePackages.create",
      resource: "backend.accountValuePackages",
      responseSchema: "#/components/schemas/SdkWorkItemResponse",
      successStatus: 201,
      body: requestBody("#/components/schemas/AccountValuePackageWriteCommand"),
      idempotent: true,
    }),
  },
  "/backend/v3/api/account_value_packages/{packageId}": {
    patch: backendOperation({
      tags: ["backend"],
      summary: "Account value packages update.",
      operationId: "backend.accountValuePackages.update",
      resource: "backend.accountValuePackages",
      responseSchema: "#/components/schemas/SdkWorkItemResponse",
      parameters: [pathParameter("packageId")],
      body: requestBody("#/components/schemas/AccountValuePackageWriteCommand"),
      idempotent: true,
    }),
  },
  "/backend/v3/api/account_value_packages/{packageId}/retire": {
    post: backendOperation({
      tags: ["backend"],
      summary: "Account value packages retire.",
      operationId: "backend.accountValuePackages.retire",
      resource: "backend.accountValuePackages",
      responseSchema: "#/components/schemas/SdkWorkCommandResponse",
      parameters: [pathParameter("packageId")],
      body: requestBody("#/components/schemas/CommerceOperationCommand", false),
      idempotent: true,
    }),
  },
  "/backend/v3/api/token_bank_plans": {
    get: backendOperation({
      tags: ["backend"],
      summary: "Token Bank plans list.",
      operationId: "backend.tokenBankPlans.list",
      resource: "backend.tokenBankPlans",
      responseSchema: "#/components/schemas/SdkWorkListResponse",
      parameters: [
        queryParameter("status", { type: "string" }),
        ...offsetPaginationParameters,
      ],
    }),
    post: backendOperation({
      tags: ["backend"],
      summary: "Token Bank plans create.",
      operationId: "backend.tokenBankPlans.create",
      resource: "backend.tokenBankPlans",
      responseSchema: "#/components/schemas/SdkWorkItemResponse",
      successStatus: 201,
      body: requestBody("#/components/schemas/TokenBankPlanWriteCommand"),
      idempotent: true,
    }),
  },
  "/backend/v3/api/token_bank_plans/{planCode}": {
    patch: backendOperation({
      tags: ["backend"],
      summary: "Token Bank plans update.",
      operationId: "backend.tokenBankPlans.update",
      resource: "backend.tokenBankPlans",
      responseSchema: "#/components/schemas/SdkWorkItemResponse",
      parameters: [pathParameter("planCode")],
      body: requestBody("#/components/schemas/TokenBankPlanWriteCommand"),
      idempotent: true,
    }),
  },
  "/backend/v3/api/token_bank_plans/{planCode}/retire": {
    post: backendOperation({
      tags: ["backend"],
      summary: "Token Bank plans retire.",
      operationId: "backend.tokenBankPlans.retire",
      resource: "backend.tokenBankPlans",
      responseSchema: "#/components/schemas/SdkWorkCommandResponse",
      parameters: [pathParameter("planCode")],
      body: requestBody("#/components/schemas/CommerceOperationCommand", false),
      idempotent: true,
    }),
  },
  "/backend/v3/api/refund_requests": {
    get: backendOperation({
      tags: ["backend"],
      summary: "Refund requests list.",
      operationId: "backend.refundRequests.list",
      resource: "backend.refundRequests",
      responseSchema: "#/components/schemas/SdkWorkListResponse",
      parameters: [
        queryParameter("status", { type: "string" }),
        ...offsetPaginationParameters,
      ],
    }),
  },
  "/backend/v3/api/refund_requests/{refundRequestId}/approve": requestActionPath(
    "refundRequestId",
    "backend.refundRequests.approve",
    "Refund requests approve.",
    "backend.refundRequests",
  ),
  "/backend/v3/api/refund_requests/{refundRequestId}/reject": requestActionPath(
    "refundRequestId",
    "backend.refundRequests.reject",
    "Refund requests reject.",
    "backend.refundRequests",
  ),
  "/backend/v3/api/refund_requests/{refundRequestId}/retry": requestActionPath(
    "refundRequestId",
    "backend.refundRequests.retry",
    "Refund requests retry.",
    "backend.refundRequests",
  ),
  "/backend/v3/api/withdrawal_requests": {
    get: backendOperation({
      tags: ["backend"],
      summary: "Withdrawal requests list.",
      operationId: "backend.withdrawalRequests.list",
      resource: "backend.withdrawalRequests",
      responseSchema: "#/components/schemas/SdkWorkListResponse",
      parameters: [
        queryParameter("status", { type: "string" }),
        ...offsetPaginationParameters,
      ],
    }),
  },
  "/backend/v3/api/withdrawal_requests/{withdrawalRequestId}/approve": requestActionPath(
    "withdrawalRequestId",
    "backend.withdrawalRequests.approve",
    "Withdrawal requests approve.",
    "backend.withdrawalRequests",
  ),
  "/backend/v3/api/withdrawal_requests/{withdrawalRequestId}/reject": requestActionPath(
    "withdrawalRequestId",
    "backend.withdrawalRequests.reject",
    "Withdrawal requests reject.",
    "backend.withdrawalRequests",
  ),
  "/backend/v3/api/withdrawal_requests/{withdrawalRequestId}/retry": requestActionPath(
    "withdrawalRequestId",
    "backend.withdrawalRequests.retry",
    "Withdrawal requests retry.",
    "backend.withdrawalRequests",
  ),
};

function requestActionPath(parameterName, operationId, summary, resource) {
  return {
    post: backendOperation({
      tags: ["backend"],
      summary,
      operationId,
      resource,
      responseSchema: "#/components/schemas/SdkWorkCommandResponse",
      parameters: [pathParameter(parameterName)],
      body: requestBody("#/components/schemas/AccountValueRequestReviewCommand", false),
      idempotent: true,
    }),
  };
}

function patchAppSpec(spec) {
  const next = structuredClone(spec);
  next.paths = { ...next.paths, ...appPaths };
  next.components ??= {};
  next.components.schemas ??= {};
  next.components.schemas.RechargeOrderCreateCommand = rechargeOrderCreateCommandSchema();
  next.components.schemas.RefundRequestCreateCommand = refundRequestCreateCommandSchema();
  next.components.schemas.WithdrawalRequestCreateCommand = withdrawalRequestCreateCommandSchema();
  next.components.schemas.AccountValueRequestResponse = accountValueRequestResponseSchema();
  next.components.schemas.TokenBankPlanResponse = tokenBankPlanResponseSchema();
  return next;
}

function patchBackendSpec(spec) {
  const next = structuredClone(spec);
  next.paths = { ...next.paths, ...backendPaths };
  next.components ??= {};
  next.components.schemas ??= {};
  ensureStandardResponseSchemas(next.components.schemas);
  next.components.schemas.AccountValuePackageWriteCommand =
    accountValuePackageWriteCommandSchema();
  next.components.schemas.TokenBankPlanWriteCommand = tokenBankPlanWriteCommandSchema();
  next.components.schemas.AccountValueRequestReviewCommand =
    accountValueRequestReviewCommandSchema();
  next.components.schemas.AccountValuePackageResponse = accountValuePackageResponseSchema();
  next.components.schemas.TokenBankPlanResponse = tokenBankPlanResponseSchema();
  next.components.schemas.AccountValueRequestResponse = accountValueRequestResponseSchema();
  return next;
}

function ensureStandardResponseSchemas(schemas) {
  schemas.SdkWorkPageData ??= {
    type: "object",
    additionalProperties: false,
    required: ["items", "pageInfo"],
    properties: {
      items: {
        type: "array",
        items: {
          type: "object",
          additionalProperties: true,
        },
      },
      pageInfo: { $ref: "#/components/schemas/PageInfo" },
    },
  };
  schemas.SdkWorkListResponse ??= {
    allOf: [
      { $ref: "#/components/schemas/SdkWorkApiResponse" },
      {
        type: "object",
        required: ["data"],
        properties: {
          data: { $ref: "#/components/schemas/SdkWorkPageData" },
        },
      },
    ],
  };
}

function rechargeOrderCreateCommandSchema() {
  return {
    type: "object",
    additionalProperties: false,
    properties: {
      subject: {
        type: "string",
        enum: [
          "points_recharge",
          "token_bank_recharge",
          "token_bank_plan_purchase",
          "token_bank_plan_renewal",
          "account_recharge_package",
          "coupon_recharge",
        ],
        default: "points_recharge",
      },
      targetAsset: {
        type: "string",
        enum: ["points", "token_bank", "cash"],
      },
      amount: { oneOf: [{ type: "string" }, { type: "number" }] },
      grantAmount: { oneOf: [{ type: "string" }, { type: "number" }] },
      clientRequestNo: { type: "string" },
      currencyCode: { type: "string" },
      packageId: { type: "string" },
      planCode: { type: "string" },
      planPeriod: {
        type: "string",
        enum: [
          "monthly",
          "quarterly",
          "yearly",
          "continuous_monthly",
          "continuous_yearly",
        ],
      },
      couponCode: { type: "string" },
      source: { type: "string" },
      paymentMethod: { type: "string" },
      paymentPassword: { type: "string" },
    },
  };
}

function refundRequestCreateCommandSchema() {
  return {
    type: "object",
    additionalProperties: false,
    required: ["originalOrderId", "targetAsset", "amount", "currencyCode"],
    properties: {
      originalOrderId: { type: "string" },
      targetAsset: { type: "string", enum: ["points", "token_bank", "cash"] },
      amount: { oneOf: [{ type: "string" }, { type: "number" }] },
      currencyCode: { type: "string" },
      reasonCode: { type: "string" },
      reasonDetail: { type: "string" },
    },
  };
}

function withdrawalRequestCreateCommandSchema() {
  return {
    type: "object",
    additionalProperties: false,
    required: ["amount", "currencyCode"],
    properties: {
      asset: { type: "string", enum: ["cash"], default: "cash" },
      amount: { oneOf: [{ type: "string" }, { type: "number" }] },
      currencyCode: { type: "string" },
      payoutMethod: { type: "string" },
      payoutAccountRef: { type: "string" },
      reasonCode: { type: "string" },
    },
  };
}

function accountValuePackageWriteCommandSchema() {
  return {
    type: "object",
    additionalProperties: false,
    required: [
      "packageCode",
      "displayName",
      "targetAsset",
      "grantAmount",
      "priceAmount",
      "currencyCode",
    ],
    properties: {
      packageCode: { type: "string" },
      displayName: { type: "string" },
      targetAsset: { type: "string", enum: ["points", "token_bank", "cash"] },
      grantAmount: { type: "string" },
      bonusAmount: { type: "string" },
      priceAmount: { type: "string" },
      currencyCode: { type: "string" },
      status: { type: "string" },
      sortWeight: { type: "integer" },
      validFrom: { type: "string" },
      validTo: { type: "string" },
    },
  };
}

function tokenBankPlanWriteCommandSchema() {
  return {
    type: "object",
    additionalProperties: false,
    required: [
      "planCode",
      "displayName",
      "planPeriod",
      "grantAmount",
      "priceAmount",
      "currencyCode",
    ],
    properties: {
      planCode: { type: "string" },
      displayName: { type: "string" },
      planPeriod: {
        type: "string",
        enum: [
          "monthly",
          "quarterly",
          "yearly",
          "continuous_monthly",
          "continuous_yearly",
        ],
      },
      grantAmount: { type: "string" },
      bonusAmount: { type: "string" },
      priceAmount: { type: "string" },
      currencyCode: { type: "string" },
      renewalPolicy: { type: "string" },
      status: { type: "string" },
      sortWeight: { type: "integer" },
    },
  };
}

function accountValueRequestReviewCommandSchema() {
  return {
    type: "object",
    additionalProperties: false,
    properties: {
      reasonCode: { type: "string" },
      reviewComment: { type: "string" },
    },
  };
}

function accountValuePackageResponseSchema() {
  return {
    type: "object",
    additionalProperties: false,
    properties: {
      packageId: { type: "string" },
      packageCode: { type: "string" },
      displayName: { type: "string" },
      targetAsset: { type: "string" },
      grantAmount: { type: "string" },
      bonusAmount: { type: "string" },
      priceAmount: { type: "string" },
      currencyCode: { type: "string" },
      status: { type: "string" },
    },
  };
}

function tokenBankPlanResponseSchema() {
  return {
    type: "object",
    additionalProperties: false,
    properties: {
      planCode: { type: "string" },
      displayName: { type: "string" },
      planPeriod: { type: "string" },
      grantAmount: { type: "string" },
      bonusAmount: { type: "string" },
      priceAmount: { type: "string" },
      currencyCode: { type: "string" },
      renewalPolicy: { type: "string" },
      status: { type: "string" },
    },
  };
}

function accountValueRequestResponseSchema() {
  return {
    type: "object",
    additionalProperties: false,
    properties: {
      accountValueRequestId: { type: "string" },
      requestNo: { type: "string" },
      originalOrderId: { type: "string" },
      ownerUserId: { type: "string" },
      subject: { type: "string" },
      targetAsset: { type: "string" },
      amount: { type: "string" },
      currencyCode: { type: "string" },
      status: { type: "string" },
      providerReferenceId: { type: "string" },
      createdAt: { type: "string" },
      updatedAt: { type: "string" },
    },
  };
}

function writeJson(targetPath, value) {
  fs.writeFileSync(targetPath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

const appAuthority = JSON.parse(fs.readFileSync(appAuthorityPath, "utf8"));
const patchedApp = patchAppSpec(appAuthority);
writeJson(appAuthorityPath, patchedApp);
writeJson(appSdkOpenApiPath, patchedApp);
writeJson(appSdkGenPath, patchedApp);

const backendAuthority = JSON.parse(fs.readFileSync(backendAuthorityPath, "utf8"));
const patchedBackend = patchBackendSpec(backendAuthority);
writeJson(backendAuthorityPath, patchedBackend);
writeJson(backendSdkOpenApiPath, patchedBackend);

console.log(
  `Materialized ${Object.keys(appPaths).length} app paths and ${Object.keys(backendPaths).length} backend paths into order OpenAPI.`,
);
