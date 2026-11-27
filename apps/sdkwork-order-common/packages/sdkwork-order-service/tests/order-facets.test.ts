import { describe, expect, it } from "vitest";
import {
  countSdkworkOrderActiveFacets,
  isSdkworkOrderScene,
  normalizeSdkworkOrderScene,
  normalizeSdkworkOrderTag,
  normalizeSdkworkOrderTags,
  SDKWORK_ORDER_SCENES,
  SDKWORK_ORDER_TAG_FILTER_MAX_COUNT,
  SDKWORK_ORDER_TAG_MAX_LENGTH,
  toSdkworkOrderFacetParams,
} from "../src/order-facets.ts";

describe("order scene facets", () => {
  it("lists the closed scene set the backend accepts", () => {
    expect([...SDKWORK_ORDER_SCENES]).toEqual([
      "web",
      "h5",
      "app",
      "mini_program",
      "cashier",
      "membership",
      "recharge",
      "platform",
      "default",
    ]);
  });

  it("recognizes every canonical scene", () => {
    for (const scene of SDKWORK_ORDER_SCENES) {
      expect(isSdkworkOrderScene(scene)).toBe(true);
      expect(normalizeSdkworkOrderScene(scene)).toBe(scene);
    }
  });

  it("resolves documented aliases to the canonical scene", () => {
    expect(normalizeSdkworkOrderScene("pc")).toBe("web");
    expect(normalizeSdkworkOrderScene("WAP")).toBe("h5");
    expect(normalizeSdkworkOrderScene("miniProgram")).toBe("mini_program");
    expect(normalizeSdkworkOrderScene("mini-program")).toBe("mini_program");
    expect(normalizeSdkworkOrderScene("flutter")).toBe("app");
    expect(normalizeSdkworkOrderScene("subscription")).toBe("membership");
    expect(normalizeSdkworkOrderScene("topup")).toBe("recharge");
  });

  it("returns undefined for an unknown scene instead of guessing", () => {
    expect(normalizeSdkworkOrderScene("telepathy")).toBeUndefined();
    expect(normalizeSdkworkOrderScene(undefined)).toBeUndefined();
    expect(normalizeSdkworkOrderScene("")).toBeUndefined();
  });
});

describe("order tag canonicalization", () => {
  it("lowercases and folds separators", () => {
    expect(normalizeSdkworkOrderTag("VIP")).toBe("vip");
    expect(normalizeSdkworkOrderTag("  vip  ")).toBe("vip");
    expect(normalizeSdkworkOrderTag("summer sale")).toBe("summer-sale");
    expect(normalizeSdkworkOrderTag("Risk__Review")).toBe("risk-review");
    expect(normalizeSdkworkOrderTag("a--b")).toBe("a-b");
    expect(normalizeSdkworkOrderTag("v2.0")).toBe("v2-0");
  });

  it("never starts or ends a tag with a separator", () => {
    expect(normalizeSdkworkOrderTag("-lead")).toBe("lead");
    expect(normalizeSdkworkOrderTag("trail-")).toBe("trail");
    expect(normalizeSdkworkOrderTag(" -- both -- ")).toBe("both");
  });

  it("keeps CJK tags", () => {
    expect(normalizeSdkworkOrderTag("高价值")).toBe("高价值");
  });

  it("is idempotent", () => {
    for (const input of ["VIP", "summer sale", "a--b", "-lead", "高价值", "v2.0"]) {
      const once = normalizeSdkworkOrderTag(input);
      expect(once).toBeTruthy();
      expect(normalizeSdkworkOrderTag(once)).toBe(once);
    }
  });

  it("drops blank input", () => {
    for (const input of ["", "   ", "---", "___", " - "]) {
      expect(normalizeSdkworkOrderTag(input)).toBeUndefined();
    }
  });

  it("rejects characters the server would reject", () => {
    expect(normalizeSdkworkOrderTag("vip!")).toBeUndefined();
    expect(normalizeSdkworkOrderTag("a/b")).toBeUndefined();
    expect(normalizeSdkworkOrderTag('vip" OR 1=1')).toBeUndefined();
  });

  it("enforces the length ceiling", () => {
    expect(normalizeSdkworkOrderTag("x".repeat(SDKWORK_ORDER_TAG_MAX_LENGTH))).toBeDefined();
    expect(normalizeSdkworkOrderTag("x".repeat(SDKWORK_ORDER_TAG_MAX_LENGTH + 1))).toBeUndefined();
  });
});

describe("order tag list normalization", () => {
  it("accepts a comma or pipe separated string", () => {
    expect(normalizeSdkworkOrderTags("vip, risk")).toEqual(["vip", "risk"]);
    expect(normalizeSdkworkOrderTags("vip|risk")).toEqual(["vip", "risk"]);
  });

  it("accepts an array of fragments", () => {
    expect(normalizeSdkworkOrderTags(["vip,risk", "campaign"])).toEqual([
      "vip",
      "risk",
      "campaign",
    ]);
  });

  it("deduplicates case-insensitively while preserving first-seen order", () => {
    expect(normalizeSdkworkOrderTags(["VIP", "vip", "risk"])).toEqual(["vip", "risk"]);
  });

  it("drops blank and invalid tokens rather than failing the whole list", () => {
    expect(normalizeSdkworkOrderTags(["vip", " ", "", "bad!", "risk"])).toEqual(["vip", "risk"]);
  });

  it("bounds the list", () => {
    const many = Array.from({ length: SDKWORK_ORDER_TAG_FILTER_MAX_COUNT + 5 }, (_, index) => `tag${index}`);
    expect(normalizeSdkworkOrderTags(many)).toHaveLength(SDKWORK_ORDER_TAG_FILTER_MAX_COUNT);
  });

  it("handles empty input", () => {
    expect(normalizeSdkworkOrderTags(undefined)).toEqual([]);
    expect(normalizeSdkworkOrderTags(null)).toEqual([]);
    expect(normalizeSdkworkOrderTags("")).toEqual([]);
  });
});

describe("order facet request params", () => {
  it("omits unset facets so SDK defaults survive", () => {
    expect(toSdkworkOrderFacetParams(undefined)).toEqual({});
    expect(toSdkworkOrderFacetParams({})).toEqual({});
    expect(toSdkworkOrderFacetParams({ tags: [] })).toEqual({});
  });

  it("sends a canonical scene with its tags and default match mode", () => {
    expect(toSdkworkOrderFacetParams({ scene: "h5", tags: ["VIP", "risk"] })).toEqual({
      scene: "h5",
      tags: ["vip", "risk"],
      tagMatch: "any",
    });
  });

  it("never sends an unrecognized scene", () => {
    expect(toSdkworkOrderFacetParams({ scene: "telepathy" as never }).scene).toBeUndefined();
  });

  it("passes an explicit match mode through", () => {
    expect(toSdkworkOrderFacetParams({ tags: ["vip"], tagMatch: "all" }).tagMatch).toBe("all");
  });
});

describe("active facet counting", () => {
  it("counts each active facet dimension once", () => {
    expect(countSdkworkOrderActiveFacets(undefined)).toBe(0);
    expect(countSdkworkOrderActiveFacets({})).toBe(0);
    expect(countSdkworkOrderActiveFacets({ scene: "h5" })).toBe(1);
    expect(countSdkworkOrderActiveFacets({ tags: ["vip"] })).toBe(1);
    expect(countSdkworkOrderActiveFacets({ scene: "h5", tags: ["vip", "risk"] })).toBe(2);
  });
});
