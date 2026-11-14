import { describe, expect, it } from "vitest";

import {
  SDKWORK_ORDER_SUBJECT_LABEL_KEYS,
  SDKWORK_ORDER_TARGET_ASSET_LABEL_KEYS,
  localizeSdkworkOrderSubject,
  localizeSdkworkOrderTargetAsset,
  resolveSdkworkOrderSubjectLabelKey,
} from "@sdkwork/order-pc-core/subject";
import { tradeAdminEnUsMessages } from "../src/i18n/en-US/commerce/order/admin-trade";
import { tradeAdminZhCnMessages } from "../src/i18n/zh-CN/commerce/order/admin-trade";

type Translate = (key: string, fallback?: string) => string;

const zh: Translate = (key, fallback) =>
  (tradeAdminZhCnMessages as Record<string, string>)[key] ?? fallback ?? key;
const en: Translate = (key, fallback) =>
  (tradeAdminEnUsMessages as Record<string, string>)[key] ?? fallback ?? key;

describe("order subject label catalog", () => {
  it("resolves every documented backend subject code", () => {
    for (const code of [
      "product",
      "virtual_goods",
      "membership",
      "points_recharge",
      "token_bank_recharge",
      "token_bank_plan_purchase",
      "token_bank_plan_renewal",
      "account_recharge_package",
      "coupon_recharge",
      "refund_request",
      "cash_withdrawal",
    ]) {
      expect(resolveSdkworkOrderSubjectLabelKey(code), code).not.toBeNull();
    }
  });

  it("normalizes spelling variants onto one label", () => {
    for (const variant of ["points_recharge", "Points Recharge", "points-recharge", " POINTS_RECHARGE "]) {
      expect(localizeSdkworkOrderSubject(variant, zh), variant).toBe("会员积分充值");
    }
  });

  it("renders the Token Bank subject as Token Bank rather than a points account", () => {
    expect(localizeSdkworkOrderSubject("token_bank_recharge", zh)).toBe("Token Bank 充值");
    expect(localizeSdkworkOrderSubject("token_bank_recharge", en)).toBe("Token Bank recharge");
  });

  it("ships a zh-CN and en-US label for every catalog key", () => {
    for (const key of SDKWORK_ORDER_SUBJECT_LABEL_KEYS) {
      expect((tradeAdminZhCnMessages as Record<string, string>)[key], key).toBeTruthy();
      expect((tradeAdminEnUsMessages as Record<string, string>)[key], key).toBeTruthy();
    }
    for (const key of SDKWORK_ORDER_TARGET_ASSET_LABEL_KEYS) {
      expect((tradeAdminZhCnMessages as Record<string, string>)[key], key).toBeTruthy();
      expect((tradeAdminEnUsMessages as Record<string, string>)[key], key).toBeTruthy();
    }
  });

  it("never masks a free-form title stored in the subject column", () => {
    expect(localizeSdkworkOrderSubject("Token Bank 100", zh)).toBe("Token Bank 100");
    expect(localizeSdkworkOrderSubject("黄金会员月卡", zh)).toBe("黄金会员月卡");
    expect(localizeSdkworkOrderSubject("", zh)).toBe("");
    expect(localizeSdkworkOrderSubject(undefined, zh)).toBe("");
  });

  it("localizes account-value asset codes used by refund and withdrawal requests", () => {
    expect(localizeSdkworkOrderTargetAsset("token_bank", zh)).toBe("Token Bank");
    expect(localizeSdkworkOrderTargetAsset("points", zh)).toBe("会员积分");
    expect(localizeSdkworkOrderTargetAsset("cash", en)).toBe("Cash");
    expect(localizeSdkworkOrderTargetAsset("mystery_asset", en)).toBe("mystery_asset");
  });
});
