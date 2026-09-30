import {
  configureSdkworkOrderSessionTokenProvider,
  type SdkworkOrderAppService,
  type SdkworkOrderSessionTokens,
} from "@sdkwork/order-service";

type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends (...args: infer TArgs) => infer TReturn
    ? (...args: TArgs) => TReturn
    : DeepPartial<T[K]>;
};

export function createOrderAppServiceMock(
  overrides: DeepPartial<SdkworkOrderAppService> = {},
): SdkworkOrderAppService {
  const base: SdkworkOrderAppService = {
    checkout: createMissingCheckoutTree(),
    memberships: {} as SdkworkOrderAppService["memberships"],
    orders: createMissingOrdersTree(),
    recharges: createMissingRechargesTree(),
    shipments: {} as SdkworkOrderAppService["shipments"],
    withdrawals: {} as SdkworkOrderAppService["withdrawals"],
  };
  return mergeOrderAppService(base, overrides);
}

export function configureOrderServiceMockSession(
  tokens: SdkworkOrderSessionTokens = { authToken: "order-auth-token" },
): void {
  configureSdkworkOrderSessionTokenProvider(() => tokens);
}

export function resetOrderServiceMockSession(): void {
  configureSdkworkOrderSessionTokenProvider(null);
}

function createMissingCheckoutTree(): SdkworkOrderAppService["checkout"] {
  const tree: Record<string, unknown> = {};
  for (const method of [
    "sessions.create",
    "sessions.retrieve",
    "sessions.quotes.create",
    "sessions.orders.create",
  ]) {
    addMissingMethod(tree, method, "checkout");
  }
  return tree as unknown as SdkworkOrderAppService["checkout"];
}

function createMissingOrdersTree(): SdkworkOrderAppService["orders"] {
  const tree: Record<string, unknown> = {};
  for (const method of [
    "list",
    "retrieve",
    "payments.create",
    "cancellations.create",
    "paymentSuccess.retrieve",
    "statistics.retrieve",
    "status.retrieve",
  ]) {
    addMissingMethod(tree, method);
  }
  return tree as unknown as SdkworkOrderAppService["orders"];
}

function createMissingRechargesTree(): SdkworkOrderAppService["recharges"] {
  const tree: Record<string, unknown> = {};
  for (const method of [
    "packages.list",
    "settings.retrieve",
    "orders.list",
    "orders.create",
    "orders.retrieve",
    "orders.cancel",
  ]) {
    addMissingMethod(tree, method, "recharges");
  }
  return tree as unknown as SdkworkOrderAppService["recharges"];
}

function addMissingMethod(root: Record<string, unknown>, method: string, prefix = "orders"): void {
  let node = root;
  const segments = method.split(".");
  for (const segment of segments.slice(0, -1)) {
    if (!node[segment] || typeof node[segment] === "function") {
      node[segment] = {};
    }
    node = node[segment] as Record<string, unknown>;
  }
  node[segments.at(-1)!] = async () => {
    throw new Error(`Missing order service test method: ${prefix}.${method}`);
  };
}

function mergeOrderAppService<T>(base: T, overrides: DeepPartial<T>): T {
  for (const [key, value] of Object.entries(overrides as Record<string, unknown>)) {
    if (
      value &&
      typeof value === "object" &&
      !Array.isArray(value) &&
      typeof (base as Record<string, unknown>)[key] === "object"
    ) {
      mergeOrderAppService((base as Record<string, unknown>)[key], value as DeepPartial<unknown>);
    } else {
      (base as Record<string, unknown>)[key] = value;
    }
  }
  return base;
}
