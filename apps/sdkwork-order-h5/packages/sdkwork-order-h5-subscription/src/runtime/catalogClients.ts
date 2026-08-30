import { createClient, type SdkworkAppClient as SdkworkMembershipAppClient } from "@sdkwork/membership-app-sdk";
import { createClient as createOrderAppClient, type SdkworkAppClient as SdkworkOrderAppClient } from "@sdkwork/order-app-sdk";

/**
 * Runtime / composition root for the subscription surface.
 *
 * SDK construction belongs here (runtime/bootstrap/core), never in the
 * service layer. Services receive the injected clients via options.
 */
export interface SubscriptionRuntimeClients {
  membershipAppSdkClient: SdkworkMembershipAppClient;
  orderAppSdkClient: SdkworkOrderAppClient;
}

export function createDefaultSubscriptionRuntimeClients(): SubscriptionRuntimeClients {
  return {
    membershipAppSdkClient: createClient({ baseUrl: "/" }),
    orderAppSdkClient: createOrderAppClient({ baseUrl: "/" }),
  };
}