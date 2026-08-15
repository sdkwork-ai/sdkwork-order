import { useEffect, useMemo, useState, type FormEvent } from "react";
import { PackagePlus, Pencil, RefreshCw, RotateCcw, Search , Download} from "lucide-react";
import {
  TradeAdminIntlProvider,
  useTradeAdminI18n,
  type TradeAdminIntlProps,
} from "../i18n/intl";
import {
  Button,
  ConfirmDialog,
  DataTable,
  type DataTableColumn,
  FilterBar,
  FilterBarActions,
  FilterBarSection,
  Input,
  Modal,
  ModalBody,
  ModalContent,
  ModalDescription,
  ModalFooter,
  ModalHeader,
  ModalTitle,
  StatusNotice,
} from "@sdkwork/ui-pc-react";
import type {
  TokenBankPlanResponse,
  TokenBankPlanWriteCommand,
} from "@sdkwork/order-pc-admin-core";
import { getSdkworkOrderBackendSdkClient } from "@sdkwork/order-pc-admin-core";
import {
  createTradeAdminService,
  type TradeAdminService,
} from "../trade-admin-service";
import {
  exportTradeListCsv,
  formatAmount,
  resolveTradeStatusLabel,
  tradeCsvFilename,
  TradeListPagination,
  TradeStatusBadge,
  TradeStatusSelect,
  readTradeUrlStatusFilter,
} from "../components/trade-shared";

const DEFAULT_PAGE_SIZE = 20;

const PLAN_PERIOD_OPTIONS = [
  { labelKey: "admin.trade.tokenBankPlans.planPeriod.monthly", value: "monthly" },
  { labelKey: "admin.trade.tokenBankPlans.planPeriod.quarterly", value: "quarterly" },
  { labelKey: "admin.trade.tokenBankPlans.planPeriod.yearly", value: "yearly" },
  { labelKey: "admin.trade.tokenBankPlans.planPeriod.continuous_monthly", value: "continuous_monthly" },
  { labelKey: "admin.trade.tokenBankPlans.planPeriod.continuous_yearly", value: "continuous_yearly" },
];

const PLAN_STATUS_OPTIONS = [
  { labelKey: "admin.trade.tokenBankPlans.status.active", value: "active" },
  { labelKey: "admin.trade.tokenBankPlans.status.retired", value: "retired" },
];

type PlanDraft = {
  planCode?: string;
  planCodeValue: string;
  displayName: string;
  planPeriod: string;
  grantAmount: string;
  bonusAmount: string;
  priceAmount: string;
  currencyCode: string;
  renewalPolicy: string;
  status: string;
};

const EMPTY_PLAN_DRAFT: PlanDraft = {
  planCode: undefined,
  planCodeValue: "",
  displayName: "",
  planPeriod: "monthly",
  grantAmount: "",
  bonusAmount: "",
  priceAmount: "",
  currencyCode: "CNY",
  renewalPolicy: "",
  status: "active",
};

function toWriteCommand(draft: PlanDraft): TokenBankPlanWriteCommand {
  return {
    planCode: draft.planCodeValue.trim(),
    displayName: draft.displayName.trim(),
    planPeriod: draft.planPeriod as TokenBankPlanWriteCommand["planPeriod"],
    grantAmount: draft.grantAmount.trim(),
    ...(draft.bonusAmount.trim() ? { bonusAmount: draft.bonusAmount.trim() } : {}),
    priceAmount: draft.priceAmount.trim(),
    currencyCode: draft.currencyCode.trim(),
    ...(draft.renewalPolicy.trim() ? { renewalPolicy: draft.renewalPolicy.trim() } : {}),
    ...(draft.status.trim() ? { status: draft.status.trim() } : {}),
  };
}

export interface SdkworkOrderTokenBankPlansPageProps extends TradeAdminIntlProps {
  canManage: boolean;
  service?: TradeAdminService;
}

export function SdkworkOrderTokenBankPlansPage({
  canManage,
  service: injectedService,
  locale,
  messages,
}: SdkworkOrderTokenBankPlansPageProps) {
  return (
    <TradeAdminIntlProvider locale={locale} messages={messages}>
      <TokenBankPlansPageInner canManage={canManage} service={injectedService} />
    </TradeAdminIntlProvider>
  );
}

