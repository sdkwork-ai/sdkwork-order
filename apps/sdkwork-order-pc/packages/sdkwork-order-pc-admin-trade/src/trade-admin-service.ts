import type {
  AccountValuePackageResponse,
  AccountValuePackageWriteCommand,
  AccountValueRequestResponse,
  AccountValueRequestReviewCommand,
  AfterSalesRequestSummary,
  CreateShipmentPackageRequest,
  OrderCancellation,
  OrderSummary,
  ReviewAfterSalesRequest,
  ShipmentPackageSummary,
  ShipmentSummary,
  SdkworkOrderBackendClient,
  TokenBankPlanResponse,
  TokenBankPlanWriteCommand,
  UpdateShipmentPackageRequest,
} from "@sdkwork/order-pc-admin-core";
import {
  createSdkworkIdempotencyParams,
  resolveSdkworkOffsetPagination,
  unwrapSdkworkOrderListPage,
} from "@sdkwork/order-service";
import {
  createTradeOperationsService,
  type TradeOperationsPage,
  type TradeOperationsQuery,
  type TradeRequestAction,
} from "@sdkwork/order-pc-admin-orders/service";

export type { TradeOperationsPage, TradeOperationsQuery, TradeRequestAction };

export type TradeReviewAction = TradeRequestAction;

/** Extended list query with domain-specific filters beyond status. */
export interface TradeAdminListQuery extends TradeOperationsQuery {
  afterSalesType?: string;
  fulfillmentId?: string;
  orderId?: string;
}

/** Extended list query for the account value package catalog. */
export interface AccountValuePackageListQuery extends TradeOperationsQuery {
  targetAsset?: string;
}

/** Review body for refund/withdrawal requests. */
export interface TradeRequestReviewInput {
  reasonCode?: string;
  reviewComment?: string;
}

/** Review body for after-sales requests (approve may carry a partial refund). */
export interface AfterSalesReviewInput {
  action: "approve" | "reject";
  approvedAmount?: string;
  reasonCode?: string;
  reasonDetail?: string;
  reviewComment?: string;
}

/** Status presets used by the workbench pending aggregation. */
export const TRADE_PENDING_STATUS = {
  afterSales: "submitted",
  refunds: "pending",
  shipments: "created",
  withdrawals: "pending",
  cancellations: "pending",
} as const;

export interface TradeWorkbenchSummary {
  pendingAfterSales: number;
  pendingRefunds: number;
  pendingShipments: number;
  pendingWithdrawals: number;
  pendingCancellations: number;
  recentOrders: OrderSummary[];
}

export interface TradeAdminService {
  getWorkbenchSummary(): Promise<TradeWorkbenchSummary>;
  listAfterSales(query?: TradeAdminListQuery): Promise<TradeOperationsPage<AfterSalesRequestSummary>>;
  getAfterSales(afterSalesRequestId: string): Promise<AfterSalesRequestSummary>;
  reviewAfterSales(afterSalesRequestId: string, input: AfterSalesReviewInput, idempotencyKey?: string): Promise<AfterSalesRequestSummary>;
  listShipments(query?: TradeAdminListQuery): Promise<TradeOperationsPage<ShipmentSummary>>;
  getShipment(shipmentId: string): Promise<ShipmentSummary>;
  listShipmentPackages(shipmentId: string): Promise<TradeOperationsPage<ShipmentPackageSummary>>;
  createShipmentPackage(shipmentId: string, body: CreateShipmentPackageRequest): Promise<ShipmentPackageSummary>;
  updateShipmentPackage(
    shipmentId: string,
    packageId: string,
    body: UpdateShipmentPackageRequest,
  ): Promise<ShipmentPackageSummary>;
  listRefundRequests(query?: TradeOperationsQuery): Promise<TradeOperationsPage<AccountValueRequestResponse>>;
  reviewRefundRequest(id: string, action: TradeReviewAction, input?: TradeRequestReviewInput, idempotencyKey?: string): Promise<void>;
  listWithdrawalRequests(query?: TradeOperationsQuery): Promise<TradeOperationsPage<AccountValueRequestResponse>>;
  reviewWithdrawalRequest(id: string, action: TradeReviewAction, input?: TradeRequestReviewInput, idempotencyKey?: string): Promise<void>;
  listCancellations(query?: TradeOperationsQuery): Promise<TradeOperationsPage<OrderCancellation>>;
  listAccountValuePackages(query?: AccountValuePackageListQuery): Promise<TradeOperationsPage<AccountValuePackageResponse>>;
  createAccountValuePackage(body: AccountValuePackageWriteCommand): Promise<AccountValuePackageResponse>;
  updateAccountValuePackage(
    packageId: string,
    body: AccountValuePackageWriteCommand,
  ): Promise<AccountValuePackageResponse>;
  retireAccountValuePackage(packageId: string): Promise<void>;
  listTokenBankPlans(query?: TradeOperationsQuery): Promise<TradeOperationsPage<TokenBankPlanResponse>>;
  createTokenBankPlan(body: TokenBankPlanWriteCommand): Promise<TokenBankPlanResponse>;
  updateTokenBankPlan(planCode: string, body: TokenBankPlanWriteCommand): Promise<TokenBankPlanResponse>;
  retireTokenBankPlan(planCode: string): Promise<void>;
}

