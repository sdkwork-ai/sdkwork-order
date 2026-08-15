import { useEffect, useMemo, useState, type FormEvent } from "react";
import { RefreshCw, RotateCcw, Search , Download} from "lucide-react";
import {
  TradeAdminIntlProvider,
  useTradeAdminI18n,
  type TradeAdminIntlProps,
} from "../i18n/intl";
import {
  Button,
  DataTable,
  type DataTableColumn,
  FilterBar,
  FilterBarActions,
  FilterBarSection,
  StatusNotice,
} from "@sdkwork/ui-pc-react";
import type { OrderCancellation } from "@sdkwork/order-pc-admin-core";
import { getSdkworkOrderBackendSdkClient } from "@sdkwork/order-pc-admin-core";
import {
  createTradeAdminService,
  type TradeAdminService,
} from "../trade-admin-service";
import {
  exportTradeListCsv,
  formatTimestamp,
  resolveTradeStatusLabel,
  tradeCsvFilename,
  TradeListPagination,
  TradeStatusBadge,
  TradeStatusSelect,
  readTradeUrlStatusFilter,
} from "../components/trade-shared";

const DEFAULT_PAGE_SIZE = 20;

const CANCELLATION_STATUS_OPTIONS = [
  { labelKey: "admin.trade.cancellations.status.pending", value: "pending" },
  { labelKey: "admin.trade.cancellations.status.approved", value: "approved" },
  { labelKey: "admin.trade.cancellations.status.rejected", value: "rejected" },
  { labelKey: "admin.trade.cancellations.status.cancelled", value: "cancelled" },
];

export interface SdkworkOrderCancellationsPageProps extends TradeAdminIntlProps {
  service?: TradeAdminService;
}

export function SdkworkOrderCancellationsPage({
  service: injectedService,
  locale,
  messages,
}: SdkworkOrderCancellationsPageProps) {
  return (
    <TradeAdminIntlProvider locale={locale} messages={messages}>
      <CancellationsPageInner service={injectedService} />
    </TradeAdminIntlProvider>
  );
}

