import { useTradeAdminLink } from "../navigation";
import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Eye, RefreshCw, RotateCcw, Search, ShieldCheck , Download} from "lucide-react";
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
  LoadingBlock,
  StatusNotice,
} from "@sdkwork/ui-pc-react";
import type { AccountValueRequestResponse } from "@sdkwork/order-pc-admin-core";
import type {
  TradeAdminService,
  TradeRequestReviewInput,
  TradeReviewAction,
} from "../trade-admin-service";
import {
  DetailRow,
  exportTradeListCsv,
  tradeCsvFilename,
  readTradeUrlStatusFilter,
  formatAmount,
  formatTimestamp,
  resolveTradeStatusLabel,
  TradeListPagination,
  TradeStatusBadge,
  TradeStatusSelect,
} from "../components/trade-shared";
import { RequestReviewDialog } from "../components/review-dialogs";

const DEFAULT_PAGE_SIZE = 20;

const REQUEST_STATUS_OPTIONS = [
  { labelKey: "admin.trade.status.pending", value: "pending" },
  { labelKey: "admin.trade.status.approved", value: "approved" },
  { labelKey: "admin.trade.status.rejected", value: "rejected" },
  { labelKey: "admin.trade.status.failed", value: "failed" },
  { labelKey: "admin.trade.status.completed", value: "completed" },
];

/** i18n key prefixes per request family (refund / withdrawal). */
export interface AccountValueRequestCopy {
  actionApprove: string;
  actionReject: string;
  actionRetry: string;
  amount: string;
  createdAt: string;
  detailTitle: string;
  no: string;
  originalOrder: string;
  owner: string;
  providerReference: string;
  review: string;
  reviewFailure: string;
  reviewSuccess: string;
  reviewTitle: string;
  subject: string;
  targetAsset: string;
  title: string;
  updatedAt: string;
}

type ReviewTarget = {
  request: AccountValueRequestResponse;
  action: TradeReviewAction;
};

export interface AccountValueRequestListPageProps extends TradeAdminIntlProps {
  canManage: boolean;
  copy: AccountValueRequestCopy;
  service: Pick<
    TradeAdminService,
    "listRefundRequests" | "listWithdrawalRequests" | "reviewRefundRequest" | "reviewWithdrawalRequest"
  >;
  listKind: "refunds" | "withdrawals";
}

export function AccountValueRequestListPage({
  canManage,
  copy,
  listKind,
  service,
  locale,
  messages,
}: AccountValueRequestListPageProps) {
  return (
    <TradeAdminIntlProvider locale={locale} messages={messages}>
      <AccountValueRequestListPageInner
        canManage={canManage}
        copy={copy}
        listKind={listKind}
        service={service}
      />
    </TradeAdminIntlProvider>
  );
}