function reviewBody(input: TradeRequestReviewInput | undefined): AccountValueRequestReviewCommand {
  const body: AccountValueRequestReviewCommand = {};
  if (input?.reasonCode) body.reasonCode = input.reasonCode;
  if (input?.reviewComment) body.reviewComment = input.reviewComment;
  return body;
}

export function createTradeAdminService(client: SdkworkOrderBackendClient): TradeAdminService {
  const operations = createTradeOperationsService(client);

  const listPackages = async (
    shipmentId: string,
  ): Promise<TradeOperationsPage<ShipmentPackageSummary>> => {
    const { items, pageInfo } = await client.shipments.packages.management.list(shipmentId, {
      page: "1",
      pageSize: "200",
    });
    return {
      items,
      page: Number(pageInfo.page ?? 1),
      pageSize: Number(pageInfo.pageSize ?? items.length),
      totalItems: Number(pageInfo.totalItems ?? items.length),
      totalPages: Number(pageInfo.totalPages ?? 1),
    };
  };

  const listAdminPage = async <T>(
    query: TradeOperationsQuery | TradeAdminListQuery,
    loader: () => Promise<unknown>,
  ): Promise<TradeOperationsPage<T>> => {
    const page = query.page ?? 1;
    const pageSize = query.pageSize ?? 20;
    const listPage = unwrapSdkworkOrderListPage<T>(await loader());
    const pagination = resolveSdkworkOffsetPagination(listPage.pageInfo, page, pageSize);
    return {
      items: listPage.items,
      page: pagination.page,
      pageSize: pagination.pageSize,
      totalItems: pagination.total,
      totalPages: pagination.totalPages,
    };
  };

  const countPending = async (loader: () => Promise<{ totalItems: number }>): Promise<number> => {
    try {
      const page = await loader();
      return Number(page.totalItems ?? 0);
    } catch {
      return 0;
    }
  };

  return {
    getWorkbenchSummary: async () => {
      const [pendingAfterSales, pendingRefunds, pendingShipments, pendingWithdrawals, pendingCancellations, recentPage] =
        await Promise.all([
          countPending(async () => {
            const page = await client.afterSales.management.list({
              page: "1",
              pageSize: "1",
              status: TRADE_PENDING_STATUS.afterSales,
            });
            return { totalItems: Number(page.pageInfo.totalItems ?? 0) };
          }),
          countPending(async () => {
            const page = await client.backend.refundRequests.list({ page: 1, pageSize: 1, status: TRADE_PENDING_STATUS.refunds });
            const listPage = unwrapSdkworkOrderListPage<AccountValueRequestResponse>(page);
            return { totalItems: resolveSdkworkOffsetPagination(listPage.pageInfo, 1, 1).total };
          }),
          countPending(async () => {
            const page = await client.shipments.list({
              page: "1",
              pageSize: "1",
              status: TRADE_PENDING_STATUS.shipments,
            });
            return { totalItems: Number(page.pageInfo.totalItems ?? 0) };
          }),
          countPending(async () => {
            const page = await client.backend.withdrawalRequests.list({ page: 1, pageSize: 1, status: TRADE_PENDING_STATUS.withdrawals });
            const listPage = unwrapSdkworkOrderListPage<AccountValueRequestResponse>(page);
            return { totalItems: resolveSdkworkOffsetPagination(listPage.pageInfo, 1, 1).total };
          }),
          countPending(async () => {
            const page = await client.orders.admin.cancellations.list({
              page: "1",
              pageSize: "1",
              status: TRADE_PENDING_STATUS.cancellations,
            });
            const listPage = unwrapSdkworkOrderListPage<OrderCancellation>(page);
            return { totalItems: resolveSdkworkOffsetPagination(listPage.pageInfo, 1, 1).total };
          }),
          client.orders.admin.list({ page: "1", pageSize: "5" }).then((page) => unwrapSdkworkOrderListPage<OrderSummary>(page)),
        ]);
      return {
        pendingAfterSales,
        pendingRefunds,
        pendingShipments,
        pendingWithdrawals,
        pendingCancellations,
        recentOrders: recentPage.items,
      };
    },
    listAfterSales: (query = {}) => listAdminPage<AfterSalesRequestSummary>(query, () =>
      client.afterSales.management.list({
        page: String(query.page ?? 1),
        pageSize: String(query.pageSize ?? 20),
        status: query.status,
        afterSalesType: query.afterSalesType,
        orderId: query.orderId,
      })),
    getAfterSales: (afterSalesRequestId) => client.afterSales.management.retrieve(afterSalesRequestId),
    reviewAfterSales: (afterSalesRequestId, input, idempotencyKey) => {
      const body: ReviewAfterSalesRequest = {
        reviewAction: input.action,
      };
      if (input.approvedAmount) body.approvedAmount = input.approvedAmount;
      if (input.reasonCode) body.reasonCode = input.reasonCode;
      if (input.reasonDetail) body.reasonDetail = input.reasonDetail;
      if (input.reviewComment) body.reviewComment = input.reviewComment;
      return client.afterSales.reviews.create(afterSalesRequestId, body, createSdkworkIdempotencyParams(idempotencyKey));
    },
    listShipments: (query = {}) => listAdminPage<ShipmentSummary>(query, () =>
      client.shipments.list({
        page: String(query.page ?? 1),
        pageSize: String(query.pageSize ?? 20),
        status: query.status,
        orderId: query.orderId,
        fulfillmentId: query.fulfillmentId,
      })),
    getShipment: (shipmentId) => client.shipments.retrieve(shipmentId),
    listShipmentPackages: (shipmentId) => listPackages(shipmentId),
    createShipmentPackage: (shipmentId, body) =>
      client.shipments.packages.create(shipmentId, body, createSdkworkIdempotencyParams()),
    updateShipmentPackage: (shipmentId, packageId, body) =>
      client.shipments.packages.update(shipmentId, packageId, body, createSdkworkIdempotencyParams()),
    listRefundRequests: (query = {}) => operations.listRefundRequests(query),
    reviewRefundRequest: (id, action, input, idempotencyKey) => {
      const body = reviewBody(input);
      const api = client.backend.refundRequests;
      const method = action === "approve" ? api.approve : action === "reject" ? api.reject : api.retry;
      return method(id, createSdkworkIdempotencyParams(idempotencyKey), body).then(() => undefined);
    },
    listWithdrawalRequests: (query = {}) => operations.listWithdrawalRequests(query),
    reviewWithdrawalRequest: (id, action, input, idempotencyKey) => {
      const body = reviewBody(input);
      const api = client.backend.withdrawalRequests;
      const method = action === "approve" ? api.approve : action === "reject" ? api.reject : api.retry;
      return method(id, createSdkworkIdempotencyParams(idempotencyKey), body).then(() => undefined);
    },
    listCancellations: (query = {}) => listAdminPage<OrderCancellation>(query, () =>
      client.orders.admin.cancellations.list({
        page: String(query.page ?? 1),
        pageSize: String(query.pageSize ?? 20),
        status: query.status,
      })),
    listAccountValuePackages: (query = {}) => listAdminPage<AccountValuePackageResponse>(query, () =>
      client.backend.accountValuePackages.list({
        page: query.page ?? 1,
        pageSize: query.pageSize ?? 20,
        status: query.status,
        targetAsset: query.targetAsset,
      })),
    createAccountValuePackage: (body) =>
      client.backend.accountValuePackages.create(body, createSdkworkIdempotencyParams()),
    updateAccountValuePackage: (packageId, body) =>
      client.backend.accountValuePackages.update(packageId, body, createSdkworkIdempotencyParams()),
    retireAccountValuePackage: (packageId) =>
      client.backend.accountValuePackages.retire(packageId, createSdkworkIdempotencyParams()).then(() => undefined),
    listTokenBankPlans: (query = {}) => listAdminPage<TokenBankPlanResponse>(query, () =>
      client.backend.tokenBankPlans.list({
        page: query.page ?? 1,
        pageSize: query.pageSize ?? 20,
        status: query.status,
      })),
    createTokenBankPlan: (body) =>
      client.backend.tokenBankPlans.create(body, createSdkworkIdempotencyParams()),
    updateTokenBankPlan: (planCode, body) =>
      client.backend.tokenBankPlans.update(planCode, body, createSdkworkIdempotencyParams()),
    retireTokenBankPlan: (planCode) =>
      client.backend.tokenBankPlans.retire(planCode, createSdkworkIdempotencyParams()).then(() => undefined),
  };
}
