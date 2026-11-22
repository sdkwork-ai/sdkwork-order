import assert from "node:assert/strict";
import { test } from "node:test";

import {
  buildWechatRequestPaymentOptions,
  parseJsapiPayload,
  paymentCancelledError,
} from "../src/utils/provider-payment.js";

const JSAPI = {
  appId: "wx1234567890",
  timeStamp: "1700000000",
  nonceStr: "nonce-value",
  package: "prepay_id=wx1700000000",
  signType: "RSA",
  paySign: "signed-value",
};

test("a complete jsapi payload becomes the wx.requestPayment option set", () => {
  const options = buildWechatRequestPaymentOptions({
    jsapiPayload: JSON.stringify(JSAPI),
    providerCode: "wechat_pay",
  });
  assert.deepEqual(options, {
    timeStamp: "1700000000",
    nonceStr: "nonce-value",
    package: "prepay_id=wx1700000000",
    signType: "RSA",
    paySign: "signed-value",
  });
});

test("a missing signType defaults to RSA instead of a guessed scheme", () => {
  const payload = { ...JSAPI };
  delete payload.signType;
  const options = buildWechatRequestPaymentOptions({ jsapiPayload: JSON.stringify(payload) });
  assert.equal(options.signType, "RSA");
});

test("numeric timestamps are normalised to the string the client requires", () => {
  const options = buildWechatRequestPaymentOptions({
    jsapiPayload: JSON.stringify({ ...JSAPI, timeStamp: 1700000000 }),
  });
  assert.equal(options.timeStamp, "1700000000");
});

test("sessions without a launchable payload never produce options", () => {
  // Native QR, redirect products and empty params have nothing to launch in-app.
  for (const paymentParams of [
    undefined,
    null,
    {},
    { qrCodePayload: "weixin://wxpay/bizpayurl?pr=abc" },
    { jsapiPayload: "" },
    { jsapiPayload: "not-json" },
    { jsapiPayload: "null" },
    { jsapiPayload: JSON.stringify({ ...JSAPI, paySign: "" }) },
    { jsapiPayload: JSON.stringify({ ...JSAPI, package: "   " }) },
  ]) {
    assert.equal(
      buildWechatRequestPaymentOptions(paymentParams),
      null,
      `expected no options for ${JSON.stringify(paymentParams)}`,
    );
  }
});

test("a partially decoded payload is rejected rather than half-built", () => {
  const partial = { appId: JSAPI.appId, timeStamp: JSAPI.timeStamp, nonceStr: JSAPI.nonceStr };
  assert.equal(
    buildWechatRequestPaymentOptions({ jsapiPayload: JSON.stringify(partial) }),
    null,
  );
  assert.deepEqual(parseJsapiPayload({ jsapiPayload: JSON.stringify(partial) }), partial);
});

test("payer cancellation is distinguishable from a real failure", () => {
  const cancelled = paymentCancelledError();
  assert.equal(cancelled.code, "PAYMENT_CANCELLED");
  assert.ok(cancelled instanceof Error);
});
