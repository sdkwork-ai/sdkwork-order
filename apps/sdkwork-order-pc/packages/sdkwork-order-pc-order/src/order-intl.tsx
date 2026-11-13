import {
  createContext,
  useContext,
  useMemo,
  type PropsWithChildren,
} from "react";
import { formatSdkworkOrderCurrencyCny as formatSdkworkCurrencyCny } from "@sdkwork/order-service";
import {
  resolveSdkworkOrderSubjectLabelKey,
} from "@sdkwork/order-pc-core/subject";
import type { SdkworkOrderFilter } from "./order-controller";
import {
  createSdkworkOrderMessages,
  normalizeSdkworkOrderLocale,
  type SdkworkOrderMessages,
  type SdkworkOrderMessagesOverrides,
} from "./order-copy";

export interface SdkworkOrderIntlValue {
  copy: SdkworkOrderMessages;
  formatCurrencyCny: (value: number | null | undefined) => string;
  formatDetailSummary: (orderId: string | number) => string;
  formatFilter: (filter: SdkworkOrderFilter) => string;
  formatItemMeta: (quantity: number | null | undefined, totalAmountCny: number | null | undefined) => string;
  formatPaginationPageLabel: (page: number, totalPages: number) => string;
  formatPaginationSummary: (shown: number, total: number) => string;
  formatPaymentMethod: (value: string | null | undefined) => string;
  formatStatus: (status: string | null | undefined, fallbackLabel?: string | null | undefined) => string;
  formatSubject: (subject: string | null | undefined) => string;
  formatTimelineLabel: (label: string | null | undefined) => string;
  formatTimestamp: (value: string | undefined) => string;
  locale: string;
}

export interface SdkworkOrderIntlProviderProps extends PropsWithChildren {
  locale?: string | null;
  messages?: SdkworkOrderMessagesOverrides;
}

function interpolateTemplate(template: string, values: Record<string, string>): string {
  return Object.entries(values).reduce(
    (output, [key, value]) => output.replaceAll(`{${key}}`, value),
    template,
  );
}

function normalizeStatusKey(
  value: string | null | undefined,
): keyof SdkworkOrderMessages["status"] {
  const normalized = String(value || "").trim().toLowerCase();

  if (normalized === "pending-payment") {
    return "pendingPayment";
  }

  if (
    normalized === "cancelled"
    || normalized === "completed"
    || normalized === "expired"
    || normalized === "paid"
    || normalized === "refunded"
    || normalized === "refunding"
  ) {
    return normalized;
  }

  return "unknown";
}

function normalizeFilterKey(value: SdkworkOrderFilter): keyof SdkworkOrderMessages["filters"] {
  const normalized = String(value).trim().toLowerCase();

  if (normalized === "pending-payment") {
    return "pendingPayment";
  }

  if (
    normalized === "all"
    || normalized === "cancelled"
    || normalized === "completed"
    || normalized === "expired"
    || normalized === "paid"
    || normalized === "refunded"
    || normalized === "refunding"
  ) {
    return normalized;
  }

  return "unknown";
}

function normalizePaymentMethodKey(
  value: string | null | undefined,
): keyof SdkworkOrderMessages["paymentMethod"] {
  const normalized = String(value || "").trim().toUpperCase().replaceAll("-", "_");

  if (normalized === "ALIPAY") {
    return "ALIPAY";
  }

  if (normalized === "BANKCARD" || normalized === "BANK_CARD") {
    return "BANKCARD";
  }

  if (normalized === "WECHAT" || normalized === "WECHAT_PAY") {
    return "WECHAT";
  }

  return "UNKNOWN";
}

function createSdkworkOrderIntlValue(
  locale?: string | null,
  overrides?: SdkworkOrderMessagesOverrides,
): SdkworkOrderIntlValue {
  const resolvedLocale = normalizeSdkworkOrderLocale(locale);
  const copy = createSdkworkOrderMessages(resolvedLocale, overrides);

  return {
    copy,
    formatCurrencyCny(value) {
      return formatSdkworkCurrencyCny(value, resolvedLocale);
    },
    formatDetailSummary(orderId) {
      return interpolateTemplate(copy.detail.summaryValue, {
        id: String(orderId),
      });
    },
    formatFilter(filter) {
      return copy.filters[normalizeFilterKey(filter)];
    },
    formatItemMeta(quantity, totalAmountCny) {
      return interpolateTemplate(copy.items.metaValue, {
        amount: formatSdkworkCurrencyCny(totalAmountCny, resolvedLocale),
        quantity: String(quantity ?? 0),
      });
    },
    formatPaginationPageLabel(page, totalPages) {
      return interpolateTemplate(copy.pagination.pageLabel, {
        page: String(page),
        totalPages: String(totalPages),
      });
    },
    formatPaginationSummary(shown, total) {
      return interpolateTemplate(copy.pagination.summary, {
        shown: String(shown),
        total: String(total),
      });
    },
    formatPaymentMethod(value) {
      const key = normalizePaymentMethodKey(value);
      if (key === "UNKNOWN") {
        return String(value || "").trim() || copy.paymentMethod.UNKNOWN;
      }

      return copy.paymentMethod[key];
    },
    formatStatus(status, fallbackLabel) {
      const key = normalizeStatusKey(status);
      if (
        key === "unknown"
        && fallbackLabel
        && fallbackLabel.trim()
        && fallbackLabel.trim().toLowerCase() !== "unknown"
      ) {
        return fallbackLabel;
      }

      return copy.status[key];
    },
    formatSubject(subject) {
      const raw = String(subject ?? "").trim();
      if (!raw) {
        return copy.service.summaryFallbackSubject;
      }

      // The shared catalog owns code normalization so `Token Bank Recharge`,
      // `token-bank-recharge`, and `token_bank_recharge` share one label.
      const canonicalKey = resolveSdkworkOrderSubjectLabelKey(raw);
      if (!canonicalKey) {
        return raw;
      }

      const code = canonicalKey.replace("orders.subject_", "");
      return copy.subject[code] ?? raw;
    },
    formatTimelineLabel(label) {
      const normalized = String(label || "").trim().toLowerCase();
      if (normalized === "created") {
        return copy.timeline.created;
      }

      if (normalized === "paid") {
        return copy.timeline.paid;
      }

      if (normalized === "current status") {
        return copy.timeline.currentStatus;
      }

      return String(label || "").trim() || copy.timeline.currentStatus;
    },
    formatTimestamp(value) {
      if (!value) {
        return copy.common.emptyValue;
      }

      const timestamp = new Date(value);
      if (Number.isNaN(timestamp.getTime())) {
        return value;
      }

      return new Intl.DateTimeFormat(resolvedLocale, {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(timestamp);
    },
    locale: resolvedLocale,
  };
}

const DEFAULT_SDKWORK_ORDER_INTL = createSdkworkOrderIntlValue();

const SdkworkOrderIntlContext = createContext<SdkworkOrderIntlValue>(
  DEFAULT_SDKWORK_ORDER_INTL,
);

export function SdkworkOrderIntlProvider({
  children,
  locale,
  messages,
}: SdkworkOrderIntlProviderProps) {
  const value = useMemo(
    () => createSdkworkOrderIntlValue(locale, messages),
    [locale, messages],
  );

  return (
    <SdkworkOrderIntlContext.Provider value={value}>
      {children}
    </SdkworkOrderIntlContext.Provider>
  );
}

export function useSdkworkOrderIntl(): SdkworkOrderIntlValue {
  return useContext(SdkworkOrderIntlContext);
}
