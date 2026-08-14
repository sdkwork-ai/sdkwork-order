import {
  createContext,
  useContext,
  useMemo,
  type PropsWithChildren,
} from "react";
import { adminOrdersEnUsMessages } from "./en-US/commerce/order/admin-orders";
import { adminOrdersZhCnMessages } from "./zh-CN/commerce/order/admin-orders";

type LocaleMessages = Record<string, string>;

export type AdminOrdersMessageKey = keyof typeof adminOrdersZhCnMessages;
export type AdminOrdersMessagesOverrides = Partial<Record<AdminOrdersMessageKey, string>>;

export interface AdminOrdersIntlValue {
  locale: string;
  t: (key: string, fallback?: string, params?: Record<string, string | number>) => string;
}

export interface AdminOrdersIntlProviderProps extends PropsWithChildren {
  /**
   * BCP 47 locale tag. `zh-CN` is the default to match the order surface
   * baseline; `en-US` (or any `en*`) selects the English fragment.
   */
  locale?: string | null;
  /** Optional per-key copy overrides, applied on top of the locale bundle. */
  messages?: AdminOrdersMessagesOverrides;
}

/** Shared props accepted by order supervision screens for host-injected copy. */
export interface AdminOrdersIntlProps {
  locale?: string | null;
  messages?: AdminOrdersMessagesOverrides;
}

const AdminOrdersIntlContext = createContext<AdminOrdersIntlValue | null>(null);

function resolveBundle(locale: string | null | undefined): LocaleMessages {
  const normalized = String(locale ?? "zh-CN").toLowerCase();
  if (normalized.startsWith("en")) {
    return adminOrdersEnUsMessages;
  }
  return adminOrdersZhCnMessages;
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

export function AdminOrdersIntlProvider({
  children,
  locale,
  messages,
}: AdminOrdersIntlProviderProps) {
  const value = useMemo<AdminOrdersIntlValue>(() => {
    const bundle = resolveBundle(locale);
    const t = (key: string, fallback?: string, params?: Record<string, string | number>): string => {
      const resolved = messages?.[key as AdminOrdersMessageKey] ?? bundle[key as AdminOrdersMessageKey];
      const template = resolved ?? fallback ?? key;
      return interpolate(template, params);
    };
    return { locale: String(locale ?? "zh-CN"), t };
  }, [locale, messages]);

  return (
    <AdminOrdersIntlContext.Provider value={value}>
      {children}
    </AdminOrdersIntlContext.Provider>
  );
}

export function useAdminOrdersI18n(): AdminOrdersIntlValue {
  const value = useContext(AdminOrdersIntlContext);
  if (!value) {
    // Standalone fallback so screens stay usable without an injected provider.
    return {
      locale: "zh-CN",
      t: (key, fallback) => fallback ?? key,
    };
  }
  return value;
}
