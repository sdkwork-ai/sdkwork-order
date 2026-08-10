import { describe, expect, it } from "vitest";

import {
  localizeOrderTitle,
  normalizeOrderLabel,
  type TranslateFunction,
} from "./orderTitle";

/** Records (key, defaultValue) pairs passed through the stub translator. */
function createStubTranslator(): {
  t: TranslateFunction;
  calls: ReadonlyArray<readonly [string, string]>;
} {
  const calls: Array<readonly [string, string]> = [];
  const t: TranslateFunction = (key, defaultValue) => {
    calls.push([key, defaultValue ?? ""]);
    // Simulate a host that ships the `orders.subject_*` resources.
    return `[[${key}]]`;
  };
  return { t, calls };
}

describe("normalizeOrderLabel", () => {
  it("trims, lowercases and collapses whitespace", () => {
    expect(normalizeOrderLabel("  Membership  ")).toBe("membership");
    expect(normalizeOrderLabel("VIP")).toBe("vip");
    expect(normalizeOrderLabel("Membership Subscription")).toBe("membership subscription");
  });

  it("treats underscores as spaces", () => {
    expect(normalizeOrderLabel("points_recharge")).toBe("points recharge");
    expect(normalizeOrderLabel("TOKEN_BANK_RECHARGE")).toBe("token bank recharge");
  });
});

describe("localizeOrderTitle", () => {
  it("maps the membership subject onto the membership resource key", () => {
    const { t, calls } = createStubTranslator();
    expect(localizeOrderTitle("Membership", t)).toBe("[[orders.subject_membership]]");
    expect(localizeOrderTitle("membership", t)).toBe("[[orders.subject_membership]]");
    expect(calls).toHaveLength(2);
    expect(calls[0][0]).toBe("orders.subject_membership");
    expect(calls[0][1]).toBe("Membership");
  });

  it("maps the recharge wire identifier onto the points recharge key", () => {
    const { t, calls } = createStubTranslator();
    expect(localizeOrderTitle("points_recharge", t)).toBe("[[orders.subject_points_recharge]]");
    expect(localizeOrderTitle("Points Recharge", t)).toBe("[[orders.subject_points_recharge]]");
    expect(calls).toHaveLength(2);
    expect(calls[1][0]).toBe("orders.subject_points_recharge");
  });

  it("maps the token bank recharge identifier onto its key", () => {
    const { t } = createStubTranslator();
    expect(localizeOrderTitle("token_bank_recharge", t)).toBe(
      "[[orders.subject_token_bank_recharge]]",
    );
  });

  it("keeps unknown labels unchanged without calling t", () => {
    const { t, calls } = createStubTranslator();
    expect(localizeOrderTitle("黄金会员月卡", t)).toBe("黄金会员月卡");
    expect(localizeOrderTitle("Custom Shop Item", t)).toBe("Custom Shop Item");
    expect(localizeOrderTitle("", t)).toBe("");
    expect(calls).toHaveLength(0);
  });

  it("falls back to the raw label when the resource is missing", () => {
    const t: TranslateFunction = (key, defaultValue) => defaultValue ?? key;
    expect(localizeOrderTitle("membership", t)).toBe("membership");
    expect(localizeOrderTitle("unknown label", t)).toBe("unknown label");
  });
});
