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
  formatTimestamp,
  TradeListPagination,
  TradeStatusBadge,
  TradeStatusSelect,
} from "../components/trade-shared";
import { RequestReviewDialog } from "../components/review-dialogs";

const DEFAULT_PAGE_SIZE = 20;

const REQUEST_STATUS_OPTIONS = [
  { label: "pending", value: "pending" },
  { label: "approved", value: "approved" },
  { label: "rejected", value: "rejected" },
  { label: "failed", value: "failed" },
  { label: "completed", value: "completed" },
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
  const { t } = useTradeAdminI18n();
  const [requests, setRequests] = useState<AccountValueRequestResponse[]>([]);
  const [page, setPage] = useState(1);
  const [draftStatus, setDraftStatus] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [totalItems, setTotalItems] = useState(0);
  const [totalPages, setTotalPages] = useState(1);
  const [loading, setLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<AccountValueRequestResponse | null>(null);
  const [reviewTarget, setReviewTarget] = useState<ReviewTarget | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

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
      pageSize: DEFAULT_PAGE_SIZE,
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
  }, [listRequests, page, refreshKey, statusFilter, t]);

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

  async function submitReview(input: TradeRequestReviewInput) {
    if (!reviewTarget) return;
    const target = reviewTarget;
    setBusy(true);
    setMessage(null);
    setListError(null);
    try {
      await reviewRequest(target.request.accountValueRequestId ?? "", target.action, input);
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
          {request.amount ?? t("admin.trade.common.noValue", "-")} {request.currencyCode ?? ""}
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
          {formatTimestamp(request.createdAt)}
        </time>
      ),
    },
  ], [copy, t]);

  const activeFilterCount = Number(Boolean(statusFilter));

  return (
    <div aria-label={t(copy.title, "Requests")} className="space-y-4">
      <form onSubmit={applyFilters}>
        <FilterBar
          summary={activeFilterCount ? t("admin.trade.list.appliedFilters", "{{count}} filter(s) applied", { count: activeFilterCount }) : undefined}
          title={t("admin.trade.filter.title", "Filters")}
        >
          <FilterBarSection>
            <label className="min-w-[12rem] flex-1 space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span>{t("admin.trade.filter.status", "Status")}</span>
              <TradeStatusSelect
                ariaLabel={t("admin.trade.filter.status", "Status")}
                options={REQUEST_STATUS_OPTIONS}
                placeholder={t("admin.trade.filter.statusPlaceholder", "e.g. submitted")}
                value={draftStatus}
                onChange={setDraftStatus}
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
        description={`${t(copy.title, "Requests")} — ${requests.length} / ${totalItems}`}
        emptyDescription={activeFilterCount ? t("admin.trade.list.filteredEmptyDescription", "No records match the current filters. Adjust the filters and retry.") : t("admin.trade.list.emptyDescription", "Records matching the current filters will appear here.")}
        emptyTitle={t("admin.trade.list.emptyTitle", "No records")}
        footer={<TradeListPagination loading={loading} onPrev={() => setPage((value) => value - 1)} onNext={() => setPage((value) => value + 1)} page={page} totalItems={totalItems} totalPages={totalPages} />}
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
                onClick={() => setReviewTarget({ request, action: "approve" })}
              >
                <ShieldCheck aria-hidden="true" className="mr-1.5 h-4 w-4" />{t(copy.review, "Review")}
              </Button>
            ) : null}
          </div>
        )}
        rowActionsLabel={t("admin.trade.list.detail", "Details")}
        rows={requests}
        stickyHeader
        title={t(copy.title, "Requests")}
        toolbar={(
          <Button aria-label={t("admin.trade.list.refresh", "Refresh")} disabled={loading} size="icon" title={t("admin.trade.list.refresh", "Refresh")} type="button" variant="outline" onClick={() => setRefreshKey((current) => current + 1)}>
            <RefreshCw aria-hidden="true" className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        )}
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
                  <span className="font-mono font-semibold tabular-nums">{detail.amount ?? t("admin.trade.common.noValue", "-")} {detail.currencyCode ?? ""}</span>
                </DetailRow>
                <DetailRow label={t(copy.originalOrder, "Original order")}>{detail.originalOrderId ?? t("admin.trade.common.noValue", "-")}</DetailRow>
                <DetailRow label={t(copy.owner, "Owner")}>{detail.ownerUserId ?? t("admin.trade.common.noValue", "-")}</DetailRow>
                <DetailRow label={t(copy.providerReference, "Provider reference")}>{detail.providerReferenceId ?? t("admin.trade.common.noValue", "-")}</DetailRow>
                <DetailRow label={t(copy.createdAt, "Created at")}>{formatTimestamp(detail.createdAt)}</DetailRow>
                <DetailRow label={t(copy.updatedAt, "Updated at")}>{formatTimestamp(detail.updatedAt)}</DetailRow>
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
        onOpenChange={(open) => { if (!open) setReviewTarget(null); }}
        open={Boolean(reviewTarget)}
        requestNo={reviewTarget?.request.requestNo ?? reviewTarget?.request.accountValueRequestId ?? ""}
        title={t(copy.reviewTitle, "Review request")}
      />
    </div>
  );
}
