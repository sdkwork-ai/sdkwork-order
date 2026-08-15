import type {
  AdminCreateRefundRequest,
  CancelOrderRequest,
  CloseOrderRequest,
  OrderDetail,
  OrderEvent,
  OrderSummary,
  ShipmentSummary,
  SdkworkOrderBackendClient,
} from "@sdkwork/order-pc-admin-core";
import {
  createSdkworkIdempotencyParams,
  resolveSdkworkOffsetPagination,
  unwrapSdkworkOrderListPage,
  unwrapSdkworkOrderResource,
} from "@sdkwork/order-service";

export interface OrderAdminListQuery {
  page?: number;
  pageSize?: number;
  status?: string;
  q?: string;
  /** Inclusive lower bound on the order creation time (ISO date-time). */
  createdFrom?: string;
  /** Inclusive upper bound on the order creation time (ISO date-time). */
  createdTo?: string;
}

export interface OrderAdminListResult {
  items: OrderSummary[];
  page: number;
  pageSize: number;
  totalItems: number;
  totalPages: number;
}

export interface OrderAdminService {
  listOrders(query: OrderAdminListQuery): Promise<OrderAdminListResult>;
  getOrder(orderId: string): Promise<OrderDetail>;
  getOrderEvents(orderId: string): Promise<OrderEvent[]>;
  getOrderShipments(orderId: string): Promise<ShipmentSummary[]>;
  cancelOrder(orderId: string, body?: CancelOrderRequest, idempotencyKey?: string): Promise<void>;
  closeOrder(orderId: string, body?: CloseOrderRequest, idempotencyKey?: string): Promise<void>;
  /**
   * Reconciles the provider payment for an order by the payment request
   * number and runs the shared order settlement saga.
   */
  confirmOrderPayment(orderId: string, requestNo: string, idempotencyKey?: string): Promise<void>;
  /**
   * Creates a refund request (partial or full) for a paid order. The refund
   * request id is derived from the idempotency key, so the same key always
   * resolves to the same request — callers must keep one key per refund
   * intent to avoid duplicate refunds.
   */
  createRefundRequest(
    orderId: string,
    input: Omit<AdminCreateRefundRequest, "amount"> & { amount: string },
    idempotencyKey: string,
  ): Promise<void>;
}

export function createOrderAdminService(
  client: SdkworkOrderBackendClient,
): OrderAdminService {
  return {
    async listOrders(query) {
      const page = query.page ?? 1;
      const pageSize = query.pageSize ?? 20;
      const raw = await client.orders.admin.list({
        page: String(page),
        pageSize: String(pageSize),
        status: query.status,
        q: query.q,
        createdFrom: query.createdFrom,
        createdTo: query.createdTo,
      });
      const listPage = unwrapSdkworkOrderListPage<OrderSummary>(raw);
      const pagination = resolveSdkworkOffsetPagination(
        listPage.pageInfo,
        page,
        pageSize,
      );
      return {
        items: listPage.items,
        page: pagination.page,
        pageSize: pagination.pageSize,
        totalItems: pagination.total,
        totalPages: pagination.totalPages,
      };
    },
    async getOrder(orderId) {
      const raw = await client.orders.admin.retrieve(orderId);
      return unwrapSdkworkOrderResource<OrderDetail>(raw);
    },
    async getOrderEvents(orderId) {
      const raw = await client.orders.admin.events.list(orderId, { page: "1", pageSize: "100" });
      return unwrapSdkworkOrderListPage<OrderEvent>(raw).items;
    },
    async getOrderShipments(orderId) {
      const raw = await client.shipments.list({
        page: "1",
        pageSize: "20",
        orderId,
      });
      return unwrapSdkworkOrderListPage<ShipmentSummary>(raw).items;
    },
    async cancelOrder(orderId, body, idempotencyKey) {
      const requestBody = body ?? { reason: "platform-cancel" };
      await client.orders.admin.cancel(
        orderId,
        createSdkworkIdempotencyParams(idempotencyKey),
        requestBody,
      );
    },
    async closeOrder(orderId, body, idempotencyKey) {
      const requestBody = body ?? { reason: "platform-close" };
      await client.orders.admin.close(
        orderId,
        createSdkworkIdempotencyParams(idempotencyKey),
        requestBody,
      );
    },
    async confirmOrderPayment(orderId, requestNo, idempotencyKey) {
      await client.orders.paymentConfirmations.create(
        orderId,
        { requestNo: requestNo.trim() },
        createSdkworkIdempotencyParams(idempotencyKey),
      );
    },
    async createRefundRequest(orderId, input, idempotencyKey) {
      await client.orders.admin.refundRequests.create(
        orderId,
        {
          amount: input.amount.trim(),
          ...(input.currencyCode ? { currencyCode: input.currencyCode } : {}),
          ...(input.reasonCode ? { reasonCode: input.reasonCode } : {}),
          ...(input.reasonMessage ? { reasonMessage: input.reasonMessage } : {}),
        },
        createSdkworkIdempotencyParams(idempotencyKey),
      );
    },
  };
}
