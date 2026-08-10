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

/** Membership package group (billing-cycle tabs on the H5 subscription page). */
export interface MembershipPackageGroup {
  id: string;
  name: string;
  description?: string;
  packages: MembershipPackage[];
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
  listMembershipPackageGroups(): Promise<MembershipPackageGroup[]>;
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

/** Normalize an `AppMembershipPackageItem` record into the UI package shape. */
function toMembershipPackage(record: Record<string, unknown>): MembershipPackage {
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
}

/** Normalize an `AppMembershipPackageGroupItem` record into the UI group shape. */
function toMembershipPackageGroup(record: Record<string, unknown>): MembershipPackageGroup {
  return {
    id: readId(record.id),
    name: readString(record.name) ?? "",
    ...(readString(record.description) ? { description: readString(record.description) } : {}),
    packages: Array.isArray(record.packages)
      ? record.packages.map((item) => toMembershipPackage(item as Record<string, unknown>))
      : [],
  };
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
      return items.map((item) => toMembershipPackage(item as Record<string, unknown>));
    },

    async listMembershipPackageGroups(): Promise<MembershipPackageGroup[]> {
      const response = await membershipClient.memberships.packageGroups.list({ page: 1, pageSize: 200 });
      const items = (response as { items?: unknown[] }).items ?? [];
      return items.map((item) => toMembershipPackageGroup(item as Record<string, unknown>));
    },

    async listTokenBankPlans(): Promise<TokenBankPlan[]> {
      // Token Bank 充值套餐与 CloudRouter 控制台"算力积分购买"同源：
      // 读积分包目录（`recharges/packages`），由后端按充值 settings
      // 计算每档到账算力积分（grant_amount / points）。
      const response = await orderClient.recharges.packages.list({ page: 1, pageSize: 200 });
      const items = (response as { items?: unknown[] }).items ?? [];
      return items.map((item) => {
        const record = item as Record<string, unknown>;
        const id = readString(record.id) ?? "";
        const points = toNumber(record.points ?? record.grantAmount);
        const bonus = toNumber(record.bonusPoints ?? 0);
        return {
          planCode: id,
          // 展示名由 UI 按当前语言拼装（`subscription.points_display`），
          // 这里只保留到账点数，避免把语言硬编码进目录数据。
          displayName: String(points),
          planPeriod: "once",
          grantAmount: String(points),
          bonusAmount: String(bonus),
          priceAmount: toNumberString(record.priceAmount ?? record.price),
          currencyCode: readString(record.currencyCode) ?? "CNY",
          renewalPolicy: "non_renewable",
        };
      });
    },
  };
}
