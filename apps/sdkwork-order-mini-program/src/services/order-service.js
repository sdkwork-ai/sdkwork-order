/**
 * Order domain service.
 *
 * Wire contract: apis/app-api/order/order-app-api.openapi.json (orders +
 * coupon redemptions + refund requests). Request/response field names mirror
 * the backend camelCase read models (sdkwork-routes-order-app-api
 * OrderSummaryResponse / OrderDetailResponse / OrderEventResponse /
 * OrderPaymentParamsResponse / AccountValueRequestResponse). Amounts are
 * minor-unit integer strings; ids stay strings; POST commands send a fresh
 * `Idempotency-Key` header per attempt (required write-command header).
 */
const { request, unwrapResource, unwrapList, uuid } = require("./transport");

/** Cashier TTL mirrors the backend payment/order window (CashierLogic.ts). */
const CASHIER_TTL_MS = 15 * 60 * 1000;

/** Order tabs; ids map 1:1 onto the backend `status` query values. */
const ORDER_TABS = [
  { code: "all", label: "全部" },
  { code: "pending_payment", label: "待付款" },
  { code: "paid", label: "已支付" },
  { code: "fulfilled", label: "待收货" },
  { code: "completed", label: "已完成" },
  { code: "cancelled", label: "已取消" },
];

function normalizeStatus(value) {
  const lowered = String(value === undefined || value === null ? "" : value).trim().toLowerCase();
  if (!lowered || lowered === "all") {
    return undefined;
  }
  return lowered;
}

function mapOrderSummary(value) {
  const record = value && typeof value === "object" ? value : {};
  return {
    orderId: String(record.orderId ?? record.id ?? ""),
    orderSn: String(record.orderSn ?? ""),
    status: String(record.status ?? ""),
    statusName: String(record.statusName ?? ""),
    subject: String(record.subject ?? "订单"),
    totalAmount: String(record.totalAmount ?? "0"),
    paidAmount: record.paidAmount !== undefined && record.paidAmount !== null ? String(record.paidAmount) : undefined,
    discountAmount:
      record.discountAmount !== undefined && record.discountAmount !== null
        ? String(record.discountAmount)
        : undefined,
    quantity: Number(record.quantity) || 0,
    createdAt: String(record.createdAt ?? ""),
    payTime: record.payTime != null ? String(record.payTime) : undefined,
    expireTime: record.expireTime != null ? String(record.expireTime) : undefined,
    paymentMethod: record.paymentMethod != null ? String(record.paymentMethod) : undefined,
    remark: record.remark != null ? String(record.remark) : undefined,
    points: record.points != null ? Number(record.points) : undefined,
  };
}

function mapOrderDetail(value) {
  const summary = mapOrderSummary(value);
  const record = value && typeof value === "object" ? value : {};
  const items = Array.isArray(record.items) ? record.items : [];
  return {
    ...summary,
    items: items.map((item, index) => {
      const line = item && typeof item === "object" ? item : {};
      return {
        id: String(line.id ?? `item-${index + 1}`),
        productName: String(line.productName ?? line.title ?? ""),
        quantity: Number(line.quantity) || 1,
        unitPrice: String(line.unitPrice ?? "0"),
        totalAmount: String(line.totalAmount ?? "0"),
      };
    }),
    outTradeNo: record.outTradeNo != null ? String(record.outTradeNo) : undefined,
    transactionId: record.transactionId != null ? String(record.transactionId) : undefined,
  };
}

function mapOrderEvent(value) {
  const record = value && typeof value === "object" ? value : {};
  return {
    eventId: String(record.eventId ?? ""),
    orderId: String(record.orderId ?? ""),
    eventType: String(record.eventType ?? ""),
    fromStatus: record.fromStatus != null ? String(record.fromStatus) : undefined,
    toStatus: String(record.toStatus ?? ""),
    actorType: record.actorType != null ? String(record.actorType) : undefined,
    actorId: record.actorId != null ? String(record.actorId) : undefined,
    message: record.message != null ? String(record.message) : undefined,
    createdAt: String(record.createdAt ?? ""),
  };
}

/** Order status list endpoint (page_size/page). */
async function listOrders(options = {}) {
  const payload = await request({
    path: "/orders",
    query: {
      status: normalizeStatus(options.status),
      page: options.page ?? 1,
      page_size: options.pageSize ?? 20,
    },
  });
  const { items, pageInfo } = unwrapList(payload);
  return { items: items.map(mapOrderSummary), pageInfo };
}

