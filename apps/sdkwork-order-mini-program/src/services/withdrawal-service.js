/**
 * Withdrawal request domain service.
 *
 * Wire contract: apis/app-api/order/order-app-api.openapi.json.
 * - POST /withdrawals/requests            → create (201, AccountValueRequestResponse)
 * - GET  /withdrawals/requests/{requestId} → retrieve one request
 *
 * The order app-api exposes no owner-scoped withdrawal list endpoint yet, so
 * the records view keeps a bounded device-local submission history (latest 20
 * request ids) in platform storage and refreshes each row through the server
 * retrieve. Swap `listLocalHistory` for the server list once that endpoint
 * ships (docs/decisions.md).
 */
const { request, unwrapResource, uuid } = require("./transport");

const HISTORY_STORAGE_KEY = "sdkwork-order-mp-withdrawal-history";
const HISTORY_LIMIT = 20;

function mapRequest(value) {
  const record = value && typeof value === "object" ? value : {};
  return {
    accountValueRequestId: String(record.accountValueRequestId ?? record.id ?? ""),
    requestNo: String(record.requestNo ?? ""),
    originalOrderId:
      record.originalOrderId != null ? String(record.originalOrderId) : undefined,
    ownerUserId: String(record.ownerUserId ?? ""),
    subject: String(record.subject ?? ""),
    targetAsset: String(record.targetAsset ?? "cash"),
    amount: String(record.amount ?? "0"),
    currencyCode: String(record.currencyCode ?? "CNY"),
    status: String(record.status ?? ""),
    providerReferenceId:
      record.providerReferenceId != null ? String(record.providerReferenceId) : undefined,
    createdAt: String(record.createdAt ?? ""),
    updatedAt: String(record.updatedAt ?? ""),
  };
}

/** POST /withdrawals/requests (201). amount is a minor-unit integer string. */
async function createWithdrawalRequest(input) {
  const item = unwrapResource(
    await request({
      path: "/withdrawals/requests",
      method: "POST",
      body: {
        asset: "cash",
        amount: input.amount,
        currencyCode: input.currencyCode || "CNY",
        payoutMethod: input.payoutMethod,
        payoutAccountRef: input.payoutAccountRef,
        reasonCode: input.reasonCode,
      },
      headers: { "Idempotency-Key": uuid() },
    }),
  );
  return mapRequest(item);
}

async function getWithdrawalRequest(withdrawalRequestId) {
  const item = unwrapResource(
    await request({ path: `/withdrawals/requests/${withdrawalRequestId}` }),
  );
  return mapRequest(item);
}

function readHistory() {
  try {
    const raw = wx.getStorageSync(HISTORY_STORAGE_KEY);
    if (!Array.isArray(raw)) {
      return [];
    }
    return raw
      .filter((entry) => entry && typeof entry === "object" && entry.accountValueRequestId)
      .slice(0, HISTORY_LIMIT);
  } catch (error) {
    return [];
  }
}

function remember(record) {
  const history = readHistory().filter(
    (entry) => entry.accountValueRequestId !== record.accountValueRequestId,
  );
  history.unshift({
    accountValueRequestId: record.accountValueRequestId,
    requestNo: record.requestNo,
    amount: record.amount,
    currencyCode: record.currencyCode,
    status: record.status,
    createdAt: record.createdAt,
  });
  try {
    wx.setStorageSync(HISTORY_STORAGE_KEY, history.slice(0, HISTORY_LIMIT));
  } catch (error) {
    // storage unavailable: history stays session-only
  }
}

/** Device-local bounded history (latest first), capped at 20 entries. */
async function listLocalHistory() {
  const history = readHistory();
  const refreshed = await Promise.all(
    history.map(async (entry) => {
      try {
        return await getWithdrawalRequest(entry.accountValueRequestId);
      } catch (error) {
        // keep the local snapshot when the server row is unreachable
        return mapRequest(entry);
      }
    }),
  );
  return refreshed;
}

module.exports = { createWithdrawalRequest, getWithdrawalRequest, listLocalHistory, HISTORY_LIMIT };
