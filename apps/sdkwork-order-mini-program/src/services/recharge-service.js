/**
 * Recharge domain service.
 *
 * Wire contract: apis/app-api/order/order-app-api.openapi.json (recharges.*).
 * - GET  /recharges/plans            → Token Bank plans (TokenBankPlanResponse)
 * - GET  /recharges/packages         → point recharge packages (RechargePackageResponse)
 * - GET+POST /recharges/orders       → owner recharge orders (list / SubmitRechargeResponse)
 * - GET  /recharges/orders/{orderId} → recharge order read model
 * - POST /recharges/orders/{orderId}/cancel → cancel (write-command headers)
 *
 * Plan/package catalog prices are major-unit decimals ("10.00"); the recharge
 * create command takes minor-unit integer strings ("1000") — the same
 * ×100 conversion applied by the H5 SubscriptionPurchaseService. Every POST
 * sends a fresh `Idempotency-Key` header per attempt.
 */
const { request, unwrapResource, unwrapList, uuid } = require("./transport");

function mapPlan(value) {
  const record = value && typeof value === "object" ? value : {};
  return {
    planCode: String(record.planCode ?? ""),
    displayName: String(record.displayName ?? record.planCode ?? ""),
    planPeriod: String(record.planPeriod ?? ""),
    grantAmount: String(record.grantAmount ?? "0"),
    bonusAmount: String(record.bonusAmount ?? "0"),
    priceAmount: String(record.priceAmount ?? "0"),
    currencyCode: String(record.currencyCode ?? "CNY"),
    renewalPolicy: String(record.renewalPolicy ?? ""),
    status: String(record.status ?? ""),
  };
}

function mapPackage(value) {
  const record = value && typeof value === "object" ? value : {};
  return {
    id: String(record.id ?? ""),
    priceAmount: String(record.priceAmount ?? "0"),
    currencyCode: String(record.currencyCode ?? "CNY"),
    bonusPoints: Number(record.bonusPoints) || 0,
    grantAmount: Number(record.grantAmount) || 0,
    points: Number(record.points) || 0,
  };
}

function mapRechargeOrder(value) {
  const record = value && typeof value === "object" ? value : {};
  return {
    orderId: String(record.orderId ?? record.id ?? ""),
    orderNo: String(record.orderNo ?? ""),
    status: String(record.status ?? ""),
    subject: String(record.subject ?? ""),
    amount: String(record.amount ?? "0"),
    points: Number(record.points) || 0,
    createdAt: String(record.createdAt ?? ""),
  };
}

async function listPlans() {
  const { items } = unwrapList(await request({ path: "/recharges/plans" }));
  return items.map(mapPlan);
}

async function listPackages() {
  const { items } = unwrapList(await request({ path: "/recharges/packages" }));
  return items.map(mapPackage);
}

/**
 * Converts a catalog major-unit price ("10.00", "10") into the minor-unit
 * integer string the recharge API requires ("1000"). Malformed prices reject
 * before any network call so a decimal price can never undercharge.
 */
function planPriceToMinor(priceAmount) {
  const raw = String(priceAmount === undefined || priceAmount === null ? "" : priceAmount).trim();
  if (!/^[0-9]+(\.[0-9]{1,2})?$/.test(raw)) {
    throw new Error("套餐价格无效，无法下单");
  }
  const [whole, fraction = ""] = raw.split(".");
  return `${whole}${`${fraction}00`.slice(0, 2)}`.replace(/^0+(?=[0-9])/, "") || "0";
}

/**
 * Creates a recharge order (POST /recharges/orders, 201).
 * `input.amount` MUST already be a minor-unit integer string (use
 * planPriceToMinor for plan/package catalog prices).
 */
async function createRechargeOrder(input) {
  const item = unwrapResource(
    await request({
      path: "/recharges/orders",
      method: "POST",
      body: {
        subject: input.subject,
        targetAsset: input.targetAsset,
        amount: input.amount,
        grantAmount: input.grantAmount,
        currencyCode: input.currencyCode || "CNY",
        packageId: input.packageId,
        planCode: input.planCode,
        planPeriod: input.planPeriod,
        paymentMethod: input.paymentMethod,
        paymentProduct: "wechat_native",
        source: input.source ?? "mini-program",
      },
      headers: { "Idempotency-Key": uuid() },
    }),
  );
  const record = item && typeof item === "object" ? item : {};
  return {
    success: record.success !== false,
    orderId: String(record.orderId ?? record.id ?? ""),
    orderNo: String(record.orderNo ?? ""),
    outTradeNo: String(record.outTradeNo ?? ""),
    subject: String(record.subject ?? ""),
    targetAsset: String(record.targetAsset ?? ""),
    amount: String(record.amount ?? "0"),
    grantAmount: String(record.grantAmount ?? "0"),
    currencyCode: String(record.currencyCode ?? "CNY"),
    points: Number(record.points) || 0,
    providerCode: String(record.providerCode ?? ""),
    paymentMethod: String(record.paymentMethod ?? ""),
    paymentProduct: String(record.paymentProduct ?? ""),
    expiresAt: record.expiresAt != null ? String(record.expiresAt) : undefined,
    status: String(record.status ?? ""),
    nextAction: String(record.nextAction ?? ""),
    cashierUrl: String(record.cashierUrl ?? ""),
    qrCodePayload: String(record.qrCodePayload ?? ""),
    requestPaymentPayload:
      record.requestPaymentPayload != null ? String(record.requestPaymentPayload) : undefined,
  };
}

async function getRechargeOrder(orderId) {
  const item = unwrapResource(await request({ path: `/recharges/orders/${orderId}` }));
  return mapRechargeOrder(item);
}

async function listRechargeOrders(options = {}) {
  const payload = await request({
    path: "/recharges/orders",
    query: { page: options.page ?? 1, page_size: options.pageSize ?? 20 },
  });
  const { items, pageInfo } = unwrapList(payload);
  return { items: items.map(mapRechargeOrder), pageInfo };
}

/** POST /recharges/orders/{orderId}/cancel (200, write-command headers). */
async function cancelRechargeOrder(orderId, cancelReason) {
  return request({
    path: `/recharges/orders/${orderId}/cancel`,
    method: "POST",
    body: cancelReason ? { cancelReason } : {},
    headers: { "Idempotency-Key": uuid() },
  });
}

module.exports = {
  listPlans,
  listPackages,
  planPriceToMinor,
  createRechargeOrder,
  getRechargeOrder,
  listRechargeOrders,
  cancelRechargeOrder,
};
