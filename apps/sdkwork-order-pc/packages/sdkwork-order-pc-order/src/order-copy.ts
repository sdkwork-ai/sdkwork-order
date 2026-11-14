export type SdkworkOrderLocale = "en-US" | "zh-CN";

export type SdkworkOrderMessagesOverrides = DeepPartial<SdkworkOrderMessages>;

export interface SdkworkOrderMessages {
  actions: {
    cancel: string;
    close: string;
    pay: string;
    refresh: string;
    viewDetails: string;
  };
  common: {
    emptyValue: string;
  };
  controller: {
    bootstrapFailed: string;
    cancelFailed: string;
    detailFailed: string;
    payFailed: string;
  };
  detail: {
    description: string;
    loading: string;
    paidAmount: string;
    status: string;
    summaryValue: string;
    title: string;
    totalAmount: string;
  };
  filters: {
    all: string;
    cancelled: string;
    completed: string;
    expired: string;
    paid: string;
    pendingPayment: string;
    refunded: string;
    refunding: string;
    unknown: string;
  };
  items: {
    description: string;
    empty: string;
    metaValue: string;
    title: string;
  };
  manifest: {
    description: string;
    title: string;
  };
  overview: {
    createdAt: string;
    description: string;
    orderSn: string;
    outTradeNo: string;
    paidAt: string;
    paymentMethod: string;
    title: string;
    transactionId: string;
  };
  page: {
    description: string;
    errorTitle: string;
    eyebrow: string;
    loading: string;
    title: string;
  };
  pagination: {
    next: string;
    pageLabel: string;
    prev: string;
    summary: string;
  };
  paymentMethod: {
    ALIPAY: string;
    BANKCARD: string;
    UNKNOWN: string;
    WECHAT: string;
  };
  service: {
    cancelFailed: string;
    clientMethodUnavailable: string;
    detailFailed: string;
    itemFallbackName: string;
    payFailed: string;
    requestFailed: string;
    signInRequired: string;
    summaryFallbackSubject: string;
  };
  stats: {
    completed: string;
    pendingPayment: string;
    totalAmount: string;
    totalOrders: string;
  };
  status: {
    cancelled: string;
    completed: string;
    expired: string;
    paid: string;
    pendingPayment: string;
    refunded: string;
    refunding: string;
    unknown: string;
  };
  /**
   * `commerce_order.subject` wire code → localized display label.
   *
   * Open-ended on purpose: hosts may append product-specific subjects without
   * changing the type, and an unknown code always falls back to the raw value
   * (see `formatSubject` in `order-intl.tsx`).
   */
  subject: Record<string, string>;
  timeline: {
    created: string;
    currentStatus: string;
    description: string;
    empty: string;
    paid: string;
    title: string;
  };
  views: {
    empty: string;
    eyebrow: string;
    title: string;
  };
}

type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends (...args: never[]) => unknown
    ? T[K]
    : T[K] extends object
      ? DeepPartial<T[K]>
      : T[K];
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function mergeDeep<T>(base: T, overrides?: DeepPartial<T>): T {
  if (!overrides) {
    return base;
  }

  const output: Record<string, unknown> = {
    ...(base as Record<string, unknown>),
  };

  for (const [key, value] of Object.entries(overrides)) {
    if (value === undefined) {
      continue;
    }

    const baseValue = output[key];
    output[key] = isRecord(baseValue) && isRecord(value)
      ? mergeDeep(baseValue, value)
      : value;
  }

  return output as T;
}