function AccountValueRequestListPageInner({
  canManage,
  copy,
  listKind,
  service,
}: Omit<AccountValueRequestListPageProps, "locale" | "messages">) {
  const { t, locale } = useTradeAdminI18n();
  const OrderLink = useTradeAdminLink();
  const [requests, setRequests] = useState<AccountValueRequestResponse[]>([]);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(DEFAULT_PAGE_SIZE);
  const [draftStatus, setDraftStatus] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [totalItems, setTotalItems] = useState(0);
  const [totalPages, setTotalPages] = useState(1);
  const [loading, setLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<AccountValueRequestResponse | null>(null);
  const [reviewTarget, setReviewTarget] = useState<ReviewTarget | null>(null);
  // One idempotency key per review intent: a double click on confirm reuses
  // the same key so the backend never executes the review twice.
  const [reviewIdempotencyKey, setReviewIdempotencyKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
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


  const listRequests = useMemo(
    () => (listKind === "refunds" ? service.listRefundRequests : service.listWithdrawalRequests),
    [listKind, service],
  );
  const reviewRequest = useMemo(
    () => (listKind === "refunds" ? service.reviewRefundRequest : service.reviewWithdrawalRequest),
    [listKind, service],
  );

  useEffect(() => {
    let active = true;
    setLoading(true);
    setListError(null);
    void listRequests({
      page,
      pageSize,
      status: statusFilter || undefined,
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
  }, [listRequests, page, pageSize, refreshKey, statusFilter, t]);

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
      tradeCsvFilename(t(copy.title, "Requests")),
      [
        t(copy.no, "Request no."),
        t(copy.subject, "Subject"),
        t(copy.targetAsset, "Target asset"),
        t(copy.amount, "Amount"),
        t("admin.trade.filter.status", "Status"),
        t(copy.createdAt, "Created at"),
      ],
      requests.map((request) => [
        request.requestNo ?? request.accountValueRequestId ?? "",
        request.subject ?? "",
        request.targetAsset ?? "",
        request.amount ?? "",
        resolveTradeStatusLabel(request.status ?? "", t),
        request.createdAt ?? "",
      ]),
    );
  };


  async function submitReview(input: TradeRequestReviewInput) {
    if (!reviewTarget) return;
    const target = reviewTarget;
    setBusy(true);
    setMessage(null);
    setListError(null);
    try {
      await reviewRequest(target.request.accountValueRequestId ?? "", target.action, input, reviewIdempotencyKey);
      setReviewIdempotencyKey("");
      const actionLabel = target.action === "approve"
        ? t(copy.actionApprove, "approved")
        : target.action === "reject"
          ? t(copy.actionReject, "rejected")
          : t(copy.actionRetry, "retried");
      setMessage(t(copy.reviewSuccess, "Request {{no}} has been {{action}}.", {
        no: target.request.requestNo ?? target.request.accountValueRequestId ?? "-",
        action: actionLabel,
      }));
      setRefreshKey((current) => current + 1);
      setReviewTarget(null);
      if (selectedId === target.request.accountValueRequestId) {
        setSelectedId(null);
      }
    } catch {
      setListError(t(copy.reviewFailure, "Review failed. Check permissions and the current request status."));
    } finally {
      setBusy(false);
    }
  }

  const columns = useMemo<DataTableColumn<AccountValueRequestResponse>[]>(() => [
    {
      id: "requestNo",
      header: t(copy.no, "Request no."),
      width: "24%",
      cell: (request) => (
        <button
          className="min-w-0 text-left"
          onClick={() => setSelectedId(request.accountValueRequestId ?? request.requestNo ?? "")}
          type="button"
        >
          <span className="block truncate font-mono text-sm font-semibold text-[var(--sdk-color-text-primary)]">
            {request.requestNo ?? request.accountValueRequestId ?? t("admin.trade.common.noValue", "-")}
          </span>
          <span className="mt-1 block truncate text-xs text-[var(--sdk-color-text-muted)]">
            {request.subject ?? t("admin.trade.common.noValue", "-")}
          </span>
        </button>
      ),
    },
    {
      id: "amount",
      align: "right",
      header: t(copy.amount, "Amount"),
      width: "18%",
      cell: (request) => (
        <span className="font-mono text-sm font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">
          {formatAmount(request.amount, locale, request.currencyCode)}
        </span>
      ),
    },
    {
      id: "targetAsset",
      header: t(copy.targetAsset, "Target asset"),
      width: "16%",
      cell: (request) => (
        <span className="text-sm text-[var(--sdk-color-text-secondary)]">{request.targetAsset ?? t("admin.trade.common.noValue", "-")}</span>
      ),
    },
    {
      id: "status",
      header: t("admin.trade.filter.status", "Status"),
      width: "16%",
      cell: (request) => <TradeStatusBadge status={request.status ?? ""} />,
    },
    {
      id: "createdAt",
      header: t(copy.createdAt, "Created at"),
      width: "26%",
      cell: (request) => (
        <time className="whitespace-nowrap text-sm text-[var(--sdk-color-text-secondary)]" dateTime={request.createdAt}>
          {formatTimestamp(request.createdAt, locale)}
        </time>
      ),
    },
  ], [copy, locale, t]);

  const activeFilterCount = Number(Boolean(statusFilter));

  return (
    <div aria-label={t(copy.title, "Requests")} className="flex min-h-0 flex-1 flex-col">
      <form onSubmit={applyFilters}>
        <FilterBar>
          <FilterBarSection wrap={false}>
            <label className="flex shrink-0 items-center gap-2 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span className="whitespace-nowrap">{t("admin.trade.filter.status", "Status")}</span>
              <TradeStatusSelect
                ariaLabel={t("admin.trade.filter.status", "Status")}
                className="w-36"
                options={REQUEST_STATUS_OPTIONS}
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
      {message ? <StatusNotice tone="success">{message}</StatusNotice> : null}

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
        getRowId={(request) => request.accountValueRequestId ?? request.requestNo ?? "row"}
        loading={loading && requests.length === 0}
        loadingLabel={t(copy.title, "Requests")}
        onRowClick={(request) => setSelectedId(request.accountValueRequestId ?? request.requestNo ?? "")}
        rowActions={(request) => (
          <div className="flex items-center justify-end gap-1">
            <Button
              aria-label={`${t("admin.trade.list.detail", "Details")} — ${request.requestNo ?? request.accountValueRequestId ?? ""}`}
              disabled={busy}
              size="sm"
              title={t("admin.trade.list.detail", "Details")}
              type="button"
              variant="ghost"
              onClick={() => setSelectedId(request.accountValueRequestId ?? request.requestNo ?? "")}
            >
              <Eye aria-hidden="true" className="mr-1.5 h-4 w-4" />{t("admin.trade.list.detail", "Details")}
            </Button>
            {canManage ? (
              <Button
                aria-label={`${t(copy.review, "Review")} — ${request.requestNo ?? request.accountValueRequestId ?? ""}`}
                disabled={busy}
                size="sm"
                title={t(copy.review, "Review")}
                type="button"
                variant="outline"
                onClick={() => {
                  setReviewTarget({ request, action: "approve" });
                  setReviewIdempotencyKey(crypto.randomUUID());
                }}
              >
                <ShieldCheck aria-hidden="true" className="mr-1.5 h-4 w-4" />{t(copy.review, "Review")}
              </Button>
            ) : null}
          </div>
        )}
        rowActionsLabel={t("admin.trade.list.detail", "Details")}
        rows={requests}
        slotProps={{
          surface: { className: "min-h-0 flex-1 flex flex-col" },
          viewport: { className: "min-h-0 flex-1" },
        }}
        stickyHeader
      />

      <Drawer open={Boolean(selectedId)} onOpenChange={(open) => { if (!open) setSelectedId(null); }}>
        <DrawerContent size="md">
          <DrawerHeader>
            <DrawerTitle>{t(copy.detailTitle, "Request detail")}</DrawerTitle>
            <DrawerDescription>{detail?.requestNo ?? selectedId}</DrawerDescription>
          </DrawerHeader>
          <DrawerBody>
            {detail ? (
              <dl className="grid grid-cols-1 gap-x-6 gap-y-5 sm:grid-cols-2">
                <DetailRow label={t(copy.no, "Request no.")}>{detail.requestNo ?? t("admin.trade.common.noValue", "-")}</DetailRow>
                <DetailRow label={t("admin.trade.filter.status", "Status")}><TradeStatusBadge status={detail.status ?? ""} /></DetailRow>
                <DetailRow label={t(copy.subject, "Subject")}>{detail.subject ?? t("admin.trade.common.noValue", "-")}</DetailRow>
                <DetailRow label={t(copy.targetAsset, "Target asset")}>{detail.targetAsset ?? t("admin.trade.common.noValue", "-")}</DetailRow>
                <DetailRow label={t(copy.amount, "Amount")}>
                  <span className="font-mono font-semibold tabular-nums">{formatAmount(detail.amount, locale, detail.currencyCode)}</span>
                </DetailRow>
                <DetailRow label={t(copy.originalOrder, "Original order")}>
                  {detail.originalOrderId ? (
                    <OrderLink href={`/admin/trade/orders?q=${detail.originalOrderId}`} className="font-mono text-[var(--sdk-color-brand-primary)] hover:underline">
                      {detail.originalOrderId}
                    </OrderLink>
                  ) : t("admin.trade.common.noValue", "-")}
                </DetailRow>
                <DetailRow label={t(copy.owner, "Owner")}>{detail.ownerUserId ?? t("admin.trade.common.noValue", "-")}</DetailRow>
                <DetailRow label={t(copy.providerReference, "Provider reference")}>{detail.providerReferenceId ?? t("admin.trade.common.noValue", "-")}</DetailRow>
                <DetailRow label={t(copy.createdAt, "Created at")}>{formatTimestamp(detail.createdAt, locale)}</DetailRow>
                <DetailRow label={t(copy.updatedAt, "Updated at")}>{formatTimestamp(detail.updatedAt, locale)}</DetailRow>
              </dl>
            ) : null}
          </DrawerBody>
          <DrawerFooter>
            <Button onClick={() => setSelectedId(null)} type="button" variant="secondary">{t("admin.trade.list.close", "Close")}</Button>
          </DrawerFooter>
        </DrawerContent>
      </Drawer>

      <RequestReviewDialog
        action={reviewTarget?.action ?? "approve"}
        busy={busy}
        label={reviewTarget?.request.subject ?? reviewTarget?.request.requestNo ?? "-"}
        onConfirm={(input) => { if (reviewTarget) void submitReview(input); }}
        onOpenChange={(open) => { if (!open) { setReviewTarget(null); setReviewIdempotencyKey(""); } }}
        open={Boolean(reviewTarget)}
        requestNo={reviewTarget?.request.requestNo ?? reviewTarget?.request.accountValueRequestId ?? ""}
        title={t(copy.reviewTitle, "Review request")}
      />
    </div>
  );
}
