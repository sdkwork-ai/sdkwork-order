/**
 * Provider launch seam for the WeChat mini-program cashier.
 *
 * The backend payment session returns `paymentParams`, a flat string map whose
 * launch payload depends on the provider product:
 *
 *   - `wechat_jsapi`  -> `jsapiPayload`  (JSON: appId/timeStamp/nonceStr/
 *                        package/signType/paySign, signed with the merchant key)
 *   - `wechat_native` -> `qrCodePayload` (a QR string; there is nothing to scan
 *                        inside the WeChat app, so it must not be offered here)
 *
 * Only JSAPI can be completed in-app. Keeping the mapping pure lets the cashier
 * page stay thin and lets the contract be unit-tested without the WeChat
 * runtime.
 */

/** The fields `wx.requestPayment` requires, in contract order. */
const REQUEST_PAYMENT_FIELDS = ["timeStamp", "nonceStr", "package", "paySign"];

/**
 * Parses the JSAPI launch payload out of the payment-session params.
 *
 * @param {object|undefined|null} paymentParams flat param map from the session
 * @returns {object|null} decoded payload, or null when it is absent/invalid
 */
function parseJsapiPayload(paymentParams) {
  if (!paymentParams || typeof paymentParams !== "object") {
    return null;
  }
  const raw = paymentParams.jsapiPayload;
  if (typeof raw !== "string" || raw.trim() === "") {
    return null;
  }
  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" ? parsed : null;
  } catch (error) {
    return null;
  }
}

/**
 * Builds the exact options object `wx.requestPayment` expects.
 *
 * Returns null when the session carries no complete JSAPI payload: the caller
 * must then not pretend to start a payment (a half-built call would open the
 * WeChat payment sheet with an invalid signature).
 *
 * @param {object|undefined|null} paymentParams flat param map from the session
 * @returns {{timeStamp: string, nonceStr: string, package: string,
 *            signType: string, paySign: string}|null}
 */
function buildWechatRequestPaymentOptions(paymentParams) {
  const payload = parseJsapiPayload(paymentParams);
  if (!payload) {
    return null;
  }
  const options = {};
  for (const field of REQUEST_PAYMENT_FIELDS) {
    const value = payload[field];
    if (value === undefined || value === null) {
      return null;
    }
    const text = String(value).trim();
    if (text === "") {
      return null;
    }
    options[field] = text;
  }
  // WeChat Pay API v3 signs JSAPI invocations with RSA. MD5/HMAC belong to the
  // retired v2 signing scheme, so an absent signType defaults to RSA instead of
  // letting the WeChat client guess.
  const signType = typeof payload.signType === "string" ? payload.signType.trim() : "";
  options.signType = signType === "" ? "RSA" : signType;
  return options;
}

/**
 * Distinguishable "the payer closed the payment sheet" failure.
 *
 * Closing the sheet is a normal choice, not an error: the cashier returns to
 * the ready state without an error banner, while any other failure is surfaced.
 *
 * @returns {Error} error carrying `code: "PAYMENT_CANCELLED"`
 */
function paymentCancelledError() {
  const error = new Error("payment cancelled by the payer");
  error.code = "PAYMENT_CANCELLED";
  return error;
}

module.exports = {
  parseJsapiPayload,
  buildWechatRequestPaymentOptions,
  paymentCancelledError,
};