const EN_US_MESSAGES: SdkworkOrderMessages = {
  actions: {
    cancel: "Cancel order",
    close: "Close",
    pay: "Pay now",
    refresh: "Refresh",
    viewDetails: "View details",
  },
  common: {
    emptyValue: "--",
  },
  controller: {
    bootstrapFailed: "Failed to load order center.",
    cancelFailed: "Failed to cancel order.",
    detailFailed: "Failed to load order detail.",
    payFailed: "Failed to retry payment.",
  },
  detail: {
    description: "Inspect billing metadata, payment progress, and ordered items.",
    loading: "Loading order detail...",
    paidAmount: "Paid amount",
    status: "Status",
    summaryValue: "Order #{id}",
    title: "Order detail",
    totalAmount: "Total amount",
  },
  filters: {
    all: "All",
    cancelled: "Cancelled",
    completed: "Completed",
    expired: "Expired",
    paid: "Paid",
    pendingPayment: "Pending payment",
    refunded: "Refunded",
    refunding: "Refunding",
    unknown: "Unknown",
  },
  items: {
    description: "Ordered products and quantities.",
    empty: "No order items were returned for this order.",
    metaValue: "Qty {quantity} | {amount}",
    title: "Items",
  },
  manifest: {
    description: "Order workspace for billing history, payment retries, and order-detail drawer routing.",
    title: "Orders",
  },
  overview: {
    createdAt: "Created",
    description: "Core identifiers and payment routing details.",
    orderSn: "Order SN",
    outTradeNo: "Out trade no",
    paidAt: "Paid",
    paymentMethod: "Payment method",
    title: "Overview",
    transactionId: "Transaction",
  },
  page: {
    description: "Review billing history, payment status, and spending summaries — all in one place.",
    errorTitle: "Order center error",
    eyebrow: "Commercial orders",
    loading: "Loading order center...",
    title: "Bills & Reports",
  },
  pagination: {
    next: "Next",
    pageLabel: "Page {page} of {totalPages}",
    prev: "Previous",
    summary: "Showing {shown} of {total} bills",
  },
  paymentMethod: {
    ALIPAY: "Alipay",
    BANKCARD: "Bank card",
    UNKNOWN: "Payment",
    WECHAT: "WeChat Pay",
  },
  service: {
    cancelFailed: "Failed to cancel order.",
    clientMethodUnavailable: "{name} is unavailable on the current app client.",
    detailFailed: "Failed to load order detail.",
    itemFallbackName: "Order item",
    payFailed: "Failed to start payment.",
    requestFailed: "Request failed.",
    signInRequired: "Please sign in to manage orders and payments.",
    summaryFallbackSubject: "Order",
  },
  stats: {
    completed: "Completed",
    pendingPayment: "Pending",
    totalAmount: "Total Spend",
    totalOrders: "Total Bills",
  },
  status: {
    cancelled: "Cancelled",
    completed: "Completed",
    expired: "Expired",
    paid: "Paid",
    pendingPayment: "Pending payment",
    refunded: "Refunded",
    refunding: "Refunding",
    unknown: "Unknown",
  },
  subject: {
    account_recharge_package: "Account value package recharge",
    cash_withdrawal: "Cash withdrawal",
    coupon_recharge: "Coupon recharge",
    membership: "Membership subscription",
    points_recharge: "Membership Points recharge",
    product: "Product order",
    refund_request: "Refund request",
    token_bank_plan_purchase: "Token Bank plan purchase",
    token_bank_plan_renewal: "Token Bank plan renewal",
    token_bank_recharge: "Token Bank recharge",
    virtual_goods: "Virtual goods order",
  },
  timeline: {
    created: "Created",
    currentStatus: "Current status",
    description: "Payment and fulfillment milestones.",
    empty: "No timeline events are available for this order.",
    paid: "Paid",
    title: "Timeline",
  },
  views: {
    empty: "No bills matched the current filter.",
    eyebrow: "Details",
    title: "Billing Details",
  },
};