async function getStatistics() {
  const item = unwrapResource(await request({ path: "/orders/statistics" }));
  const record = item && typeof item === "object" ? item : {};
  return {
    totalOrders: Number(record.totalOrders) || 0,
    pendingPayment: Number(record.pendingPayment) || 0,
    pendingShipment: Number(record.pendingShipment) || 0,
    pendingReceipt: Number(record.pendingReceipt) || 0,
    completed: Number(record.completed) || 0,
    totalAmount: String(record.totalAmount ?? "0"),
  };
}

async function getOrderDetail(orderId) {
  const item = unwrapResource(await request({ path: `/orders/${orderId}` }));
  return mapOrderDetail(item);
}

async function getOrderEvents(orderId, options = {}) {
  const payload = await request({
    path: `/orders/${orderId}/events`,
    query: { page: options.page ?? 1, page_size: options.pageSize ?? 20 },
  });
  const { items, pageInfo } = unwrapList(payload);
  return { items: items.map(mapOrderEvent), pageInfo };
}

async function getOrderStatus(orderId) {
  const item = unwrapResource(await request({ path: `/orders/${orderId}/status` }));
  const record = item && typeof item === "object" ? item : {};
  return {
    status: String(record.status ?? ""),
    statusName: String(record.statusName ?? ""),
  };
}

/** Payment polling contract: GET /orders/{orderId}/payment_success. */
async function getPaymentSuccess(orderId) {
  const item = unwrapResource(await request({ path: `/orders/${orderId}/payment_success` }));
  const record = item && typeof item === "object" ? item : {};
  return {
    paid: record.paid === true,
    status: String(record.status ?? ""),
    statusName: String(record.statusName ?? ""),
  };
}

/**
 * Creates a payment session (POST /orders/{orderId}/payments, 201).
 * `paymentMethod` must be one of the backend whitelist:
 * wechat_pay | wechat_jsapi | alipay | alipay_wap | balance.
 * The fresh `Idempotency-Key` dedupes transport-level retries of one attempt.
 */
async function createPayment(orderId, paymentMethod) {
  const item = unwrapResource(
    await request({
      path: `/orders/${orderId}/payments`,
      method: "POST",
      body: { paymentMethod },
      headers: { "Idempotency-Key": uuid() },
    }),
  );
  const record = item && typeof item === "object" ? item : {};
  return {
    amount: String(record.amount ?? "0"),
    orderId: String(record.orderId ?? orderId),
    outTradeNo: String(record.outTradeNo ?? ""),
    paymentId: String(record.paymentId ?? ""),
    paymentMethod: String(record.paymentMethod ?? paymentMethod),
    paymentParams:
      record.paymentParams && typeof record.paymentParams === "object" ? record.paymentParams : {},
  };
}

/** POST /orders/{orderId}/cancellations (201, SdkWorkCommandData). */
async function cancelOrder(orderId, cancelReason) {
  const body = cancelReason ? { cancelReason } : {};
  return request({
    path: `/orders/${orderId}/cancellations`,
    method: "POST",
    body,
    headers: { "Idempotency-Key": uuid() },
  });
}

/** POST /orders/{orderId}/receipt_confirmations (201). */
async function confirmReceipt(orderId) {
  return request({
    path: `/orders/${orderId}/receipt_confirmations`,
    method: "POST",
    body: {},
    headers: { "Idempotency-Key": uuid() },
  });
}

/** POST /orders/coupon_redemptions → CouponRedemptionResult. */
async function redeemCoupon(couponCode) {
  const item = unwrapResource(
    await request({
      path: "/orders/coupon_redemptions",
      method: "POST",
      body: { couponCode: String(couponCode || "").trim() },
      headers: { "Idempotency-Key": uuid() },
    }),
  );
  const record = item && typeof item === "object" ? item : {};
  const benefit = record.benefit && typeof record.benefit === "object" ? record.benefit : {};
  return {
    orderId: String(record.orderId ?? ""),
    orderNo: String(record.orderNo ?? ""),
    status: String(record.status ?? ""),
    replayed: record.replayed === true,
    benefit,
  };
}

/** GET /orders/refund_requests (server-paginated). */
async function listRefundRequests(options = {}) {
  const payload = await request({
    path: "/orders/refund_requests",
    query: { page: options.page ?? 1, page_size: options.pageSize ?? 20 },
  });
  const { items, pageInfo } = unwrapList(payload);
  return { items: items.map(mapAccountValueRequest), pageInfo };
}

