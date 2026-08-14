import type {
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
  cancelOrder(orderId: string, body?: CancelOrderRequest): Promise<void>;
  closeOrder(orderId: string, body?: CloseOrderRequest): Promise<void>;
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
    async cancelOrder(orderId, body) {
      const requestBody = body ?? { reason: "platform-cancel" };
      await client.orders.admin.cancel(
        orderId,
        createSdkworkIdempotencyParams(),
        requestBody,
      );
    },
    async closeOrder(orderId, body) {
      const requestBody = body ?? { reason: "platform-close" };
      await client.orders.admin.close(
        orderId,
        createSdkworkIdempotencyParams(),
        requestBody,
      );
    },
  };
}
