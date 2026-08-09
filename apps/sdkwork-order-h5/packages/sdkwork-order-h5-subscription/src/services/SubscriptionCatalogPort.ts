import { createClient, type SdkworkAppClient as SdkworkMembershipAppClient } from "@sdkwork/membership-app-sdk";
import { createClient as createOrderAppClient, type SdkworkAppClient as SdkworkOrderAppClient } from "@sdkwork/order-app-sdk";

/**
 * Subscription catalog port: the order-domain H5 surface consumes catalog
 * data from the owning domains (membership packages, token bank plans)
 * without coupling the purchase UI to any concrete domain SDK.
 *
 * The default implementation uses the composed domain SDK clients; product
 * hosts may inject their own instances instead.
 */
export interface MembershipPackage {
  id: string;
  name: string;
  price: string;
  originalPrice?: string;
  durationDays: number;
  planName?: string;
  recommended?: boolean;
  tags?: string[];
}

export interface TokenBankPlan {
  planCode: string;
  displayName: string;
  planPeriod: string;
  grantAmount: string;
  bonusAmount: string;
  priceAmount: string;
  currencyCode: string;
  renewalPolicy: string;
}

export interface SubscriptionCatalogPort {
  listMembershipPackages(): Promise<MembershipPackage[]>;
  listTokenBankPlans(): Promise<TokenBankPlan[]>;
}

function readString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

/** Membership package ids are numeric (`external_id`); normalize to string. */
function readId(value: unknown): string {
  if (typeof value === "number" && Number.isFinite(value)) {
    return String(value);
  }
  return readString(value) ?? "";
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

export interface CreateSubscriptionCatalogPortOptions {
  membershipAppSdkClient?: SdkworkMembershipAppClient;
  orderAppSdkClient?: SdkworkOrderAppClient;
}

export function createDefaultSubscriptionCatalogPort(
  options: CreateSubscriptionCatalogPortOptions = {},
): SubscriptionCatalogPort {
  const membershipClient = options.membershipAppSdkClient ?? createClient({ baseUrl: "/" });
  const orderClient = options.orderAppSdkClient ?? createOrderAppClient({ baseUrl: "/" });

  return {
    async listMembershipPackages(): Promise<MembershipPackage[]> {
      const response = await membershipClient.memberships.packages.list({ page: 1, pageSize: 200 });
      const items = (response as { items?: unknown[] }).items ?? [];
      return items.map((item) => {
        const record = item as Record<string, unknown>;
        return {
          id: readId(record.id),
          name: readString(record.name) ?? "",
          price: toNumberString(record.price),
          ...(readString(record.originalPrice) ? { originalPrice: readString(record.originalPrice) } : {}),
          durationDays: toNumber(record.durationDays),
          ...(readString(record.planName) ? { planName: readString(record.planName) } : {}),
          ...(toBoolean(record.recommended) ? { recommended: true } : {}),
          ...(Array.isArray(record.tags) ? { tags: record.tags.map(String) } : {}),
        };
      });
    },

    async listTokenBankPlans(): Promise<TokenBankPlan[]> {
      const response = await orderClient.recharges.plans.list({ page: 1, pageSize: 200 });
      const items = (response as { items?: unknown[] }).items ?? [];
      return items.map((item) => {
        const record = item as Record<string, unknown>;
        return {
          planCode: readString(record.planCode) ?? readString(record.id) ?? "",
          displayName: readString(record.displayName) ?? readString(record.name) ?? "",
          planPeriod: readString(record.planPeriod) ?? "",
          grantAmount: toNumberString(record.grantAmount),
          bonusAmount: toNumberString(record.bonusAmount),
          priceAmount: toNumberString(record.priceAmount),
          currencyCode: readString(record.currencyCode) ?? "CNY",
          renewalPolicy: readString(record.renewalPolicy) ?? "",
        };
      });
    },
  };
}
