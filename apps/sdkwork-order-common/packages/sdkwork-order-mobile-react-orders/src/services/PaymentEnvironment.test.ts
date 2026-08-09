import { describe, expect, it } from "vitest";

import {
  detectPaymentEnvironment,
  detectPaymentRegion,
  isMobileDevice,
  setPaymentRegionOverride,
} from "./PaymentEnvironment";

describe("detectPaymentEnvironment", () => {
  it("detects the Alipay app webview", () => {
    const ua =
      "Mozilla/5.0 (Linux; Android 13; PEEM00) AppleWebKit/537.36 " +
      "(KHTML, like Gecko) Version/4.0 Chrome/109.0.0.0 Safari/537.36 AlipayClient/10.5.88.9100";
    expect(detectPaymentEnvironment(ua)).toBe("alipay");
  });

  it("detects the WeChat app webview", () => {
    const ua =
      "Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 " +
      "(KHTML, like Gecko) Mobile/15E148 MicroMessenger/8.0.49";
    expect(detectPaymentEnvironment(ua)).toBe("wechat");
  });

  it("falls back to a plain browser for desktop and mobile browsers", () => {
    expect(detectPaymentEnvironment(undefined)).toBe("browser");
    expect(
      detectPaymentEnvironment(
        "Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 Mobile Safari/604.1",
      ),
    ).toBe("browser");
    expect(
      detectPaymentEnvironment("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0"),
    ).toBe("browser");
  });
});

describe("detectPaymentRegion", () => {
  it("honours the host-injected deployment region", () => {
    setPaymentRegionOverride("overseas");
    expect(detectPaymentRegion("zh-CN")).toBe("overseas");
    setPaymentRegionOverride("cn");
    expect(detectPaymentRegion("en-US")).toBe("cn");
    setPaymentRegionOverride(null);
  });

  it("falls back to the payer language", () => {
    expect(detectPaymentRegion("zh-CN")).toBe("cn");
    expect(detectPaymentRegion("zh-Hans")).toBe("cn");
    expect(detectPaymentRegion("en-US")).toBe("overseas");
    expect(detectPaymentRegion("ja-JP")).toBe("overseas");
  });
});

describe("isMobileDevice", () => {
  it("detects phones and tablets", () => {
    expect(
      isMobileDevice(
        "Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 Mobile Safari/604.1",
      ),
    ).toBe(true);
    expect(
      isMobileDevice(
        "Mozilla/5.0 (Linux; Android 13; PEEM00) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Mobile Safari/537.36",
      ),
    ).toBe(true);
  });

  it("treats desktop browsers as non-mobile", () => {
    expect(
      isMobileDevice("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0"),
    ).toBe(false);
    expect(
      isMobileDevice("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Safari/605.1.15"),
    ).toBe(false);
  });
});
