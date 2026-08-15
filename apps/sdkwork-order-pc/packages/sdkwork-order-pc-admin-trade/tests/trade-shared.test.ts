import { describe, expect, it } from "vitest";
import {
  formatAmount,
  formatTimestamp,
  readTradeUrlStatusFilter,
} from "../src/components/trade-shared";

describe("trade shared helpers", () => {
  it("formats amounts with thousands separators and two decimals", () => {
    expect(formatAmount("99.00")).toBe("99.00");
    expect(formatAmount("1000.00")).toBe("1,000.00");
    expect(formatAmount("1234567.5", "en-US")).toBe("1,234,567.50");
    expect(formatAmount("50.00", "zh-CN", "CNY")).toBe("50.00 CNY");
  });

  it("falls back to the raw value for non-numeric amounts", () => {
    expect(formatAmount(undefined)).toBe("-");
    expect(formatAmount("abc")).toBe("abc");
    expect(formatAmount("")).toBe("-");
  });

  it("formats timestamps with the given locale", () => {
    const value = "2026-07-18T00:00:00.000Z";
    expect(formatTimestamp(undefined)).toBe("--");
    expect(formatTimestamp("not-a-date")).toBe("not-a-date");
    expect(formatTimestamp(value, "en-US")).toContain("2026");
  });

  it("reads the status deep link from the browser URL", () => {
    expect(readTradeUrlStatusFilter()).toBe("");
  });
});
