import {
  createContext,
  useContext,
  useMemo,
  type PropsWithChildren,
} from "react";
import { tradeAdminEnUsMessages } from "./en-US/commerce/order/admin-trade";
import { tradeAdminZhCnMessages } from "./zh-CN/commerce/order/admin-trade";

type LocaleMessages = Record<string, string>;

export type TradeAdminMessageKey = keyof typeof tradeAdminZhCnMessages;
export type TradeAdminMessagesOverrides = Partial<Record<TradeAdminMessageKey, string>>;

export interface TradeAdminIntlValue {
  locale: string;
  t: (key: string, fallback?: string, params?: Record<string, string | number>) => string;
}

export interface TradeAdminIntlProviderProps extends PropsWithChildren {
  /**
   * BCP 47 locale tag. `zh-CN` is the default to match the order surface
   * baseline; `en-US` (or any `en*`) selects the English fragment.
   */
  locale?: string | null;
  /** Optional per-key copy overrides, applied on top of the locale bundle. */
  messages?: TradeAdminMessagesOverrides;
}

/** Shared props accepted by trading center screens for host-injected copy. */
export interface TradeAdminIntlProps {
  locale?: string | null;
  messages?: TradeAdminMessagesOverrides;
}

const TradeAdminIntlContext = createContext<TradeAdminIntlValue | null>(null);

function resolveBundle(locale: string | null | undefined): LocaleMessages {
  const normalized = String(locale ?? "zh-CN").toLowerCase();
  if (normalized.startsWith("en")) {
    return tradeAdminEnUsMessages;
  }
  return tradeAdminZhCnMessages;
}

function interpolate(template: string, params: Record<string, string | number> | undefined): string {
  if (!params) {
    return template;
  }
  return Object.entries(params).reduce(
    (output, [key, value]) => output.replaceAll(`{{${key}}}`, String(value)),
    template,
  );
}

export function TradeAdminIntlProvider({
  children,
  locale,
  messages,
}: TradeAdminIntlProviderProps) {
  const value = useMemo<TradeAdminIntlValue>(() => {
    const bundle = resolveBundle(locale);
    const t = (key: string, fallback?: string, params?: Record<string, string | number>): string => {
      const resolved = messages?.[key as TradeAdminMessageKey] ?? bundle[key as TradeAdminMessageKey];
      const template = resolved ?? fallback ?? key;
      return interpolate(template, params);
    };
    return { locale: String(locale ?? "zh-CN"), t };
  }, [locale, messages]);

  return (
    <TradeAdminIntlContext.Provider value={value}>
      {children}
    </TradeAdminIntlContext.Provider>
  );
}

export function useTradeAdminI18n(): TradeAdminIntlValue {
  const value = useContext(TradeAdminIntlContext);
  if (!value) {
    // Standalone fallback so screens stay usable without an injected provider.
    return {
      locale: "zh-CN",
      t: (key, fallback) => fallback ?? key,
    };
  }
  return value;
}
