import {
  createClient as createOrderAppClient,
  type SdkworkAppClient as SdkworkOrderAppClient,
  type SdkworkAppConfig,
} from "@sdkwork/order-app-sdk";
import {
  createSdkworkIdempotencyParams,
  createSdkworkOrderAppService,
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
  const data = (response as { data?: Record<string, unknown> })?.data ?? response;
  const record = (data ?? {}) as Record<string, unknown>;
  return {
    withdrawalRequestId: String(record.withdrawalRequestId ?? record.id ?? ""),
    requestNo: record.requestNo != null ? String(record.requestNo) : undefined,
    status: String(record.status ?? "requested"),
    ...record,
  };
}
