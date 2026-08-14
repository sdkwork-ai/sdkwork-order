import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Eye, RefreshCw, RotateCcw, Search, ShieldCheck } from "lucide-react";
import {
  TradeAdminIntlProvider,
  useTradeAdminI18n,
  type TradeAdminIntlProps,
} from "../i18n/intl";
import {
  Button,
  DataTable,
  type DataTableColumn,
  Drawer,
  DrawerBody,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerHeader,
  DrawerTitle,
  FilterBar,
  FilterBarActions,
  FilterBarSection,
  Input,
  LoadingBlock,
  StatusNotice,
} from "@sdkwork/ui-pc-react";
import type { AfterSalesRequestSummary } from "@sdkwork/order-pc-admin-core";
import { getSdkworkOrderBackendSdkClient } from "@sdkwork/order-pc-admin-core";
import {
  createTradeAdminService,
  type AfterSalesReviewInput,
  type TradeAdminService,
} from "../trade-admin-service";
import {
  DetailRow,
  formatTimestamp,
  TradeListPagination,
  TradeStatusBadge,
  TradeStatusSelect,
} from "../components/trade-shared";
import { AfterSalesReviewDialog } from "../components/review-dialogs";

const DEFAULT_PAGE_SIZE = 20;

export interface SdkworkOrderAfterSalesPageProps extends TradeAdminIntlProps {
  canManage: boolean;
  service?: TradeAdminService;
}

const AFTER_SALES_STATUS_OPTIONS = [
  { label: "submitted", value: "submitted" },
  { label: "approved", value: "approved" },
  { label: "processing", value: "processing" },
  { label: "completed", value: "completed" },
  { label: "rejected", value: "rejected" },
  { label: "cancelled", value: "cancelled" },
  { label: "withdrawn", value: "withdrawn" },
];

type ReviewTarget = {
  request: AfterSalesRequestSummary;
  action: "approve" | "reject";
};

export function SdkworkOrderAfterSalesPage({
  canManage,
  service: injectedService,
  locale,
  messages,
}: SdkworkOrderAfterSalesPageProps) {
  return (
    <TradeAdminIntlProvider locale={locale} messages={messages}>
      <AfterSalesPageInner canManage={canManage} service={injectedService} />
    </TradeAdminIntlProvider>
  );
}

