/**
 * Payment environment detection for the H5 cashier.
 *
 * The cashier behaves differently inside the Alipay app, the WeChat app and
 * a plain mobile browser: only Alipay payment is offered inside Alipay
 * (WAP redirect), only WeChat payment inside WeChat (JSAPI via OAuth), and
 * the full method list in a browser.
 *
 * Deployment region (`cn` / `overseas`) narrows the offered methods per
 * environment. The region is normally injected by the host deployment
 * (`configureOrderMobileRuntime`); when absent, the payer language is used
 * as a best-effort fallback so the cashier never shows CN-only channels to
 * overseas payers and vice versa.
 */

export type PaymentEnvironment = "alipay" | "wechat" | "browser";

/** Deployment region: mainland China cashier vs overseas cashier. */
export type PaymentRegion = "cn" | "overseas";

const ALIPAY_UA_PATTERN = /AlipayClient/i;
const WECHAT_UA_PATTERN = /MicroMessenger/i;

/** Matches phone/tablet user agents (mobile H5 cashier, no QR scanning). */
const MOBILE_UA_PATTERN =
  /Android|iPhone|iPad|iPod|Mobile|Windows Phone|webOS|BlackBerry|IEMobile|Opera Mini/i;

let paymentRegionOverride: PaymentRegion | null = null;

/**
 * Overrides the detected region. Hosts call this from the runtime
 * composition (`configureOrderMobileRuntime`) with their deployment value.
 * Pass `null` to restore locale-based detection.
 */
export function setPaymentRegionOverride(region: PaymentRegion | null): void {
  paymentRegionOverride = region;
}

function resolveNavigatorLanguage(): string | undefined {
  if (typeof navigator === "undefined") {
    return undefined;
  }
  const candidates = [
    navigator.language,
    ...(Array.isArray(navigator.languages) ? navigator.languages : []),
  ];
  for (const language of candidates) {
    if (typeof language === "string" && language.trim()) {
      return language.trim();
    }
  }
  return undefined;
}

/**
 * Detects the payment region for the cashier. The host-injected override
 * wins; otherwise the payer language (`zh-CN` / `zh*`) selects the CN
 * cashier and anything else defaults to the overseas cashier.
 */
export function detectPaymentRegion(language?: string): PaymentRegion {
  if (paymentRegionOverride) {
    return paymentRegionOverride;
  }
  const resolved = language?.trim() || resolveNavigatorLanguage() || "";
  return /^zh/i.test(resolved) ? "cn" : "overseas";
}

/** True when the cashier runs inside the Alipay app webview. */
export function isAlipayEnvironment(environment: PaymentEnvironment): boolean {
  return environment === "alipay";
}

/** True when the cashier runs inside the WeChat app webview. */
export function isWechatEnvironment(environment: PaymentEnvironment): boolean {
  return environment === "wechat";
}

/**
 * Detects the payment environment from a user agent string. The browser
 * global is only read when `userAgent` is omitted, so tests can inject a
 * fixed UA.
 */
export function detectPaymentEnvironment(userAgent?: string): PaymentEnvironment {
  const ua = userAgent ?? (typeof navigator !== "undefined" ? navigator.userAgent : "");
  if (ALIPAY_UA_PATTERN.test(ua)) {
    return "alipay";
  }
  if (WECHAT_UA_PATTERN.test(ua)) {
    return "wechat";
  }
  return "browser";
}

/**
 * True when the payer is on a phone/tablet. Mobile browsers must not be
 * shown QR codes; they jump to the provider H5 cashier instead (WeChat H5
 * pay / Alipay WAP), while desktop browsers keep the native QR scan flow.
 */
export function isMobileDevice(userAgent?: string): boolean {
  const ua = userAgent ?? (typeof navigator !== "undefined" ? navigator.userAgent : "");
  return MOBILE_UA_PATTERN.test(ua);
}
