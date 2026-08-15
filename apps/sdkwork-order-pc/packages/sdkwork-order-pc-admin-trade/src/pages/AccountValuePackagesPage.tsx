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
  AccountValuePackageResponse,
  AccountValuePackageWriteCommand,
} from "@sdkwork/order-pc-admin-core";
import { getSdkworkOrderBackendSdkClient } from "@sdkwork/order-pc-admin-core";
import {
  createTradeAdminService,
  type TradeAdminService,
} from "../trade-admin-service";
import {
  exportTradeListCsv,
  formatAmount,
  readTradeUrlStatusFilter,
  resolveTradeStatusLabel,
  tradeCsvFilename,
  TradeListPagination,
  TradeStatusBadge,
  TradeStatusSelect,
} from "../components/trade-shared";

const DEFAULT_PAGE_SIZE = 20;

const TARGET_ASSET_OPTIONS = [
  { labelKey: "admin.trade.accountValuePackages.targetAsset.points", value: "points" },
  { labelKey: "admin.trade.accountValuePackages.targetAsset.token_bank", value: "token_bank" },
  { labelKey: "admin.trade.accountValuePackages.targetAsset.cash", value: "cash" },
];

const PACKAGE_STATUS_OPTIONS = [
  { labelKey: "admin.trade.accountValuePackages.status.active", value: "active" },
  { labelKey: "admin.trade.accountValuePackages.status.retired", value: "retired" },
];

type PackageDraft = {
  packageId?: string;
  packageCode: string;
  displayName: string;
  targetAsset: string;
  grantAmount: string;
  bonusAmount: string;
  priceAmount: string;
  currencyCode: string;
  status: string;
};

const EMPTY_PACKAGE_DRAFT: PackageDraft = {
  packageCode: "",
  displayName: "",
  targetAsset: "points",
  grantAmount: "",
  bonusAmount: "",
  priceAmount: "",
  currencyCode: "CNY",
  status: "active",
};

function toWriteCommand(draft: PackageDraft): AccountValuePackageWriteCommand {
  return {
    packageCode: draft.packageCode.trim(),
    displayName: draft.displayName.trim(),
    targetAsset: draft.targetAsset as AccountValuePackageWriteCommand["targetAsset"],
    grantAmount: draft.grantAmount.trim(),
    ...(draft.bonusAmount.trim() ? { bonusAmount: draft.bonusAmount.trim() } : {}),
    priceAmount: draft.priceAmount.trim(),
    currencyCode: draft.currencyCode.trim(),
    ...(draft.status.trim() ? { status: draft.status.trim() } : {}),
  };
}

export interface SdkworkOrderAccountValuePackagesPageProps extends TradeAdminIntlProps {
  canManage: boolean;
  service?: TradeAdminService;
}

export function SdkworkOrderAccountValuePackagesPage({
  canManage,
  service: injectedService,
  locale,
  messages,
}: SdkworkOrderAccountValuePackagesPageProps) {
  return (
    <TradeAdminIntlProvider locale={locale} messages={messages}>
      <AccountValuePackagesPageInner canManage={canManage} service={injectedService} />
    </TradeAdminIntlProvider>
  );
}