/** POST /orders/refund_requests (201) → AccountValueRequestResponse. */
async function createRefundRequest(input) {
  const item = unwrapResource(
    await request({
      path: "/orders/refund_requests",
      method: "POST",
      body: {
        originalOrderId: input.originalOrderId,
        targetAsset: input.targetAsset,
        amount: input.amount,
        currencyCode: input.currencyCode || "CNY",
        reasonCode: input.reasonCode,
        reasonDetail: input.reasonDetail,
      },
      headers: { "Idempotency-Key": uuid() },
    }),
  );
  return mapAccountValueRequest(item);
}

function mapAccountValueRequest(value) {
  const record = value && typeof value === "object" ? value : {};
  return {
    accountValueRequestId: String(record.accountValueRequestId ?? record.id ?? ""),
    requestNo: String(record.requestNo ?? ""),
    originalOrderId:
      record.originalOrderId != null ? String(record.originalOrderId) : undefined,
    ownerUserId: String(record.ownerUserId ?? ""),
    subject: String(record.subject ?? ""),
    targetAsset: String(record.targetAsset ?? ""),
    amount: String(record.amount ?? "0"),
    currencyCode: String(record.currencyCode ?? "CNY"),
    status: String(record.status ?? ""),
    providerReferenceId:
      record.providerReferenceId != null ? String(record.providerReferenceId) : undefined,
    createdAt: String(record.createdAt ?? ""),
    updatedAt: String(record.updatedAt ?? ""),
  };
}

/**
 * Polls GET /orders/{orderId}/payment_success with exponential backoff until
 * `paid` or the deadline. On deadline expiry it takes one final order-status
 * reading (GET /orders/{orderId}/status) so a webhook that only moved the
 * order still resolves the terminal phase; resolves `{ timedOut: true }`
 * otherwise. Never rejects for pollable conditions (transport errors are
 * retried until the deadline); only rejects when the final status read fails.
 */
function pollPaymentSuccess(orderId, options = {}) {
  const intervalMs = options.intervalMs ?? 3000;
  const maxIntervalMs = options.maxIntervalMs ?? 8000;
  const deadlineMs = options.deadlineMs ?? Date.now() + CASHIER_TTL_MS;
  const onCountdown = typeof options.onCountdown === "function" ? options.onCountdown : null;

  return new Promise((resolve, reject) => {
    let delay = intervalMs;

    const finish = (result) => {
      if (onCountdown) {
        onCountdown(0);
      }
      resolve(result);
    };

    const finalCheck = async () => {
      try {
        const orderStatus = await getOrderStatus(orderId);
        const normalized = orderStatus.status.toLowerCase();
        if (["paid", "fulfilled", "completed"].includes(normalized)) {
          finish({ paid: true, status: orderStatus.status, statusName: orderStatus.statusName, timedOut: false });
          return;
        }
        const paymentStatus = await getPaymentSuccess(orderId);
        finish({ ...paymentStatus, timedOut: true });
      } catch (error) {
        reject(error);
      }
    };

    const tick = async () => {
      const remainingSeconds = Math.max(0, Math.floor((deadlineMs - Date.now()) / 1000));
      if (onCountdown) {
        onCountdown(remainingSeconds);
      }
      if (Date.now() >= deadlineMs) {
        await finalCheck();
        return;
      }
      let status;
      try {
        status = await getPaymentSuccess(orderId);
      } catch (error) {
        if (error && error.statusCode === 401) {
          reject(error);
          return;
        }
        // transient network/server error: keep backing off until the deadline
        delay = Math.min(Math.round(delay * 1.5), maxIntervalMs);
        setTimeout(tick, delay);
        return;
      }
      if (status.paid) {
        finish({ ...status, timedOut: false });
        return;
      }
      delay = Math.min(Math.round(delay * 1.5), maxIntervalMs);
      setTimeout(tick, delay);
    };

    tick();
  });
}

module.exports = {
  ORDER_TABS,
  CASHIER_TTL_MS,
  listOrders,
  getStatistics,
  getOrderDetail,
  getOrderEvents,
  getOrderStatus,
  getPaymentSuccess,
  createPayment,
  cancelOrder,
  confirmReceipt,
  redeemCoupon,
  listRefundRequests,
  createRefundRequest,
  pollPaymentSuccess,
};
