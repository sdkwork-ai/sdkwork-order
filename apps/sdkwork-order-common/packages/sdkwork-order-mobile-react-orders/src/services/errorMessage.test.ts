import { describe, expect, it } from "vitest";

import { toUserErrorMessage, type TranslateFunction } from "./errorMessage";

/** Minimal t() fake mirroring i18next: returns the key itself when missing. */
function createTranslate(resources: Record<string, string>): TranslateFunction {
  return (key: string) => resources[key] ?? key;
}

describe("toUserErrorMessage", () => {
  const t = createTranslate({
    "errors.result.40901": "操作冲突，请刷新后重试",
    "errors.network": "网络连接异常，请检查网络后重试",
  });

  it("prefers the problem i18nKey translation when a resource exists", () => {
    const error = Object.assign(new Error("Conflict"), {
      problem: {
        code: 40901,
        detail: "payment method has no eligible channel",
        i18nKey: "errors.result.40901",
        traceId: "trace-1",
      },
    });
    expect(toUserErrorMessage(t, error)).toBe("操作冲突，请刷新后重试");
  });

  it("falls back to the server detail when the i18nKey has no resource", () => {
    const error = Object.assign(new Error("Conflict"), {
      problem: {
        code: 40999,
        detail: "payment method has no eligible channel",
        i18nKey: "errors.result.40999",
        traceId: "trace-1",
      },
    });
    expect(toUserErrorMessage(t, error)).toBe("payment method has no eligible channel");
  });

  it("maps SDK error code families onto localized keys", () => {
    const error = Object.assign(new Error("fetch failed"), { code: "NETWORK_ERROR" });
    expect(toUserErrorMessage(t, error)).toBe("网络连接异常，请检查网络后重试");
  });

  it("falls back to error.message for plain errors", () => {
    expect(toUserErrorMessage(t, new Error("boom"))).toBe("boom");
  });

  it("falls back to String() for non-Error values", () => {
    expect(toUserErrorMessage(t, 42)).toBe("42");
    expect(toUserErrorMessage(t, undefined)).toBe("undefined");
  });
});