function AfterSalesPageInner({
  canManage,
  service: injectedService,
}: {
  canManage: boolean;
  service?: TradeAdminService;
}) {
  const { t } = useTradeAdminI18n();
  const service = useMemo(
    () => injectedService ?? createTradeAdminService(getSdkworkOrderBackendSdkClient()),
    [injectedService],
  );
  const [requests, setRequests] = useState<AfterSalesRequestSummary[]>([]);
  const [page, setPage] = useState(1);
  const [draftStatus, setDraftStatus] = useState("");
  const [draftType, setDraftType] = useState("");
  const [draftOrderId, setDraftOrderId] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [typeFilter, setTypeFilter] = useState("");
  const [orderIdFilter, setOrderIdFilter] = useState("");
  const [totalItems, setTotalItems] = useState(0);
  const [totalPages, setTotalPages] = useState(1);
  const [loading, setLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<AfterSalesRequestSummary | null>(null);
  const [reviewTarget, setReviewTarget] = useState<ReviewTarget | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setListError(null);
    void service.listAfterSales({
      page,
      pageSize: DEFAULT_PAGE_SIZE,
      status: statusFilter || undefined,
      afterSalesType: typeFilter || undefined,
      orderId: orderIdFilter || undefined,
    }).then((result) => {
      if (!active) return;
      setRequests(result.items);
      setTotalItems(result.totalItems);
      setTotalPages(Math.max(1, result.totalPages));
    }).catch(() => {
      if (!active) return;
      setListError(t("admin.trade.list.loadError", "List loading failed. Please check permissions and network."));
      setRequests([]);
      setTotalItems(0);
    }).finally(() => {
      if (active) setLoading(false);
    });
    return () => { active = false; };
  }, [orderIdFilter, page, refreshKey, service, statusFilter, t, typeFilter]);

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      setDetailError(null);
      return;
    }
    let active = true;
    setDetailLoading(true);
    setDetailError(null);
    void service.getAfterSales(selectedId)
      .then((value) => { if (active) setDetail(value); })
      .catch(() => {
        if (!active) return;
        setDetail(null);
        setDetailError(t("admin.trade.list.loadError", "List loading failed. Please check permissions and network."));
      })
      .finally(() => { if (active) setDetailLoading(false); });
    return () => { active = false; };
  }, [selectedId, service, t]);

  const applyFilters = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setPage(1);
    setStatusFilter(draftStatus.trim());
    setTypeFilter(draftType.trim());
    setOrderIdFilter(draftOrderId.trim());
    setRefreshKey((current) => current + 1);
  };

  const resetFilters = () => {
    setDraftStatus("");
    setDraftType("");
    setDraftOrderId("");
    setStatusFilter("");
    setTypeFilter("");
    setOrderIdFilter("");
    setPage(1);
    setRefreshKey((current) => current + 1);
  };

  async function submitReview(input: AfterSalesReviewInput) {
    if (!reviewTarget) return;
    const target = reviewTarget;
    setBusy(true);
    setMessage(null);
    setListError(null);
    try {
      await service.reviewAfterSales(target.request.afterSalesRequestId, input);
      setMessage(t("admin.trade.afterSales.reviewSuccess", "After-sales request {{no}} has been {{action}}.", {
        no: target.request.afterSalesNo,
        action: t(input.action === "approve" ? "admin.trade.afterSales.action.approve" : "admin.trade.afterSales.action.reject", input.action === "approve" ? "approved" : "rejected"),
      }));
      setRefreshKey((current) => current + 1);
      setReviewTarget(null);
      if (selectedId === target.request.afterSalesRequestId) {
        setSelectedId(null);
      }
    } catch {
      setListError(t("admin.trade.afterSales.reviewFailure", "Review failed. Check permissions and the current request status."));
    } finally {
      setBusy(false);
    }
  }

  const columns = useMemo<DataTableColumn<AfterSalesRequestSummary>[]>(() => [
    {
      id: "afterSalesNo",
      header: t("admin.trade.afterSales.no", "After-sales no."),
      width: "26%",
      cell: (request) => (
        <button
          className="min-w-0 text-left"
          onClick={() => setSelectedId(request.afterSalesRequestId)}
          type="button"
        >
          <span className="block truncate font-mono text-sm font-semibold text-[var(--sdk-color-text-primary)]">
            {request.afterSalesNo}
          </span>
          <span className="mt-1 block truncate font-mono text-xs text-[var(--sdk-color-text-muted)]">
            #{request.afterSalesRequestId}
          </span>
        </button>
      ),
    },
    {
      id: "order",
      header: t("admin.trade.afterSales.orderId", "Order ID"),
      width: "20%",
      cell: (request) => (
        <span className="block truncate font-mono text-xs text-[var(--sdk-color-text-secondary)]">
          {request.orderId}
        </span>
      ),
    },
    {
      id: "type",
      header: t("admin.trade.afterSales.type", "Type"),
      width: "14%",
      cell: (request) => (
        <span className="text-sm text-[var(--sdk-color-text-secondary)]">{request.afterSalesType || t("admin.trade.common.noValue", "-")}</span>
      ),
    },
    {
      id: "status",
      header: t("admin.trade.filter.status", "Status"),
      width: "16%",
      cell: (request) => <TradeStatusBadge status={request.status} />,
    },
    {
      align: "right",
      id: "amount",
      header: t("admin.trade.afterSales.requestedAmount", "Requested amount"),
      width: "20%",
      cell: (request) => (
        <span className="font-mono text-sm font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">
          {request.requestedAmount} {request.currencyCode}
        </span>
      ),
    },
  ], [t]);

  const activeFilterCount = Number(Boolean(statusFilter)) + Number(Boolean(typeFilter)) + Number(Boolean(orderIdFilter));

  return (
    <div aria-label={t("admin.trade.afterSales.title", "After-sales requests")} className="space-y-4">
      <form onSubmit={applyFilters}>
        <FilterBar
          summary={activeFilterCount ? t("admin.trade.list.appliedFilters", "{{count}} filter(s) applied", { count: activeFilterCount }) : undefined}
          title={t("admin.trade.filter.title", "Filters")}
        >
          <FilterBarSection>
            <label className="min-w-[10rem] flex-1 space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span>{t("admin.trade.filter.status", "Status")}</span>
              <TradeStatusSelect
                ariaLabel={t("admin.trade.filter.status", "Status")}
                options={AFTER_SALES_STATUS_OPTIONS}
                placeholder={t("admin.trade.filter.statusPlaceholder", "e.g. submitted")}
                value={draftStatus}
                onChange={setDraftStatus}
              />
            </label>
            <label className="min-w-[10rem] flex-1 space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span>{t("admin.trade.filter.afterSalesType", "After-sales type")}</span>
              <Input
                placeholder={t("admin.trade.filter.afterSalesTypePlaceholder", "e.g. refund")}
                value={draftType}
                onChange={(event) => setDraftType(event.target.value)}
              />
            </label>
            <label className="min-w-[14rem] flex-1 space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span>{t("admin.trade.filter.orderId", "Order ID")}</span>
              <Input
                placeholder={t("admin.trade.filter.orderIdPlaceholder", "Order ID or order number")}
                value={draftOrderId}
                onChange={(event) => setDraftOrderId(event.target.value)}
              />
            </label>
          </FilterBarSection>
          <FilterBarActions>
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
      {message ? <StatusNotice tone="success">{message}</StatusNotice> : null}

      <DataTable
        columns={columns}
        density="compact"
        description={`${t("admin.trade.afterSales.title", "After-sales requests")} — ${requests.length} / ${totalItems}`}
        emptyDescription={activeFilterCount ? t("admin.trade.list.filteredEmptyDescription", "No records match the current filters. Adjust the filters and retry.") : t("admin.trade.list.emptyDescription", "Records matching the current filters will appear here.")}
        emptyTitle={t("admin.trade.list.emptyTitle", "No records")}
        footer={<TradeListPagination loading={loading} onPrev={() => setPage((value) => value - 1)} onNext={() => setPage((value) => value + 1)} page={page} totalItems={totalItems} totalPages={totalPages} />}
        getRowId={(request) => request.afterSalesRequestId}
        loading={loading && requests.length === 0}
        loadingLabel={t("admin.trade.afterSales.title", "After-sales requests")}
        onRowClick={(request) => setSelectedId(request.afterSalesRequestId)}
        rowActions={(request) => (
          <div className="flex items-center justify-end gap-1">
            <Button aria-label={`${t("admin.trade.list.detail", "Details")} — ${request.afterSalesNo}`} disabled={busy} size="sm" title={t("admin.trade.list.detail", "Details")} type="button" variant="ghost" onClick={() => setSelectedId(request.afterSalesRequestId)}>
              <Eye aria-hidden="true" className="mr-1.5 h-4 w-4" />{t("admin.trade.list.detail", "Details")}
            </Button>
            {canManage ? (
              <Button aria-label={`${t("admin.trade.afterSales.review", "Review")} — ${request.afterSalesNo}`} disabled={busy} size="sm" title={t("admin.trade.afterSales.review", "Review")} type="button" variant="outline" onClick={() => setReviewTarget({ request, action: "approve" })}>
                <ShieldCheck aria-hidden="true" className="mr-1.5 h-4 w-4" />{t("admin.trade.afterSales.review", "Review")}
              </Button>
            ) : null}
          </div>
        )}
        rowActionsLabel={t("admin.trade.list.detail", "Details")}
        rows={requests}
        stickyHeader
        title={t("admin.trade.afterSales.title", "After-sales requests")}
        toolbar={(
          <Button aria-label={t("admin.trade.list.refresh", "Refresh")} disabled={loading} size="icon" title={t("admin.trade.list.refresh", "Refresh")} type="button" variant="outline" onClick={() => setRefreshKey((current) => current + 1)}>
            <RefreshCw aria-hidden="true" className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        )}
      />

      <Drawer open={Boolean(selectedId)} onOpenChange={(open) => { if (!open) setSelectedId(null); }}>
        <DrawerContent size="md">
          <DrawerHeader>
            <DrawerTitle>{t("admin.trade.afterSales.detailTitle", "After-sales detail")}</DrawerTitle>
            <DrawerDescription>{detail?.afterSalesNo ?? selectedId}</DrawerDescription>
          </DrawerHeader>
          <DrawerBody>
            {detailLoading ? <LoadingBlock label={t("admin.trade.afterSales.detailTitle", "After-sales detail")} /> : null}
            {detailError ? <StatusNotice tone="danger">{detailError}</StatusNotice> : null}
            {detail ? (
              <dl className="grid grid-cols-1 gap-x-6 gap-y-5 sm:grid-cols-2">
                <DetailRow label={t("admin.trade.afterSales.no", "After-sales no.")}>{detail.afterSalesNo}</DetailRow>
                <DetailRow label={t("admin.trade.filter.status", "Status")}><TradeStatusBadge status={detail.status} /></DetailRow>
                <DetailRow label={t("admin.trade.afterSales.orderId", "Order ID")}>{detail.orderId}</DetailRow>
                <DetailRow label={t("admin.trade.afterSales.type", "Type")}>{detail.afterSalesType || t("admin.trade.common.noValue", "-")}</DetailRow>
                <DetailRow label={t("admin.trade.afterSales.reasonCode", "Reason code")}>{detail.reasonCode || t("admin.trade.common.noValue", "-")}</DetailRow>
                <DetailRow label={t("admin.trade.afterSales.requestedAmount", "Requested amount")}>
                  <span className="font-mono font-semibold tabular-nums">{detail.requestedAmount} {detail.currencyCode}</span>
                </DetailRow>
              </dl>
            ) : null}
          </DrawerBody>
          <DrawerFooter>
            <Button onClick={() => setSelectedId(null)} type="button" variant="secondary">{t("admin.trade.list.close", "Close")}</Button>
          </DrawerFooter>
        </DrawerContent>
      </Drawer>

      <AfterSalesReviewDialog
        action={reviewTarget?.action ?? "approve"}
        busy={busy}
        label={reviewTarget
          ? `${t("admin.trade.afterSales.requestedAmount", "Requested amount")}: ${reviewTarget.request.requestedAmount} ${reviewTarget.request.currencyCode}`
          : ""}
        onConfirm={(input) => { if (reviewTarget) void submitReview({ ...input, action: reviewTarget.action }); }}
        onOpenChange={(open) => { if (!open) setReviewTarget(null); }}
        open={Boolean(reviewTarget)}
        requestNo={reviewTarget?.request.afterSalesNo ?? ""}
        title={reviewTarget?.action === "reject"
          ? t("admin.trade.afterSales.reviewReject", "Reject")
          : t("admin.trade.afterSales.reviewApprove", "Approve")}
      />
    </div>
  );
}
