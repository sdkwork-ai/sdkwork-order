import { useEffect, useState, type ReactNode } from "react";
import {
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
} from "lucide-react";
import {
  Button,
  StatusBadge,
  type StatusBadgeVariant,
} from "@sdkwork/ui-pc-react";
import { useTradeAdminI18n } from "../i18n/intl";

export function formatTimestamp(value?: string, locale?: string): string {
  if (!value) return "--";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString(locale ?? "zh-CN");
}

/**
 * Formats a decimal amount string with thousands separators and two fraction
 * digits, optionally appending the currency code. Invalid values are returned
 * as-is so backend decimals never corrupt the display.
 */
export function formatAmount(value?: string, locale?: string, currencyCode?: string): string {
  if (!value) return "-";
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return value;
  const formatted = numeric.toLocaleString(locale ?? "zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
  return currencyCode ? `${formatted} ${currencyCode}` : formatted;
}

/** Serializes the current page of a list into a UTF-8 CSV file for download. */
export function exportTradeListCsv(
  filename: string,
  headers: string[],
  rows: (string | number | null | undefined)[][],
): void {
  const escapeCell = (cell: string | number | null | undefined): string => {
    const text = String(cell ?? "");
    return /[",\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
  };
  const lines = [headers, ...rows].map((row) => row.map(escapeCell).join(","));
  // UTF-8 BOM keeps Chinese headers readable in Excel.
  const blob = new Blob([`\uFEFF${lines.join("\r\n")}`], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

/** Builds a `yyyyMMdd`-suffixed CSV filename from a list title. */
export function tradeCsvFilename(title: string): string {
  const now = new Date();
  const date = [
    now.getFullYear(),
    String(now.getMonth() + 1).padStart(2, "0"),
    String(now.getDate()).padStart(2, "0"),
  ].join("");
  return `${title}-${date}.csv`;
}

/**
 * Reads a query parameter from the browser URL for workbench deep links,
 * keeping the package free of host router dependencies. Rendered without a
 * browser (tests, SSR) this safely returns an empty value.
 */
export function readTradeUrlFilter(key: string): string {
  if (typeof window === "undefined") return "";
  try {
    return new URLSearchParams(window.location.search).get(key) ?? "";
  } catch {
    return "";
  }
}

/** Reads the `status` query parameter for workbench pending deep links. */
export function readTradeUrlStatusFilter(): string {
  return readTradeUrlFilter("status");
}

export function resolveTradeStatusVariant(status: string): StatusBadgeVariant {
  const normalized = status.toLowerCase();
  if (["completed", "approved", "succeeded", "success", "delivered", "paid"].includes(normalized)) return "success";
  if (["submitted", "pending", "processing", "created", "shipped", "pending_payment", "refunding"].includes(normalized)) return "warning";
  if (["rejected", "failed", "cancelled", "canceled", "withdrawn", "expired", "refunded", "retired"].includes(normalized)) return "danger";
  return "default";
}

/**
 * Resolves a localized status label.
 *
 * Lookup order: domain-specific key (`admin.trade.<domain>.status.<status>`)
 * first, then the shared status keys (`admin.trade.status.<status>`), then the
 * caller-provided label, and finally the raw status value. Unknown statuses
 * stay visible as-is instead of collapsing to a generic translation.
 */
export function resolveTradeStatusLabel(
  status: string,
  t: (key: string, fallback?: string) => string,
  domain?: string,
): string {
  const normalized = String(status ?? "").toLowerCase();
  if (!normalized) return t("admin.trade.common.noValue", "-");
  if (domain) {
    const domainLabel = t(`admin.trade.${domain}.status.${normalized}`, "");
    if (domainLabel !== "") return domainLabel;
  }
  const sharedLabel = t(`admin.trade.status.${normalized}`, "");
  return sharedLabel !== "" ? sharedLabel : status;
}

export function TradeStatusBadge({ domain, status, label }: { domain?: string; status: string; label?: string }) {
  const { t } = useTradeAdminI18n();
  const resolved = resolveTradeStatusLabel(status, t, domain);
  return (
    <StatusBadge
      label={resolved !== status ? resolved : label ?? status}
      showIcon
      status={status}
      variant={resolveTradeStatusVariant(status)}
    />
  );
}

export function TradeListPagination({
  loading,
  onFirst,
  onJump,
  onLast,
  onNext,
  onPageSizeChange,
  onPrev,
  page,
  pageSize,
  pageSizeOptions = [10, 20, 50, 100],
  totalItems,
  totalPages,
}: {
  loading: boolean;
  onFirst: () => void;
  onJump: (page: number) => void;
  onLast: () => void;
  onNext: () => void;
  onPageSizeChange: (pageSize: number) => void;
  onPrev: () => void;
  page: number;
  pageSize: number;
  pageSizeOptions?: number[];
  totalItems: number;
  totalPages: number;
}) {
  const { t } = useTradeAdminI18n();
  const [pageInput, setPageInput] = useState(String(page));
  useEffect(() => setPageInput(String(page)), [page]);
  const commitPage = () => {
    const next = Number(pageInput);
    if (Number.isInteger(next) && next >= 1 && next <= totalPages) {
      onJump(next);
    } else {
      setPageInput(String(page));
    }
  };
  const from = totalItems === 0 ? 0 : (page - 1) * pageSize + 1;
  const to = Math.min(page * pageSize, totalItems);
  return (
    <div className="flex min-w-0 flex-1 flex-nowrap items-center justify-between gap-3">
      <span className="min-w-0 flex-1 truncate text-sm tabular-nums text-[var(--sdk-color-text-secondary)]">
        {t("admin.trade.list.pageRange", "{{from}}-{{to}} of {{total}} records", { from, to, total: totalItems })}
      </span>
      <div className="flex shrink-0 flex-nowrap items-center gap-2">
        <label className="flex shrink-0 items-center gap-1.5 text-sm text-[var(--sdk-color-text-secondary)]">
          <span className="whitespace-nowrap">{t("admin.trade.list.pageSizeLabel", "Rows per page")}</span>
          <select
            aria-label={t("admin.trade.list.pageSizeLabel", "Rows per page")}
            className="h-9 w-16 rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-2 text-sm text-[var(--sdk-color-text-primary)] outline-none transition-colors focus-visible:border-[var(--sdk-color-brand-primary)]"
            value={pageSize}
            onChange={(event) => onPageSizeChange(Number(event.target.value))}
          >
            {pageSizeOptions.map((option) => (
              <option key={option} value={option}>{option}</option>
            ))}
          </select>
        </label>
        <div className="grid shrink-0 grid-cols-[2.25rem_2.25rem_minmax(4.5rem,auto)_2.25rem_2.25rem] items-center gap-1">
          <Button
            aria-label={t("admin.trade.list.firstPage", "First page")}
            className="h-9 w-9 p-0"
            disabled={page <= 1 || loading}
            size="icon"
            title={t("admin.trade.list.firstPage", "First page")}
            type="button"
            variant="outline"
            onClick={onFirst}
          >
            <ChevronsLeft aria-hidden="true" className="h-4 w-4" />
          </Button>
          <Button
            aria-label={t("admin.trade.list.prevPage", "Previous page")}
            className="h-9 w-9 p-0"
            disabled={page <= 1 || loading}
            size="icon"
            title={t("admin.trade.list.prevPage", "Previous page")}
            type="button"
            variant="outline"
            onClick={onPrev}
          >
            <ChevronLeft aria-hidden="true" className="h-4 w-4" />
          </Button>
          <input
            aria-label={t("admin.trade.list.gotoPage", "Go to page")}
            className="h-9 w-12 rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] text-center text-sm tabular-nums text-[var(--sdk-color-text-primary)] outline-none transition-colors focus-visible:border-[var(--sdk-color-brand-primary)]"
            inputMode="numeric"
            value={pageInput}
            onBlur={commitPage}
            onChange={(event) => setPageInput(event.target.value.replace(/\D/g, ""))}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                commitPage();
              }
            }}
          />
          <span aria-hidden="true" className="whitespace-nowrap text-sm text-[var(--sdk-color-text-muted)]">
            / {totalPages}
          </span>
          <Button
            aria-label={t("admin.trade.list.nextPage", "Next page")}
            className="h-9 w-9 p-0"
            disabled={page >= totalPages || loading}
            size="icon"
            title={t("admin.trade.list.nextPage", "Next page")}
            type="button"
            variant="outline"
            onClick={onNext}
          >
            <ChevronRight aria-hidden="true" className="h-4 w-4" />
          </Button>
          <Button
            aria-label={t("admin.trade.list.lastPage", "Last page")}
            className="h-9 w-9 p-0"
            disabled={page >= totalPages || loading}
            size="icon"
            title={t("admin.trade.list.lastPage", "Last page")}
            type="button"
            variant="outline"
            onClick={onLast}
          >
            <ChevronsRight aria-hidden="true" className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}

export function DetailRow({ label, children }: { label: ReactNode; children: ReactNode }) {
  return (
    <div>
      <dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{label}</dt>
      <dd className="mt-1 break-all text-sm text-[var(--sdk-color-text-primary)]">{children}</dd>
    </div>
  );
}

export interface TradeStatusOption {
  /** i18n key for the option label, resolved through the trade admin bundle. */
  labelKey: string;
  value: string;
}

/** Select styled to match the design system field tokens. */
export function TradeStatusSelect({
  ariaLabel,
  className,
  options,
  placeholder,
  value,
  onChange,
}: {
  ariaLabel: string;
  className?: string;
  options: TradeStatusOption[];
  placeholder: string;
  value: string;
  onChange: (value: string) => void;
}) {
  const { t } = useTradeAdminI18n();
  // Fill the parent by default; callers may pass an explicit width class
  // (for example `w-36` inside a horizontal filter row) to override it.
  const widthClass = className?.includes("w-") ? "" : "w-full";
  return (
    <select
      aria-label={ariaLabel}
      className={`h-9 ${widthClass} rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-3 text-sm text-[var(--sdk-color-text-primary)] outline-none transition-colors focus-visible:border-[var(--sdk-color-brand-primary)] ${className ?? ""}`}
      value={value}
      onChange={(event) => onChange(event.target.value)}
    >
      <option value="">{placeholder}</option>
      {options.map((option) => (
        <option key={option.value} value={option.value}>{t(option.labelKey, option.value)}</option>
      ))}
    </select>
  );
}
