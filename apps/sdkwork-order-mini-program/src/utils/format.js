/**
 * Formatting helpers.
 *
 * Money amounts arrive as minor-unit integer strings ("6990" = 69.90 CNY,
 * API_SPEC.md section 13.6 / minor-unit rule). Conversion uses string math so
 * values beyond Number.MAX_SAFE_INTEGER never round. Ids and snowflake values
 * stay strings end to end.
 */

/** Formats a minor-unit amount string into a display string ("¥69.90"). */
function formatMinor(minorAmount, options) {
  const opts = options || {};
  const symbol = opts.symbol === undefined ? "¥" : opts.symbol;
  if (minorAmount === undefined || minorAmount === null || minorAmount === "") {
    return "--";
  }
  const raw = String(minorAmount).trim();
  const match = /^([+-]?)([0-9]+)$/.exec(raw);
  if (!match) {
    return `${symbol}${raw}`;
  }
  const sign = match[1] === "-" ? "-" : "";
  const digits = match[2];
  const padded = digits.padStart(3, "0");
  const yuan = padded.slice(0, padded.length - 2).replace(/^0+(?=[0-9])/, "");
  const fen = padded.slice(padded.length - 2);
  return `${sign}${symbol}${yuan}.${fen}`;
}

/**
 * Converts a yuan text input ("12.5", "12", "0.01") into a minor-unit integer
 * string ("1250", "1200", "1"). Returns null when the input is not a valid
 * positive amount with at most two decimals.
 */
function yuanInputToMinor(text) {
  const raw = String(text === undefined || text === null ? "" : text).trim();
  if (!/^[0-9]+(\.[0-9]{1,2})?$/.test(raw)) {
    return null;
  }
  const [whole, fraction = ""] = raw.split(".");
  const paddedFraction = `${fraction}00`.slice(0, 2);
  const digits = `${whole}${paddedFraction}`.replace(/^0+(?=[0-9])/, "");
  return digits || "0";
}

/** True when the value is a non-empty decimal-digit string (ids, amounts). */
function isDigitString(value) {
  return typeof value === "string" && /^[0-9]+$/.test(value);
}

function formatTime(value) {
  if (!value) {
    return "--";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return String(value);
  }
  const pad = (part) => String(part).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function formatCountdown(remainingSeconds) {
  const total = Math.max(0, Math.floor(remainingSeconds));
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

const ORDER_STATUS_LABELS = {
  pending_payment: "待付款",
  paid: "已支付",
  fulfilled: "待收货",
  completed: "已完成",
  cancelled: "已取消",
  expired: "已超时",
  refunding: "退款中",
  refunded: "已退款",
};

function orderStatusLabel(status) {
  return ORDER_STATUS_LABELS[String(status || "").toLowerCase()] || "处理中";
}

const REQUEST_STATUS_LABELS = {
  requested: "已受理",
  pending: "处理中",
  processing: "处理中",
  approved: "已通过",
  completed: "已完成",
  succeeded: "已完成",
  rejected: "已拒绝",
  cancelled: "已取消",
  failed: "已失败",
};

function requestStatusLabel(status) {
  return REQUEST_STATUS_LABELS[String(status || "").toLowerCase()] || "处理中";
}

const ASSET_LABELS = {
  points: "积分",
  token_bank: "Token Bank",
  cash: "现金",
};

function assetLabel(asset) {
  return ASSET_LABELS[String(asset || "").toLowerCase()] || String(asset || "--");
}

module.exports = {
  formatMinor,
  yuanInputToMinor,
  isDigitString,
  formatTime,
  formatCountdown,
  orderStatusLabel,
  requestStatusLabel,
  assetLabel,
};