function TokenBankPlansPageInner({
  canManage,
  service: injectedService,
}: {
  canManage: boolean;
  service?: TradeAdminService;
}) {
  const { t, locale } = useTradeAdminI18n();
  const service = useMemo(
    () => injectedService ?? createTradeAdminService(getSdkworkOrderBackendSdkClient()),
    [injectedService],
  );
  const [items, setItems] = useState<TokenBankPlanResponse[]>([]);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(DEFAULT_PAGE_SIZE);
  const [draftStatus, setDraftStatus] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [totalItems, setTotalItems] = useState(0);
  const [totalPages, setTotalPages] = useState(1);
  const [loading, setLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [draft, setDraft] = useState<PlanDraft>(EMPTY_PLAN_DRAFT);
  const [retireTarget, setRetireTarget] = useState<TokenBankPlanResponse | null>(null);
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


  useEffect(() => {
    let active = true;
    setLoading(true);
    setListError(null);
    void service.listTokenBankPlans({
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
      setListError(t("admin.trade.tokenBankPlans.loadFailed", "Plan list loading failed. Check permissions and network."));
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
      tradeCsvFilename(t("admin.trade.tokenBankPlans.title", "Token Bank plans")),
      [
        t("admin.trade.tokenBankPlans.planCode", "Plan code"),
        t("admin.trade.tokenBankPlans.displayName", "Name"),
        t("admin.trade.tokenBankPlans.planPeriod", "Period"),
        t("admin.trade.tokenBankPlans.grantAmount", "Grant amount"),
        t("admin.trade.tokenBankPlans.bonusAmount", "Bonus amount"),
        t("admin.trade.tokenBankPlans.priceAmount", "Price"),
        t("admin.trade.tokenBankPlans.currencyCode", "Currency"),
        t("admin.trade.filter.status", "Status"),
      ],
      items.map((item) => [
        item.planCode ?? "",
        item.displayName ?? "",
        item.planPeriod ?? "",
        item.grantAmount ?? "",
        item.bonusAmount ?? "",
        item.priceAmount ?? "",
        item.currencyCode ?? "",
        resolveTradeStatusLabel(item.status ?? "", t, "tokenBankPlans"),
      ]),
    );
  };


  const openCreate = () => {
    setDraft(EMPTY_PLAN_DRAFT);
    setDialogOpen(true);
  };

  const openEdit = (item: TokenBankPlanResponse) => {
    setDraft({
      planCode: item.planCode,
      planCodeValue: item.planCode ?? "",
      displayName: item.displayName ?? "",
      planPeriod: item.planPeriod ?? "monthly",
      grantAmount: item.grantAmount ?? "",
      bonusAmount: item.bonusAmount ?? "",
      priceAmount: item.priceAmount ?? "",
      currencyCode: item.currencyCode ?? "CNY",
      renewalPolicy: item.renewalPolicy ?? "",
      status: item.status ?? "active",
    });
    setDialogOpen(true);
  };

  async function savePlan() {
    setBusy(true);
    setMessage(null);
    setListError(null);
    try {
      const values = toWriteCommand(draft);
      if (draft.planCode) {
        await service.updateTokenBankPlan(draft.planCode, values);
      } else {
        await service.createTokenBankPlan(values);
      }
      setMessage(t("admin.trade.tokenBankPlans.saved", "Plan {{name}} has been saved.", {
        name: draft.displayName || draft.planCodeValue,
      }));
      setDialogOpen(false);
      setRefreshKey((current) => current + 1);
    } catch {
      setListError(t("admin.trade.tokenBankPlans.saveFailed", "Saving failed. Check permissions and the form contents."));
    } finally {
      setBusy(false);
    }
  }

  async function retirePlan() {
    if (!retireTarget) return;
    const target = retireTarget;
    setBusy(true);
    setMessage(null);
    setListError(null);
    try {
      await service.retireTokenBankPlan(target.planCode ?? "");
      setMessage(t("admin.trade.tokenBankPlans.retired", "Plan {{name}} has been retired.", {
        name: target.displayName ?? target.planCode ?? "-",
      }));
      setRetireTarget(null);
      setRefreshKey((current) => current + 1);
    } catch {
      setListError(t("admin.trade.tokenBankPlans.retireFailed", "Retiring failed. Check permissions and the current state."));
    } finally {
      setBusy(false);
    }
  }

  const columns = useMemo<DataTableColumn<TokenBankPlanResponse>[]>(() => [
    {
      id: "planCode",
      header: t("admin.trade.tokenBankPlans.planCode", "Plan code"),
      width: "16%",
      cell: (item) => (
        <span className="block truncate font-mono text-sm font-semibold text-[var(--sdk-color-text-primary)]">
          {item.planCode ?? t("admin.trade.common.noValue", "-")}
        </span>
      ),
    },
    {
      id: "displayName",
      header: t("admin.trade.tokenBankPlans.displayName", "Name"),
      width: "18%",
      cell: (item) => (
        <span className="block truncate text-sm text-[var(--sdk-color-text-secondary)]">
          {item.displayName ?? t("admin.trade.common.noValue", "-")}
        </span>
      ),
    },
    {
      id: "planPeriod",
      header: t("admin.trade.tokenBankPlans.planPeriod", "Period"),
      width: "14%",
      cell: (item) => {
        const period = item.planPeriod ?? "";
        const label = t(`admin.trade.tokenBankPlans.planPeriod.${period.toLowerCase()}`, "");
        return (
          <span className="text-sm text-[var(--sdk-color-text-secondary)]">{label !== "" ? label : period || t("admin.trade.common.noValue", "-")}</span>
        );
      },
    },
    {
      id: "amounts",
      header: t("admin.trade.tokenBankPlans.grantAmount", "Grant amount"),
      width: "16%",
      cell: (item) => (
        <span className="font-mono text-sm tabular-nums text-[var(--sdk-color-text-secondary)]">
          {formatAmount(item.grantAmount, locale)}
          {item.bonusAmount ? ` + ${formatAmount(item.bonusAmount, locale)}` : ""}
        </span>
      ),
    },
    {
      id: "price",
      header: t("admin.trade.tokenBankPlans.priceAmount", "Price"),
      width: "14%",
      cell: (item) => (
        <span className="font-mono text-sm font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">
          {formatAmount(item.priceAmount, locale, item.currencyCode)}
        </span>
      ),
    },
    {
      id: "status",
      header: t("admin.trade.filter.status", "Status"),
      width: "12%",
      cell: (item) => <TradeStatusBadge domain="tokenBankPlans" status={item.status ?? ""} />,
    },
  ], [locale, t]);

  const activeFilterCount = Number(Boolean(statusFilter));

  return (
    <div aria-label={t("admin.trade.tokenBankPlans.title", "Token Bank plans")} className="flex min-h-0 flex-1 flex-col">
      <form onSubmit={applyFilters}>
        <FilterBar>
          <FilterBarSection wrap={false}>
            <label className="flex shrink-0 items-center gap-2 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span className="whitespace-nowrap">{t("admin.trade.filter.status", "Status")}</span>
              <TradeStatusSelect
                ariaLabel={t("admin.trade.filter.status", "Status")}
                className="w-36"
                options={PLAN_STATUS_OPTIONS}
                placeholder={t("admin.trade.filter.statusAll", "All statuses")}
                value={draftStatus}
                onChange={setDraftStatus}
              />
            </label>
          </FilterBarSection>
          <FilterBarActions>
            {canManage ? (
              <Button aria-label={t("admin.trade.tokenBankPlans.add", "Add plan")} disabled={busy} size="sm" type="button" variant="outline" onClick={openCreate}>
                <PackagePlus aria-hidden="true" className="mr-1.5 h-4 w-4" />
                {t("admin.trade.tokenBankPlans.add", "Add plan")}
              </Button>
            ) : null}
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
        getRowId={(item) => item.planCode ?? "row"}
        loading={loading && items.length === 0}
        loadingLabel={t("admin.trade.tokenBankPlans.title", "Token Bank plans")}
        rowActions={(item) => (
          <div className="flex items-center justify-end gap-1">
            {canManage ? (
              <>
                <Button aria-label={`${t("admin.trade.tokenBankPlans.edit", "Edit plan")} — ${item.planCode ?? ""}`} disabled={busy} size="sm" title={t("admin.trade.tokenBankPlans.edit", "Edit plan")} type="button" variant="ghost" onClick={() => openEdit(item)}>
                  <Pencil aria-hidden="true" className="mr-1.5 h-4 w-4" />{t("admin.trade.tokenBankPlans.edit", "Edit plan")}
                </Button>
                {item.status !== "retired" ? (
                  <Button aria-label={`${t("admin.trade.tokenBankPlans.retire", "Retire")} — ${item.planCode ?? ""}`} disabled={busy} size="sm" title={t("admin.trade.tokenBankPlans.retire", "Retire")} type="button" variant="outline" onClick={() => setRetireTarget(item)}>
                    {t("admin.trade.tokenBankPlans.retire", "Retire")}
                  </Button>
                ) : null}
              </>
            ) : null}
          </div>
        )}
        rowActionsLabel={t("admin.trade.tokenBankPlans.edit", "Edit plan")}
        rows={items}
        slotProps={{
          surface: { className: "min-h-0 flex-1 flex flex-col" },
          viewport: { className: "min-h-0 flex-1" },
        }}
        stickyHeader
      />

      <Modal open={dialogOpen} onOpenChange={(open) => { if (!busy) setDialogOpen(open); }}>
        <ModalContent size="sm">
          <ModalHeader>
            <ModalTitle>
              {draft.planCode
                ? t("admin.trade.tokenBankPlans.edit", "Edit plan")
                : t("admin.trade.tokenBankPlans.add", "Add plan")}
            </ModalTitle>
            <ModalDescription>{draft.planCodeValue}</ModalDescription>
          </ModalHeader>
          <form onSubmit={(event) => { event.preventDefault(); void savePlan(); }}>
            <ModalBody>
              <div className="space-y-4">
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.tokenBankPlans.planCode", "Plan code")}</span>
                  <Input
                    disabled={Boolean(draft.planCode)}
                    placeholder="TB-100"
                    required
                    value={draft.planCodeValue}
                    onChange={(event) => setDraft((current) => ({ ...current, planCodeValue: event.target.value }))}
                  />
                </label>
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.tokenBankPlans.displayName", "Name")}</span>
                  <Input
                    required
                    value={draft.displayName}
                    onChange={(event) => setDraft((current) => ({ ...current, displayName: event.target.value }))}
                  />
                </label>
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.tokenBankPlans.planPeriod", "Period")}</span>
                  <TradeStatusSelect
                    ariaLabel={t("admin.trade.tokenBankPlans.planPeriod", "Period")}
                    options={PLAN_PERIOD_OPTIONS}
                    placeholder={t("admin.trade.common.selectPlaceholder", "Select")}
                    value={draft.planPeriod}
                    onChange={(planPeriod) => setDraft((current) => ({ ...current, planPeriod }))}
                  />
                </label>
                <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
                  <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                    <span>{t("admin.trade.tokenBankPlans.grantAmount", "Grant amount")}</span>
                    <Input
                      placeholder="1000.00"
                      required
                      value={draft.grantAmount}
                      onChange={(event) => setDraft((current) => ({ ...current, grantAmount: event.target.value }))}
                    />
                  </label>
                  <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                    <span>{t("admin.trade.tokenBankPlans.bonusAmount", "Bonus amount")}</span>
                    <Input
                      placeholder="0.00"
                      value={draft.bonusAmount}
                      onChange={(event) => setDraft((current) => ({ ...current, bonusAmount: event.target.value }))}
                    />
                  </label>
                  <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                    <span>{t("admin.trade.tokenBankPlans.priceAmount", "Price")}</span>
                    <Input
                      placeholder="99.00"
                      required
                      value={draft.priceAmount}
                      onChange={(event) => setDraft((current) => ({ ...current, priceAmount: event.target.value }))}
                    />
                  </label>
                </div>
                <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                  <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                    <span>{t("admin.trade.tokenBankPlans.currencyCode", "Currency")}</span>
                    <Input
                      placeholder="CNY"
                      required
                      value={draft.currencyCode}
                      onChange={(event) => setDraft((current) => ({ ...current, currencyCode: event.target.value }))}
                    />
                  </label>
                  <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                    <span>{t("admin.trade.tokenBankPlans.renewalPolicy", "Renewal policy")}</span>
                    <Input
                      placeholder="auto"
                      value={draft.renewalPolicy}
                      onChange={(event) => setDraft((current) => ({ ...current, renewalPolicy: event.target.value }))}
                    />
                  </label>
                </div>
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.filter.status", "Status")}</span>
                  <TradeStatusSelect
                    ariaLabel={t("admin.trade.filter.status", "Status")}
                    options={PLAN_STATUS_OPTIONS}
                    placeholder={t("admin.trade.common.selectPlaceholder", "Select")}
                    value={draft.status}
                    onChange={(status) => setDraft((current) => ({ ...current, status }))}
                  />
                </label>
              </div>
            </ModalBody>
            <ModalFooter>
              <Button disabled={busy} type="button" variant="secondary" onClick={() => setDialogOpen(false)}>
                {t("admin.trade.list.close", "Close")}
              </Button>
              <Button disabled={busy} loading={busy} type="submit">
                {t("admin.trade.review.confirm", "Confirm")}
              </Button>
            </ModalFooter>
          </form>
        </ModalContent>
      </Modal>

      <ConfirmDialog
        cancelLabel={t("admin.trade.list.close", "Close")}
        closeOnConfirm={false}
        confirmLabel={t("admin.trade.tokenBankPlans.retire", "Retire")}
        confirmLoading={busy}
        description={t("admin.trade.tokenBankPlans.retireConfirmDescription", "Retiring {{name}} will make it unavailable for purchase.", {
          name: retireTarget?.displayName ?? retireTarget?.planCode ?? "-",
        })}
        onConfirm={() => { void retirePlan(); }}
        onOpenChange={(open) => { if (!open && !busy) setRetireTarget(null); }}
        open={Boolean(retireTarget)}
        title={t("admin.trade.tokenBankPlans.retireConfirmTitle", "Retire plan")}
        tone="warning"
      />
    </div>
  );
}
