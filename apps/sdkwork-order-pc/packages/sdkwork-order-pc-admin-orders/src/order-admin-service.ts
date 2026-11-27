import type {
  AdminCreateRefundRequest,
  CancelOrderRequest,
  CloseOrderRequest,
  OrderClassification,
  OrderDetail,
  OrderEvent,
  OrderFacet,
  OrderSummary,
  ShipmentSummary,
  SdkworkOrderBackendClient,
} from "@sdkwork/order-pc-admin-core";
import {
  createSdkworkIdempotencyParams,
  resolveSdkworkOffsetPagination,
  unwrapSdkworkOrderListPage,
  unwrapSdkworkOrderResource,
  type SdkworkOrderScene,
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
  /** Commerce scene facet; omitted means "all scenes". */
  scene?: SdkworkOrderScene;
  /** Tag facet values (canonical); empty means "all tags". */
  tags?: readonly string[];
  /** How `tags` combine. Defaults to `any` on the server. */
  tagMatch?: "any" | "all";
}

/** Facet aggregate scope: honours the narrowing filters but not tag/scene. */
export interface OrderAdminFacetQuery {
  status?: string;
  q?: string;
  createdFrom?: string;
  createdTo?: string;
}

/** Body accepted by the classification write endpoints. */
export interface OrderClassificationInput {
  tags: readonly string[];
  /** Replacement scene (replace endpoint only). */
  scene?: SdkworkOrderScene;
  remark?: string;
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
  /**
   * Scene and tag bucket counts for the filter bar.
   *
   * Counts ignore the tag/scene predicates so each bucket shows the size the
   * operator would get by *adding* that value to the current selection.
   */
  getOrderFacets(query?: OrderAdminFacetQuery): Promise<OrderFacet>;
  /** Replaces the whole tag set (and optionally the scene). Idempotent. */
  replaceOrderClassification(
    orderId: string,
    input: OrderClassificationInput,
    idempotencyKey: string,
  ): Promise<OrderClassification>;
  /** Adds tags to the order's existing set. */
  addOrderTags(
    orderId: string,
    input: OrderClassificationInput,
    idempotencyKey: string,
  ): Promise<OrderClassification>;
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
        ...(query.scene ? { scene: query.scene } : {}),
        ...(query.tags && query.tags.length > 0
          ? { tags: [...query.tags], tagMatch: query.tagMatch ?? "any" }
          : {}),
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
    async getOrderFacets(query) {
      const raw = await client.orders.admin.facets.list({
        status: query?.status,
        q: query?.q,
        createdFrom: query?.createdFrom,
        createdTo: query?.createdTo,
      });
      return unwrapSdkworkOrderResource<OrderFacet>(raw);
    },
    async replaceOrderClassification(orderId, input, idempotencyKey) {
      const raw = await client.orders.admin.classification.update(
        orderId,
        toClassificationBody(input),
        createSdkworkIdempotencyParams(idempotencyKey),
      );
      return unwrapSdkworkOrderResource<OrderClassification>(raw);
    },
    async addOrderTags(orderId, input, idempotencyKey) {
      const raw = await client.orders.admin.tags.create(
        orderId,
        toClassificationBody(input, { withScene: false }),
        createSdkworkIdempotencyParams(idempotencyKey),
      );
      return unwrapSdkworkOrderResource<OrderClassification>(raw);
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

/**
 * Maps the UI classification state onto the wire body.
 *
 * Tags are canonicalized once here so every caller (token editor, quick-add
 * chip, bulk action) submits the same normalized values the server would
 * otherwise have to fix up. `scene` is only sent by the replace endpoint: the
 * add/remove endpoints would ignore it, and sending it there would suggest a
 * scene change that never happens.
 */
function toClassificationBody(
  input: OrderClassificationInput,
  options: { withScene?: boolean } = {},
): { tags: string[]; scene?: SdkworkOrderScene; remark?: string } {
  const remark = input.remark?.trim();
  return {
    tags: [...input.tags],
    ...(options.withScene !== false && input.scene ? { scene: input.scene } : {}),
    ...(remark ? { remark } : {}),
  };
}
