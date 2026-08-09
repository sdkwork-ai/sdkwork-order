import {
  createSdkworkMembershipCheckoutService,
  createSdkworkOrderAppService,
  type SdkworkMembershipCheckoutPayment,
  type SdkworkOrderAppService,
} from "@sdkwork/order-service";
import { createClient as createOrderAppClient, type SdkworkAppClient as SdkworkOrderAppClient, type SdkworkAppConfig } from "@sdkwork/order-app-sdk";
import { createClient as createMembershipAppClient, type SdkworkAppClient as SdkworkMembershipAppClient } from "@sdkwork/membership-app-sdk";

/**
 * PC subscription purchase port for the order domain: membership package
 * catalog data comes from the membership domain; checkout (order creation,
 * payment, status polling) is handled by the order-domain service.
 */
export interface PcSubscriptionPackage {
  id: string;
  name: string;
  price: string;
  originalPrice?: string;
  durationDays: number;
  planName?: string;
  recommended?: boolean;
}

export interface PcSubscriptionPurchasePort {
  listPackages(): Promise<PcSubscriptionPackage[]>;
  createCheckout(packageId: string): Promise<SdkworkMembershipCheckoutPayment>;
  getCheckoutStatus(orderId: string): Promise<SdkworkMembershipCheckoutPayment>;
}

function readString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function toNumberString(value: unknown): string {
  return typeof value === "number" ? String(value) : readString(value) ?? "0";
}

function toNumber(value: unknown): number {
  if (typeof value === "number") {
    return value;
  }
  const parsed = Number(readString(value));
  return Number.isFinite(parsed) ? parsed : 0;
}

function toBoolean(value: unknown): boolean {
  return value === true || value === "true" || value === 1 || value === "1";
}

export interface CreatePcSubscriptionPurchaseServiceOptions {
  appService?: SdkworkOrderAppService;
  appConfig?: Partial<SdkworkAppConfig>;
  membershipAppSdkClient?: SdkworkMembershipAppClient;
}

export function createPcSubscriptionPurchaseService(
  options: CreatePcSubscriptionPurchaseServiceOptions = {},
): PcSubscriptionPurchasePort {
  const appService = options.appService
    ?? createSdkworkOrderAppService({
      appClient: createOrderAppClient({
        baseUrl: options.appConfig?.baseUrl ?? "/",
        tokenManager: options.appConfig?.tokenManager,
        accessToken: options.appConfig?.accessToken,
        authToken: options.appConfig?.authToken,
        tenantId: options.appConfig?.tenantId,
        organizationId: options.appConfig?.organizationId,
        platform: "pc",
        authMode: options.appConfig?.authMode ?? "dual-token",
      } as SdkworkAppConfig) as unknown as SdkworkOrderAppClient,
    });
  const membershipClient = options.membershipAppSdkClient
    ?? createMembershipAppClient({ baseUrl: options.appConfig?.baseUrl ?? "/" });
  const checkoutService = createSdkworkMembershipCheckoutService({ appService });

  return {
    async listPackages(): Promise<PcSubscriptionPackage[]> {
      const response = await membershipClient.memberships.packages.list({ page: 1, pageSize: 200 });
      const items = (response as { items?: unknown[] }).items ?? [];
      return items.map((item) => {
        const record = item as Record<string, unknown>;
        return {
          id: readString(record.id) ?? "",
          name: readString(record.name) ?? "",
          price: toNumberString(record.price),
          ...(readString(record.originalPrice) ? { originalPrice: readString(record.originalPrice) } : {}),
          durationDays: toNumber(record.durationDays),
          ...(readString(record.planName) ? { planName: readString(record.planName) } : {}),
          ...(toBoolean(record.recommended) ? { recommended: true } : {}),
        };
      });
    },

    createCheckout: (packageId) =>
      checkoutService.createCheckout({
        action: "purchase",
        packageId: Number(packageId),
        paymentProduct: "mobile_cashier_h5",
      }),

    getCheckoutStatus: (orderId) => checkoutService.getCheckoutStatus(orderId),
  };
}