function AccountValuePackagesPageInner({
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
  const [items, setItems] = useState<AccountValuePackageResponse[]>([]);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(DEFAULT_PAGE_SIZE);
  const [draftStatus, setDraftStatus] = useState("");
  const [draftTargetAsset, setDraftTargetAsset] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [targetAssetFilter, setTargetAssetFilter] = useState("");
  const [totalItems, setTotalItems] = useState(0);
  const [totalPages, setTotalPages] = useState(1);
  const [loading, setLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [draft, setDraft] = useState<PackageDraft>(EMPTY_PACKAGE_DRAFT);
  const [retireTarget, setRetireTarget] = useState<AccountValuePackageResponse | null>(null);
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
    void service.listAccountValuePackages({
      page,
      pageSize,
      status: statusFilter || undefined,
      targetAsset: targetAssetFilter || undefined,
    }).then((result) => {
      if (!active) return;
      setItems(result.items);
      setTotalItems(result.totalItems);
      setTotalPages(Math.max(1, result.totalPages));
    }).catch(() => {
      if (!active) return;
      setListError(t("admin.trade.accountValuePackages.loadFailed", "Value package list loading failed. Check permissions and network."));
      setItems([]);
      setTotalItems(0);
    }).finally(() => {
      if (active) setLoading(false);
    });
    return () => { active = false; };
  }, [page, pageSize, refreshKey, service, statusFilter, targetAssetFilter, t]);

  const applyFilters = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setPage(1);
    setStatusFilter(draftStatus.trim());
    setTargetAssetFilter(draftTargetAsset.trim());
    setRefreshKey((current) => current + 1);
  };

  const resetFilters = () => {
    setDraftStatus("");
    setDraftTargetAsset("");
    setStatusFilter("");
    setTargetAssetFilter("");
    setPage(1);
    setRefreshKey((current) => current + 1);
  };

  const changePageSize = (nextPageSize: number) => {
    setPageSize(nextPageSize);
    setPage(1);
  };
  const handleExport = () => {
    exportTradeListCsv(
      tradeCsvFilename(t("admin.trade.accountValuePackages.title", "Value packages")),
      [
        t("admin.trade.accountValuePackages.packageCode", "Package code"),
        t("admin.trade.accountValuePackages.displayName", "Name"),
        t("admin.trade.accountValuePackages.targetAsset", "Target asset"),
        t("admin.trade.accountValuePackages.grantAmount", "Grant amount"),
        t("admin.trade.accountValuePackages.bonusAmount", "Bonus amount"),
        t("admin.trade.accountValuePackages.priceAmount", "Price"),
        t("admin.trade.accountValuePackages.currencyCode", "Currency"),
        t("admin.trade.filter.status", "Status"),
      ],
      items.map((item) => [
        item.packageCode ?? item.packageId ?? "",
        item.displayName ?? "",
        item.targetAsset ?? "",
        item.grantAmount ?? "",
        item.bonusAmount ?? "",
        item.priceAmount ?? "",
        item.currencyCode ?? "",
        resolveTradeStatusLabel(item.status ?? "", t, "accountValuePackages"),
      ]),
    );
  };


  const openCreate = () => {
    setDraft(EMPTY_PACKAGE_DRAFT);
    setDialogOpen(true);
  };

  const openEdit = (item: AccountValuePackageResponse) => {
    setDraft({
      packageId: item.packageId,
      packageCode: item.packageCode ?? "",
      displayName: item.displayName ?? "",
      targetAsset: item.targetAsset ?? "points",
      grantAmount: item.grantAmount ?? "",
      bonusAmount: item.bonusAmount ?? "",
      priceAmount: item.priceAmount ?? "",
      currencyCode: item.currencyCode ?? "CNY",
      status: item.status ?? "active",
    });
    setDialogOpen(true);
  };

  async function savePackage() {
    setBusy(true);
    setMessage(null);
    setListError(null);
    try {
      const values = toWriteCommand(draft);
      if (draft.packageId) {
        await service.updateAccountValuePackage(draft.packageId, values);
      } else {
        await service.createAccountValuePackage(values);
      }
      setMessage(t("admin.trade.accountValuePackages.saved", "Value package {{name}} has been saved.", {
        name: draft.displayName || draft.packageCode,
      }));
      setDialogOpen(false);
      setRefreshKey((current) => current + 1);
    } catch {
      setListError(t("admin.trade.accountValuePackages.saveFailed", "Saving failed. Check permissions and the form contents."));
    } finally {
      setBusy(false);
    }
  }

  async function retirePackage() {
    if (!retireTarget) return;
    const target = retireTarget;
    setBusy(true);
    setMessage(null);
    setListError(null);
    try {
      await service.retireAccountValuePackage(target.packageId ?? "");
      setMessage(t("admin.trade.accountValuePackages.retired", "Value package {{name}} has been retired.", {
        name: target.displayName ?? target.packageCode ?? target.packageId ?? "-",
      }));
      setRetireTarget(null);
      setRefreshKey((current) => current + 1);
    } catch {
      setListError(t("admin.trade.accountValuePackages.retireFailed", "Retiring failed. Check permissions and the current state."));
    } finally {
      setBusy(false);
    }
  }

  const columns = useMemo<DataTableColumn<AccountValuePackageResponse>[]>(() => [
    {
      id: "packageCode",
      header: t("admin.trade.accountValuePackages.packageCode", "Package code"),
      width: "18%",
      cell: (item) => (
        <span className="block truncate font-mono text-sm font-semibold text-[var(--sdk-color-text-primary)]">
          {item.packageCode ?? item.packageId ?? t("admin.trade.common.noValue", "-")}
        </span>
      ),
    },
    {
      id: "displayName",
      header: t("admin.trade.accountValuePackages.displayName", "Name"),
      width: "18%",
      cell: (item) => (
        <span className="block truncate text-sm text-[var(--sdk-color-text-secondary)]">
          {item.displayName ?? t("admin.trade.common.noValue", "-")}
        </span>
      ),
    },
    {
      id: "targetAsset",
      header: t("admin.trade.accountValuePackages.targetAsset", "Target asset"),
      width: "14%",
      cell: (item) => {
        const asset = item.targetAsset ?? "";
        const label = t(`admin.trade.accountValuePackages.targetAsset.${asset.toLowerCase()}`, "");
        return (
          <span className="text-sm text-[var(--sdk-color-text-secondary)]">{label !== "" ? label : asset || t("admin.trade.common.noValue", "-")}</span>
        );
      },
    },
    {
      id: "amounts",
      header: t("admin.trade.accountValuePackages.grantAmount", "Grant amount"),
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
      header: t("admin.trade.accountValuePackages.priceAmount", "Price"),
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
      cell: (item) => <TradeStatusBadge domain="accountValuePackages" status={item.status ?? ""} />,
    },
  ], [locale, t]);

  const activeFilterCount = Number(Boolean(statusFilter)) + Number(Boolean(targetAssetFilter));

  return (
    <div aria-label={t("admin.trade.accountValuePackages.title", "Value packages")} className="flex min-h-0 flex-1 flex-col">
      <form onSubmit={applyFilters}>
        <FilterBar>
          <FilterBarSection wrap={false}>
            <label className="flex shrink-0 items-center gap-2 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span className="whitespace-nowrap">{t("admin.trade.filter.status", "Status")}</span>
              <TradeStatusSelect
                ariaLabel={t("admin.trade.filter.status", "Status")}
                className="w-36"
                options={PACKAGE_STATUS_OPTIONS}
                placeholder={t("admin.trade.filter.statusAll", "All statuses")}
                value={draftStatus}
                onChange={setDraftStatus}
              />
            </label>
            <label className="flex shrink-0 items-center gap-2 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span className="whitespace-nowrap">{t("admin.trade.accountValuePackages.targetAsset", "Target asset")}</span>
              <TradeStatusSelect
                ariaLabel={t("admin.trade.accountValuePackages.targetAsset", "Target asset")}
                className="w-36"
                options={TARGET_ASSET_OPTIONS}
                placeholder={t("admin.trade.accountValuePackages.targetAssetPlaceholder", "All assets")}
                value={draftTargetAsset}
                onChange={setDraftTargetAsset}
              />
            </label>
          </FilterBarSection>
          <FilterBarActions>
            {canManage ? (
              <Button aria-label={t("admin.trade.accountValuePackages.add", "Add package")} disabled={busy} size="sm" type="button" variant="outline" onClick={openCreate}>
                <PackagePlus aria-hidden="true" className="mr-1.5 h-4 w-4" />
                {t("admin.trade.accountValuePackages.add", "Add package")}
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
        getRowId={(item) => item.packageId ?? item.packageCode ?? "row"}
        loading={loading && items.length === 0}
        loadingLabel={t("admin.trade.accountValuePackages.title", "Value packages")}
        rowActions={(item) => (
          <div className="flex items-center justify-end gap-1">
            {canManage ? (
              <>
                <Button aria-label={`${t("admin.trade.accountValuePackages.edit", "Edit package")} — ${item.packageCode ?? item.packageId ?? ""}`} disabled={busy} size="sm" title={t("admin.trade.accountValuePackages.edit", "Edit package")} type="button" variant="ghost" onClick={() => openEdit(item)}>
                  <Pencil aria-hidden="true" className="mr-1.5 h-4 w-4" />{t("admin.trade.accountValuePackages.edit", "Edit package")}
                </Button>
                {item.status !== "retired" ? (
                  <Button aria-label={`${t("admin.trade.accountValuePackages.retire", "Retire")} — ${item.packageCode ?? item.packageId ?? ""}`} disabled={busy} size="sm" title={t("admin.trade.accountValuePackages.retire", "Retire")} type="button" variant="outline" onClick={() => setRetireTarget(item)}>
                    {t("admin.trade.accountValuePackages.retire", "Retire")}
                  </Button>
                ) : null}
              </>
            ) : null}
          </div>
        )}
        rowActionsLabel={t("admin.trade.accountValuePackages.edit", "Edit package")}
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
              {draft.packageId
                ? t("admin.trade.accountValuePackages.edit", "Edit package")
                : t("admin.trade.accountValuePackages.add", "Add package")}
            </ModalTitle>
            <ModalDescription>{draft.packageCode || draft.packageId}</ModalDescription>
          </ModalHeader>
          <form onSubmit={(event) => { event.preventDefault(); void savePackage(); }}>
            <ModalBody>
              <div className="space-y-4">
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.accountValuePackages.packageCode", "Package code")}</span>
                  <Input
                    placeholder="PKG-100"
                    required
                    value={draft.packageCode}
                    onChange={(event) => setDraft((current) => ({ ...current, packageCode: event.target.value }))}
                  />
                </label>
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.accountValuePackages.displayName", "Name")}</span>
                  <Input
                    required
                    value={draft.displayName}
                    onChange={(event) => setDraft((current) => ({ ...current, displayName: event.target.value }))}
                  />
                </label>
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.accountValuePackages.targetAsset", "Target asset")}</span>
                  <TradeStatusSelect
                    ariaLabel={t("admin.trade.accountValuePackages.targetAsset", "Target asset")}
                    options={TARGET_ASSET_OPTIONS}
                    placeholder={t("admin.trade.common.selectPlaceholder", "Select")}
                    value={draft.targetAsset}
                    onChange={(targetAsset) => setDraft((current) => ({ ...current, targetAsset }))}
                  />
                </label>
                <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
                  <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                    <span>{t("admin.trade.accountValuePackages.grantAmount", "Grant amount")}</span>
                    <Input
                      placeholder="100.00"
                      required
                      value={draft.grantAmount}
                      onChange={(event) => setDraft((current) => ({ ...current, grantAmount: event.target.value }))}
                    />
                  </label>
                  <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                    <span>{t("admin.trade.accountValuePackages.bonusAmount", "Bonus amount")}</span>
                    <Input
                      placeholder="0.00"
                      value={draft.bonusAmount}
                      onChange={(event) => setDraft((current) => ({ ...current, bonusAmount: event.target.value }))}
                    />
                  </label>
                  <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                    <span>{t("admin.trade.accountValuePackages.priceAmount", "Price")}</span>
                    <Input
                      placeholder="99.00"
                      required
                      value={draft.priceAmount}
                      onChange={(event) => setDraft((current) => ({ ...current, priceAmount: event.target.value }))}
                    />
                  </label>
                </div>
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.accountValuePackages.currencyCode", "Currency")}</span>
                  <Input
                    placeholder="CNY"
                    required
                    value={draft.currencyCode}
                    onChange={(event) => setDraft((current) => ({ ...current, currencyCode: event.target.value }))}
                  />
                </label>
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.filter.status", "Status")}</span>
                  <TradeStatusSelect
                    ariaLabel={t("admin.trade.filter.status", "Status")}
                    options={PACKAGE_STATUS_OPTIONS}
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
        confirmLabel={t("admin.trade.accountValuePackages.retire", "Retire")}
        confirmLoading={busy}
        description={t("admin.trade.accountValuePackages.retireConfirmDescription", "Retiring {{name}} will make it unavailable for purchase.", {
          name: retireTarget?.displayName ?? retireTarget?.packageCode ?? retireTarget?.packageId ?? "-",
        })}
        onConfirm={() => { void retirePackage(); }}
        onOpenChange={(open) => { if (!open && !busy) setRetireTarget(null); }}
        open={Boolean(retireTarget)}
        title={t("admin.trade.accountValuePackages.retireConfirmTitle", "Retire value package")}
        tone="warning"
      />
    </div>
  );
}
