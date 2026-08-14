import type { ReactNode } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import {
  Button,
  StatusBadge,
  type StatusBadgeVariant,
} from "@sdkwork/ui-pc-react";
import { useTradeAdminI18n } from "../i18n/intl";

export function formatTimestamp(value?: string): string {
  if (!value) return "--";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN");
}

export function resolveTradeStatusVariant(status: string): StatusBadgeVariant {
  const normalized = status.toLowerCase();
  if (["completed", "approved", "succeeded", "success", "delivered"].includes(normalized)) return "success";
  if (["submitted", "pending", "processing", "created", "shipped"].includes(normalized)) return "warning";
  if (["rejected", "failed", "cancelled", "canceled", "withdrawn", "expired"].includes(normalized)) return "danger";
  return "default";
}

export function TradeStatusBadge({ status, label }: { status: string; label?: string }) {
  return (
    <StatusBadge
      label={label ?? status}
      showIcon
      status={status}
      variant={resolveTradeStatusVariant(status)}
    />
  );
}

export function TradeListPagination({
  loading,
  onPrev,
  onNext,
  page,
  totalItems,
  totalPages,
}: {
  loading: boolean;
  onPrev: () => void;
  onNext: () => void;
  page: number;
  totalItems: number;
  totalPages: number;
}) {
  const { t } = useTradeAdminI18n();
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
      <span className="min-w-0 truncate text-sm tabular-nums text-[var(--sdk-color-text-secondary)]">
        {t("admin.trade.list.totalRecords", "{{count}} records in total", { count: totalItems })}
      </span>
      <div className="grid w-full grid-cols-[2.25rem_minmax(4.5rem,1fr)_2.25rem] items-center gap-1 sm:w-auto sm:grid-cols-[2.25rem_minmax(4.5rem,auto)_2.25rem]">
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
        <span
          aria-label={t("admin.trade.list.pageInfo", "Page {{page}} of {{totalPages}}", { page, totalPages })}
          className="flex h-9 items-center justify-center gap-2 rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-3 text-sm tabular-nums text-[var(--sdk-color-text-muted)]"
        >
          <strong className="font-semibold text-[var(--sdk-color-text-primary)]">{page}</strong>
          <span aria-hidden="true">/</span>
          <span aria-hidden="true">{totalPages}</span>
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
  options: { label: string; value: string }[];
  placeholder: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <select
      aria-label={ariaLabel}
      className={`h-9 w-full rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-3 text-sm text-[var(--sdk-color-text-primary)] outline-none transition-colors focus-visible:border-[var(--sdk-color-brand-primary)] ${className ?? ""}`}
      value={value}
      onChange={(event) => onChange(event.target.value)}
    >
      <option value="">{placeholder}</option>
      {options.map((option) => (
        <option key={option.value} value={option.value}>{option.label}</option>
      ))}
    </select>
  );
}