function CancellationsPageInner({ service: injectedService }: { service?: TradeAdminService }) {
  const { t, locale } = useTradeAdminI18n();
  const service = useMemo(
    () => injectedService ?? createTradeAdminService(getSdkworkOrderBackendSdkClient()),
    [injectedService],
  );
  const [items, setItems] = useState<OrderCancellation[]>([]);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(DEFAULT_PAGE_SIZE);
  const [draftStatus, setDraftStatus] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [totalItems, setTotalItems] = useState(0);
  const [totalPages, setTotalPages] = useState(1);
  const [loading, setLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const urlStatus = readTradeUrlStatusFilter();
  useEffect(() => {
    if (urlStatus) {
      setStatusFilter(urlStatus);
      setDraftStatus(urlStatus);
    }
    // Apply deep links only when the URL status changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [urlStatus]);


  useEffect(() => {
    let active = true;
    setLoading(true);
    setListError(null);
    void service.listCancellations({
      page,
      pageSize,
      status: statusFilter || undefined,
    }).then((result) => {
      if (!active) return;
      setItems(result.items);
      setTotalItems(result.totalItems);
      setTotalPages(Math.max(1, result.totalPages));
    }).catch(() => {
      if (!active) return;
      setListError(t("admin.trade.list.loadError", "List loading failed. Please check permissions and network."));
      setItems([]);
      setTotalItems(0);
    }).finally(() => {
      if (active) setLoading(false);
    });
    return () => { active = false; };
  }, [page, pageSize, refreshKey, service, statusFilter, t]);

  const applyFilters = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setPage(1);
    setStatusFilter(draftStatus.trim());
    setRefreshKey((current) => current + 1);
  };

  const resetFilters = () => {
    setDraftStatus("");
    setStatusFilter("");
    setPage(1);
    setRefreshKey((current) => current + 1);
  };

  const changePageSize = (nextPageSize: number) => {
    setPageSize(nextPageSize);
    setPage(1);
  };
  const handleExport = () => {
    exportTradeListCsv(
      tradeCsvFilename(t("admin.trade.cancellations.title", "Cancellations")),
      [
        t("admin.trade.cancellations.no", "Cancellation no."),
        t("admin.trade.cancellations.orderId", "Order ID"),
        t("admin.trade.cancellations.reasonCode", "Reason code"),
        t("admin.trade.cancellations.reasonMessage", "Reason message"),
        t("admin.trade.filter.status", "Status"),
        t("admin.trade.cancellations.createdAt", "Created at"),
      ],
      items.map((item) => [
        item.id,
        item.orderId,
        item.reasonCode,
        item.reasonMessage ?? "",
        resolveTradeStatusLabel(item.status, t, "cancellations"),
        item.createdAt,
      ]),
    );
  };


  const columns = useMemo<DataTableColumn<OrderCancellation>[]>(() => [
    {
      id: "id",
      header: t("admin.trade.cancellations.no", "Cancellation no."),
      width: "20%",
      cell: (item) => (
        <span className="block truncate font-mono text-sm font-semibold text-[var(--sdk-color-text-primary)]">
          {item.id}
        </span>
      ),
    },
    {
      id: "orderId",
      header: t("admin.trade.cancellations.orderId", "Order ID"),
      width: "20%",
      cell: (item) => (
        <span className="block truncate font-mono text-xs text-[var(--sdk-color-text-secondary)]">
          {item.orderId}
        </span>
      ),
    },
    {
      id: "reasonCode",
      header: t("admin.trade.cancellations.reasonCode", "Reason code"),
      width: "16%",
      cell: (item) => (
        <span className="text-sm text-[var(--sdk-color-text-secondary)]">{item.reasonCode || t("admin.trade.common.noValue", "-")}</span>
      ),
    },
    {
      id: "reasonMessage",
      header: t("admin.trade.cancellations.reasonMessage", "Reason message"),
      width: "26%",
      cell: (item) => (
        <span className="block truncate text-sm text-[var(--sdk-color-text-secondary)]">
          {item.reasonMessage || t("admin.trade.common.noValue", "-")}
        </span>
      ),
    },
    {
      id: "status",
      header: t("admin.trade.filter.status", "Status"),
      width: "14%",
      cell: (item) => <TradeStatusBadge domain="cancellations" status={item.status} />,
    },
    {
      id: "createdAt",
      header: t("admin.trade.cancellations.createdAt", "Created at"),
      width: "20%",
      cell: (item) => (
        <time className="whitespace-nowrap text-sm text-[var(--sdk-color-text-secondary)]" dateTime={item.createdAt}>
          {formatTimestamp(item.createdAt, locale)}
        </time>
      ),
    },
  ], [locale, t]);

  const activeFilterCount = Number(Boolean(statusFilter));

  return (
    <div aria-label={t("admin.trade.cancellations.title", "Cancellations")} className="flex min-h-0 flex-1 flex-col">
      <form onSubmit={applyFilters}>
        <FilterBar>
          <FilterBarSection wrap={false}>
            <label className="flex shrink-0 items-center gap-2 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span className="whitespace-nowrap">{t("admin.trade.filter.status", "Status")}</span>
              <TradeStatusSelect
                ariaLabel={t("admin.trade.filter.status", "Status")}
                className="w-36"
                options={CANCELLATION_STATUS_OPTIONS}
                placeholder={t("admin.trade.filter.statusAll", "All statuses")}
                value={draftStatus}
                onChange={setDraftStatus}
              />
            </label>
          </FilterBarSection>
          <FilterBarActions>
            <Button aria-label={t("admin.trade.list.export", "Export")} disabled={loading} size="icon" title={t("admin.trade.list.export", "Export")} type="button" variant="outline" onClick={handleExport}>
              <Download aria-hidden="true" className="h-4 w-4" />
            </Button>
            <Button aria-label={t("admin.trade.list.refresh", "Refresh")} disabled={loading} size="icon" title={t("admin.trade.list.refresh", "Refresh")} type="button" variant="outline" onClick={() => setRefreshKey((current) => current + 1)}>
              <RefreshCw aria-hidden="true" className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
            </Button>
            <Button disabled={loading} type="button" variant="outline" onClick={resetFilters}>
              <RotateCcw aria-hidden="true" className="mr-2 h-4 w-4" />
              {t("admin.trade.list.reset", "Reset")}
            </Button>
            <Button disabled={loading} type="submit">
              <Search aria-hidden="true" className="mr-2 h-4 w-4" />
              {t("admin.trade.list.search", "Search")}
            </Button>
          </FilterBarActions>
        </FilterBar>
      </form>

      {listError ? <StatusNotice tone="danger">{listError}</StatusNotice> : null}

      <DataTable
        className="min-h-0 flex-1"
        columns={columns}
        density="compact"
        emptyDescription={activeFilterCount ? t("admin.trade.list.filteredEmptyDescription", "No records match the current filters. Adjust the filters and retry.") : t("admin.trade.list.emptyDescription", "Records matching the current filters will appear here.")}
        emptyTitle={t("admin.trade.list.emptyTitle", "No records")}
        footer={<TradeListPagination
          loading={loading}
          onFirst={() => setPage(1)}
          onJump={(next) => setPage(next)}
          onLast={() => setPage(totalPages)}
          onNext={() => setPage((value) => value + 1)}
          onPageSizeChange={changePageSize}
          onPrev={() => setPage((value) => value - 1)}
          page={page}
          pageSize={pageSize}
          totalItems={totalItems}
          totalPages={totalPages}
        />}
        getRowId={(item) => item.id}
        loading={loading && items.length === 0}
        loadingLabel={t("admin.trade.cancellations.title", "Cancellations")}
        rows={items}
        slotProps={{
          surface: { className: "min-h-0 flex-1 flex flex-col" },
          viewport: { className: "min-h-0 flex-1" },
        }}
        stickyHeader
      />
    </div>
  );
}