const ZH_CN_MESSAGES: SdkworkOrderMessages = {
  actions: {
    cancel: "\u53d6\u6d88\u8ba2\u5355",
    close: "\u5173\u95ed",
    pay: "\u7acb\u5373\u652f\u4ed8",
    refresh: "\u5237\u65b0",
    viewDetails: "\u67e5\u770b\u8be6\u60c5",
  },
  common: {
    emptyValue: "--",
  },
  controller: {
    bootstrapFailed: "\u52a0\u8f7d\u8ba2\u5355\u4e2d\u5fc3\u5931\u8d25\u3002",
    cancelFailed: "\u53d6\u6d88\u8ba2\u5355\u5931\u8d25\u3002",
    detailFailed: "\u52a0\u8f7d\u8ba2\u5355\u8be6\u60c5\u5931\u8d25\u3002",
    payFailed: "\u91cd\u8bd5\u652f\u4ed8\u5931\u8d25\u3002",
  },
  detail: {
    description: "\u67e5\u770b\u8d26\u5355\u5143\u6570\u636e\u3001\u652f\u4ed8\u8fdb\u5ea6\u548c\u8ba2\u5355\u5546\u54c1\u660e\u7ec6\u3002",
    loading: "\u6b63\u5728\u52a0\u8f7d\u8ba2\u5355\u8be6\u60c5...",
    paidAmount: "\u5df2\u652f\u4ed8\u91d1\u989d",
    status: "\u72b6\u6001",
    summaryValue: "\u8ba2\u5355 #{id}",
    title: "\u8ba2\u5355\u8be6\u60c5",
    totalAmount: "\u8ba2\u5355\u603b\u989d",
  },
  filters: {
    all: "\u5168\u90e8",
    cancelled: "\u5df2\u53d6\u6d88",
    completed: "\u5df2\u5b8c\u6210",
    expired: "\u5df2\u8fc7\u671f",
    paid: "\u5df2\u652f\u4ed8",
    pendingPayment: "\u5f85\u652f\u4ed8",
    refunded: "\u5df2\u9000\u6b3e",
    refunding: "\u9000\u6b3e\u4e2d",
    unknown: "\u672a\u77e5",
  },
  items: {
    description: "\u5df2\u4e0b\u5355\u7684\u5546\u54c1\u4e0e\u6570\u91cf\u660e\u7ec6\u3002",
    empty: "\u5f53\u524d\u8ba2\u5355\u6682\u672a\u8fd4\u56de\u5546\u54c1\u660e\u7ec6\u3002",
    metaValue: "\u6570\u91cf {quantity} | {amount}",
    title: "\u5546\u54c1\u660e\u7ec6",
  },
  manifest: {
    description: "\u7528\u4e8e\u8d26\u5355\u5386\u53f2\u67e5\u770b\u3001\u652f\u4ed8\u91cd\u8bd5\u4e0e\u8ba2\u5355\u8be6\u60c5\u62bd\u5c49\u8def\u7531\u7684\u8ba2\u5355\u5de5\u4f5c\u533a\u3002",
    title: "\u8ba2\u5355",
  },
  overview: {
    createdAt: "\u521b\u5efa\u65f6\u95f4",
    description: "\u67e5\u770b\u8ba2\u5355\u6807\u8bc6\u548c\u652f\u4ed8\u8def\u7531\u660e\u7ec6\u3002",
    orderSn: "\u8ba2\u5355\u53f7",
    outTradeNo: "\u5546\u6237\u8ba2\u5355\u53f7",
    paidAt: "\u652f\u4ed8\u65f6\u95f4",
    paymentMethod: "\u652f\u4ed8\u65b9\u5f0f",
    title: "\u6982\u89c8",
    transactionId: "\u4ea4\u6613\u6d41\u6c34",
  },
  page: {
    description: "\u96c6\u4e2d\u67e5\u770b\u8d26\u5355\u5386\u53f2\u3001\u652f\u4ed8\u72b6\u6001\u4e0e\u6d88\u8d39\u6c47\u603b\uff0c\u638c\u63e1\u6bcf\u4e00\u7b14\u8d26\u5355\u7684\u5b8c\u6574\u751f\u547d\u5468\u671f\u3002",
    errorTitle: "\u8ba2\u5355\u4e2d\u5fc3\u5f02\u5e38",
    eyebrow: "\u5546\u4e1a\u5316\u8ba2\u5355",
    loading: "\u6b63\u5728\u52a0\u8f7d\u8ba2\u5355\u4e2d\u5fc3...",
    title: "\u8d26\u5355\u4e0e\u62a5\u8868",
  },
  pagination: {
    next: "\u4e0b\u4e00\u9875",
    pageLabel: "\u7b2c {page} / {totalPages} \u9875",
    prev: "\u4e0a\u4e00\u9875",
    summary: "\u5171 {total} \u6761\u8d26\u5355\uff0c\u5f53\u524d\u5c55\u793a {shown} \u6761",
  },
  paymentMethod: {
    ALIPAY: "\u652f\u4ed8\u5b9d",
    BANKCARD: "\u94f6\u884c\u5361",
    UNKNOWN: "\u652f\u4ed8",
    WECHAT: "\u5fae\u4fe1\u652f\u4ed8",
  },
  service: {
    cancelFailed: "\u53d6\u6d88\u8ba2\u5355\u5931\u8d25\u3002",
    clientMethodUnavailable: "\u5f53\u524d\u5e94\u7528\u5ba2\u6237\u7aef\u672a\u63d0\u4f9b {name} \u80fd\u529b\u3002",
    detailFailed: "\u52a0\u8f7d\u8ba2\u5355\u8be6\u60c5\u5931\u8d25\u3002",
    itemFallbackName: "\u8ba2\u5355\u5546\u54c1",
    payFailed: "\u53d1\u8d77\u652f\u4ed8\u5931\u8d25\u3002",
    requestFailed: "\u8bf7\u6c42\u5931\u8d25\u3002",
    signInRequired: "\u8bf7\u5148\u767b\u5f55\u540e\u518d\u7ba1\u7406\u8ba2\u5355\u4e0e\u652f\u4ed8\u3002",
    summaryFallbackSubject: "\u8ba2\u5355",
  },
  stats: {
    completed: "\u5df2\u5b8c\u6210",
    pendingPayment: "\u5f85\u652f\u4ed8",
    totalAmount: "\u6d88\u8d39\u603b\u989d",
    totalOrders: "\u8d26\u5355\u603b\u6570",
  },
  status: {
    cancelled: "\u5df2\u53d6\u6d88",
    completed: "\u5df2\u5b8c\u6210",
    expired: "\u5df2\u8fc7\u671f",
    paid: "\u5df2\u652f\u4ed8",
    pendingPayment: "\u5f85\u652f\u4ed8",
    refunded: "\u5df2\u9000\u6b3e",
    refunding: "\u9000\u6b3e\u4e2d",
    unknown: "\u672a\u77e5",
  },
  subject: {
    account_recharge_package: "\u8d26\u6237\u4ef7\u503c\u5305\u5145\u503c",
    cash_withdrawal: "\u63d0\u73b0\u7533\u8bf7",
    coupon_recharge: "\u5151\u6362\u7801\u5145\u503c",
    membership: "\u4f1a\u5458\u8ba2\u9605",
    points_recharge: "\u4f1a\u5458\u79ef\u5206\u5145\u503c",
    product: "\u5546\u54c1\u8ba2\u5355",
    refund_request: "\u9000\u6b3e\u7533\u8bf7",
    token_bank_plan_purchase: "Token Bank \u5957\u9910\u8d2d\u4e70",
    token_bank_plan_renewal: "Token Bank \u5957\u9910\u7eed\u8ba2",
    token_bank_recharge: "Token Bank \u5145\u503c",
    virtual_goods: "\u865a\u62df\u5546\u54c1\u8ba2\u5355",
  },
  timeline: {
    created: "\u5df2\u521b\u5efa",
    currentStatus: "\u5f53\u524d\u72b6\u6001",
    description: "\u652f\u4ed8\u4e0e\u5c65\u7ea6\u8fdb\u5ea6\u91cc\u7a0b\u7891\u3002",
    empty: "\u5f53\u524d\u8ba2\u5355\u6682\u65e0\u65f6\u95f4\u7ebf\u8bb0\u5f55\u3002",
    paid: "\u5df2\u652f\u4ed8",
    title: "\u65f6\u95f4\u7ebf",
  },
  views: {
    empty: "\u5f53\u524d\u7b5b\u9009\u6761\u4ef6\u4e0b\u6ca1\u6709\u5339\u914d\u7684\u8d26\u5355\u8bb0\u5f55\u3002",
    eyebrow: "\u660e\u7ec6",
    title: "\u8d26\u5355\u660e\u7ec6",
  },
};

const SDKWORK_ORDER_MESSAGES: Record<SdkworkOrderLocale, SdkworkOrderMessages> = {
  "en-US": EN_US_MESSAGES,
  "zh-CN": ZH_CN_MESSAGES,
};

export function normalizeSdkworkOrderLocale(locale?: string | null): SdkworkOrderLocale {
  const normalized = String(locale || "").trim().toLowerCase();
  if (normalized.startsWith("zh")) {
    return "zh-CN";
  }

  return "en-US";
}

export function createSdkworkOrderMessages(
  locale?: string | null,
  overrides?: SdkworkOrderMessagesOverrides,
): SdkworkOrderMessages {
  return mergeDeep(
    SDKWORK_ORDER_MESSAGES[normalizeSdkworkOrderLocale(locale)],
    overrides,
  );
}
