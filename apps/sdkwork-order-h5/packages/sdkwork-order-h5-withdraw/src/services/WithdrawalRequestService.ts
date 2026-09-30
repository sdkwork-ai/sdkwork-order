import {
  createClient as createOrderAppClient,
  type SdkworkAppClient as SdkworkOrderAppClient,
  type SdkworkAppConfig,
} from "@sdkwork/order-app-sdk";
import {
  createSdkworkIdempotencyParams,
  createSdkworkOrderAppService,
  unwrapSdkworkOrderResource,
  type SdkworkOrderAppService,
} from "@sdkwork/order-service";

export interface WithdrawalRequestInput {
  amount: string;
  currencyCode: string;
  payoutMethod?: string;
  payoutAccountRef?: string;
  reasonCode?: string;
}

export interface WithdrawalRequestResult {
  withdrawalRequestId: string;
  requestNo?: string;
  status: string;
  [key: string]: unknown;
}

export interface WithdrawalRequestPort {
  createWithdrawalRequest(input: WithdrawalRequestInput): Promise<WithdrawalRequestResult>;
  retrieveWithdrawalRequest(withdrawalRequestId: string): Promise<WithdrawalRequestResult>;
}

export interface CreateWithdrawalRequestServiceOptions {
  appService?: SdkworkOrderAppService;
  appConfig?: Partial<SdkworkAppConfig>;
  orderAppSdkClient?: SdkworkOrderAppClient;
}

function resolveAppService(options: CreateWithdrawalRequestServiceOptions): SdkworkOrderAppService {
  if (options.appService) {
    return options.appService;
  }
  const client = options.orderAppSdkClient ?? createOrderAppClient({
    baseUrl: options.appConfig?.baseUrl ?? "/",
    tokenManager: options.appConfig?.tokenManager,
    accessToken: options.appConfig?.accessToken,
    authToken: options.appConfig?.authToken,
    tenantId: options.appConfig?.tenantId,
    organizationId: options.appConfig?.organizationId,
    platform: "h5",
    authMode: options.appConfig?.authMode ?? "dual-token",
  } as SdkworkAppConfig);
  return createSdkworkOrderAppService({ appClient: client as unknown as SdkworkOrderAppClient });
}

/**
 * Order-domain cash withdrawal request service. Cash withdrawal is owned by
 * sdkwork-order `withdrawals.requests` flows (see
 * `specs/ACCOUNT_VALUE_ORDER_SPEC.md`): the request is created through the
 * order app SDK with an idempotency key; sdkwork-account holds withdrawable
 * cash and sdkwork-payment executes the provider payout only when order
 * orchestrates the lifecycle.
 */
export function createWithdrawalRequestService(
  options: CreateWithdrawalRequestServiceOptions = {},
): WithdrawalRequestPort {
  const appService = resolveAppService(options);

  return {
    createWithdrawalRequest: async (input) => {
      const params = createSdkworkIdempotencyParams();
      const response = await appService.withdrawals.requests.create(
        {
          asset: "cash",
          amount: input.amount,
          currencyCode: input.currencyCode,
          payoutMethod: input.payoutMethod,
          payoutAccountRef: input.payoutAccountRef,
          reasonCode: input.reasonCode,
        },
        params,
      );
      return normalizeWithdrawalRequest(response);
    },
    retrieveWithdrawalRequest: async (withdrawalRequestId) => {
      const response = await appService.withdrawals.requests.retrieve(withdrawalRequestId);
      return normalizeWithdrawalRequest(response);
    },
  };
}

function normalizeWithdrawalRequest(response: unknown): WithdrawalRequestResult {
  // `unwrapSdkworkOrderResource` (shared with the composed order service)
  // unwraps the SdkWork envelope (`data.item`) and tolerates already-unwrapped
  // payloads, so the H5 surface never reads a bare `{ item }` wrapper as the
  // record (which silently produced an empty withdrawal id).
  const record = unwrapSdkworkOrderResource<Record<string, unknown>>(
    response,
    "Withdrawal request response is invalid.",
  );
  const source = (record ?? {}) as Record<string, unknown>;
  return {
    withdrawalRequestId: String(source.withdrawalRequestId ?? source.id ?? ""),
    requestNo: source.requestNo != null ? String(source.requestNo) : undefined,
    status: String(source.status ?? "requested"),
    ...source,
  };
}
