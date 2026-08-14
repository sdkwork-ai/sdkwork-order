import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Eye, PackagePlus, Pencil, RefreshCw, RotateCcw, Search } from "lucide-react";
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
  Modal,
  ModalBody,
  ModalContent,
  ModalDescription,
  ModalFooter,
  ModalHeader,
  ModalTitle,
} from "@sdkwork/ui-pc-react";
import type {
  CreateShipmentPackageRequest,
  ShipmentPackageSummary,
  ShipmentSummary,
  UpdateShipmentPackageRequest,
} from "@sdkwork/order-pc-admin-core";
import { getSdkworkOrderBackendSdkClient } from "@sdkwork/order-pc-admin-core";
import {
  createTradeAdminService,
  type TradeAdminService,
} from "../trade-admin-service";
import {
  DetailRow,
  formatTimestamp,
  TradeListPagination,
  TradeStatusBadge,
  TradeStatusSelect,
} from "../components/trade-shared";

const DEFAULT_PAGE_SIZE = 20;

export interface SdkworkOrderShipmentsPageProps extends TradeAdminIntlProps {
  canManage: boolean;
  service?: TradeAdminService;
}

const SHIPMENT_STATUS_OPTIONS = [
  { label: "created", value: "created" },
  { label: "shipped", value: "shipped" },
  { label: "delivered", value: "delivered" },
];

const PACKAGE_STATUS_OPTIONS = [
  { label: "created", value: "created" },
  { label: "shipped", value: "shipped" },
  { label: "delivered", value: "delivered" },
];

type PackageDraft = {
  packageId?: string;
  packageNo: string;
  packageType: string;
  trackingNo: string;
  status: string;
};

const EMPTY_PACKAGE_DRAFT: PackageDraft = {
  packageNo: "",
  packageType: "",
  trackingNo: "",
  status: "created",
};

function toCreateCommand(draft: PackageDraft): CreateShipmentPackageRequest {
  return {
    packageType: draft.packageType.trim() || "standard",
    ...(draft.packageNo.trim() ? { packageNo: draft.packageNo.trim() } : {}),
    ...(draft.trackingNo.trim() ? { trackingNo: draft.trackingNo.trim() } : {}),
    ...(draft.status.trim() ? { status: draft.status.trim() } : {}),
  };
}

function toUpdateCommand(draft: PackageDraft): UpdateShipmentPackageRequest {
  const command: UpdateShipmentPackageRequest = {};
  if (draft.packageType.trim()) command.packageType = draft.packageType.trim();
  if (draft.trackingNo.trim()) command.trackingNo = draft.trackingNo.trim();
  if (draft.status.trim()) command.status = draft.status.trim();
  return command;
}

export function SdkworkOrderShipmentsPage({
  canManage,
  service: injectedService,
  locale,
  messages,
}: SdkworkOrderShipmentsPageProps) {
  return (
    <TradeAdminIntlProvider locale={locale} messages={messages}>
      <ShipmentsPageInner canManage={canManage} service={injectedService} />
    </TradeAdminIntlProvider>
  );
}

function ShipmentsPageInner({
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
  const [shipments, setShipments] = useState<ShipmentSummary[]>([]);
  const [page, setPage] = useState(1);
  const [draftStatus, setDraftStatus] = useState("");
  const [draftOrderId, setDraftOrderId] = useState("");
  const [draftFulfillmentId, setDraftFulfillmentId] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [orderIdFilter, setOrderIdFilter] = useState("");
  const [fulfillmentIdFilter, setFulfillmentIdFilter] = useState("");
  const [totalItems, setTotalItems] = useState(0);
  const [totalPages, setTotalPages] = useState(1);
  const [loading, setLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<ShipmentSummary | null>(null);
  const [packages, setPackages] = useState<ShipmentPackageSummary[]>([]);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [packageDialogOpen, setPackageDialogOpen] = useState(false);
  const [packageDraft, setPackageDraft] = useState<PackageDraft>(EMPTY_PACKAGE_DRAFT);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setListError(null);
    void service.listShipments({
      page,
      pageSize: DEFAULT_PAGE_SIZE,
      status: statusFilter || undefined,
      orderId: orderIdFilter || undefined,
      fulfillmentId: fulfillmentIdFilter || undefined,
    }).then((result) => {
      if (!active) return;
      setShipments(result.items);
      setTotalItems(result.totalItems);
      setTotalPages(Math.max(1, result.totalPages));
    }).catch(() => {
      if (!active) return;
      setListError(t("admin.trade.list.loadError", "List loading failed. Please check permissions and network."));
      setShipments([]);
      setTotalItems(0);
    }).finally(() => {
      if (active) setLoading(false);
    });
    return () => { active = false; };
  }, [fulfillmentIdFilter, orderIdFilter, page, refreshKey, service, statusFilter, t]);

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      setPackages([]);
      setDetailError(null);
      return;
    }
    let active = true;
    setDetailLoading(true);
    setDetailError(null);
    void Promise.all([
      service.getShipment(selectedId),
      service.listShipmentPackages(selectedId),
    ]).then(([shipment, packagePage]) => {
      if (!active) return;
      setDetail(shipment);
      setPackages(packagePage.items);
    }).catch(() => {
      if (!active) return;
      setDetail(null);
      setPackages([]);
      setDetailError(t("admin.trade.list.loadError", "List loading failed. Please check permissions and network."));
    }).finally(() => {
      if (active) setDetailLoading(false);
    });
    return () => { active = false; };
  }, [selectedId, service, t]);

  const applyFilters = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setPage(1);
    setStatusFilter(draftStatus.trim());
    setOrderIdFilter(draftOrderId.trim());
    setFulfillmentIdFilter(draftFulfillmentId.trim());
    setRefreshKey((current) => current + 1);
  };

  const resetFilters = () => {
    setDraftStatus("");
    setDraftOrderId("");
    setDraftFulfillmentId("");
    setStatusFilter("");
    setOrderIdFilter("");
    setFulfillmentIdFilter("");
    setPage(1);
    setRefreshKey((current) => current + 1);
  };

  const openCreatePackage = () => {
    setPackageDraft(EMPTY_PACKAGE_DRAFT);
    setPackageDialogOpen(true);
  };

  const openEditPackage = (item: ShipmentPackageSummary) => {
    setPackageDraft({
      packageId: item.packageId,
      packageNo: item.packageNo,
      packageType: item.packageType,
      trackingNo: item.trackingNo ?? "",
      status: item.status,
    });
    setPackageDialogOpen(true);
  };

  async function savePackage() {
    if (!detail) return;
    const targetShipmentId = detail.shipmentId;
    setBusy(true);
    setMessage(null);
    setListError(null);
    try {
      if (packageDraft.packageId) {
        await service.updateShipmentPackage(
          targetShipmentId,
          packageDraft.packageId,
          toUpdateCommand(packageDraft),
        );
      } else {
        await service.createShipmentPackage(targetShipmentId, toCreateCommand(packageDraft));
      }
      setMessage(t("admin.trade.shipments.packageSaved", "Package {{no}} has been saved.", {
        no: packageDraft.packageNo || packageDraft.packageId || t("admin.trade.common.noValue", "-"),
      }));
      setPackageDialogOpen(false);
      const packagePage = await service.listShipmentPackages(targetShipmentId);
      setPackages(packagePage.items);
    } catch {
      setListError(t("admin.trade.shipments.packageSaveFailure", "Saving the package failed. Check permissions and shipment state."));
    } finally {
      setBusy(false);
    }
  }

  const columns = useMemo<DataTableColumn<ShipmentSummary>[]>(() => [
    {
      id: "shipmentNo",
      header: t("admin.trade.shipments.no", "Shipment no."),
      width: "28%",
      cell: (shipment) => (
        <button
          className="min-w-0 text-left"
          onClick={() => setSelectedId(shipment.shipmentId)}
          type="button"
        >
          <span className="block truncate font-mono text-sm font-semibold text-[var(--sdk-color-text-primary)]">
            {shipment.shipmentNo}
          </span>
          <span className="mt-1 block truncate font-mono text-xs text-[var(--sdk-color-text-muted)]">
            #{shipment.shipmentId}
          </span>
        </button>
      ),
    },
    {
      id: "status",
      header: t("admin.trade.filter.status", "Status"),
      width: "16%",
      cell: (shipment) => <TradeStatusBadge status={shipment.status} />,
    },
    {
      id: "carrier",
      header: t("admin.trade.shipments.carrierCode", "Carrier"),
      width: "18%",
      cell: (shipment) => (
        <span className="text-sm text-[var(--sdk-color-text-secondary)]">{shipment.carrierCode || t("admin.trade.common.noValue", "-")}</span>
      ),
    },
    {
      id: "trackingNo",
      header: t("admin.trade.shipments.trackingNo", "Tracking no."),
      width: "20%",
      cell: (shipment) => (
        <span className="block truncate font-mono text-xs text-[var(--sdk-color-text-secondary)]">
          {shipment.trackingNo || t("admin.trade.common.noValue", "-")}
        </span>
      ),
    },
    {
      id: "fulfillmentId",
      header: t("admin.trade.shipments.fulfillmentId", "Fulfillment"),
      width: "18%",
      cell: (shipment) => (
        <span className="block truncate font-mono text-xs text-[var(--sdk-color-text-secondary)]">
          {shipment.fulfillmentId || t("admin.trade.common.noValue", "-")}
        </span>
      ),
    },
  ], [t]);

  const activeFilterCount = Number(Boolean(statusFilter)) + Number(Boolean(orderIdFilter)) + Number(Boolean(fulfillmentIdFilter));

  return (
    <div aria-label={t("admin.trade.shipments.title", "Shipment management")} className="space-y-4">
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
                options={SHIPMENT_STATUS_OPTIONS}
                placeholder={t("admin.trade.filter.statusPlaceholder", "e.g. submitted")}
                value={draftStatus}
                onChange={setDraftStatus}
              />
            </label>
            <label className="min-w-[12rem] flex-1 space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span>{t("admin.trade.shipments.orderId", "Order ID")}</span>
              <Input
                placeholder={t("admin.trade.filter.orderIdPlaceholder", "Order ID or order number")}
                value={draftOrderId}
                onChange={(event) => setDraftOrderId(event.target.value)}
              />
            </label>
            <label className="min-w-[12rem] flex-1 space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span>{t("admin.trade.shipments.fulfillmentId", "Fulfillment")}</span>
              <Input
                placeholder={t("admin.trade.filter.fulfillmentIdPlaceholder", "Fulfillment ID")}
                value={draftFulfillmentId}
                onChange={(event) => setDraftFulfillmentId(event.target.value)}
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
        description={`${t("admin.trade.shipments.title", "Shipment management")} — ${shipments.length} / ${totalItems}`}
        emptyDescription={activeFilterCount ? t("admin.trade.list.filteredEmptyDescription", "No records match the current filters. Adjust the filters and retry.") : t("admin.trade.list.emptyDescription", "Records matching the current filters will appear here.")}
        emptyTitle={t("admin.trade.list.emptyTitle", "No records")}
        footer={<TradeListPagination loading={loading} onPrev={() => setPage((value) => value - 1)} onNext={() => setPage((value) => value + 1)} page={page} totalItems={totalItems} totalPages={totalPages} />}
        getRowId={(shipment) => shipment.shipmentId}
        loading={loading && shipments.length === 0}
        loadingLabel={t("admin.trade.shipments.title", "Shipment management")}
        onRowClick={(shipment) => setSelectedId(shipment.shipmentId)}
        rowActions={(shipment) => (
          <div className="flex items-center justify-end gap-1">
            <Button aria-label={`${t("admin.trade.list.detail", "Details")} — ${shipment.shipmentNo}`} disabled={busy} size="sm" title={t("admin.trade.list.detail", "Details")} type="button" variant="ghost" onClick={() => setSelectedId(shipment.shipmentId)}>
              <Eye aria-hidden="true" className="mr-1.5 h-4 w-4" />{t("admin.trade.list.detail", "Details")}
            </Button>
          </div>
        )}
        rowActionsLabel={t("admin.trade.list.detail", "Details")}
        rows={shipments}
        stickyHeader
        title={t("admin.trade.shipments.title", "Shipment management")}
        toolbar={(
          <Button aria-label={t("admin.trade.list.refresh", "Refresh")} disabled={loading} size="icon" title={t("admin.trade.list.refresh", "Refresh")} type="button" variant="outline" onClick={() => setRefreshKey((current) => current + 1)}>
            <RefreshCw aria-hidden="true" className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        )}
      />

      <Drawer open={Boolean(selectedId)} onOpenChange={(open) => { if (!open) setSelectedId(null); }}>
        <DrawerContent size="lg">
          <DrawerHeader>
            <DrawerTitle>{t("admin.trade.shipments.detailTitle", "Shipment detail")}</DrawerTitle>
            <DrawerDescription>{detail?.shipmentNo ?? selectedId}</DrawerDescription>
          </DrawerHeader>
          <DrawerBody>
            {detailLoading ? <LoadingBlock label={t("admin.trade.shipments.detailTitle", "Shipment detail")} /> : null}
            {detailError ? <StatusNotice tone="danger">{detailError}</StatusNotice> : null}
            {detail ? (
              <div className="space-y-6">
                <dl className="grid grid-cols-1 gap-x-6 gap-y-5 sm:grid-cols-2">
                  <DetailRow label={t("admin.trade.shipments.no", "Shipment no.")}>{detail.shipmentNo}</DetailRow>
                  <DetailRow label={t("admin.trade.filter.status", "Status")}><TradeStatusBadge status={detail.status} /></DetailRow>
                  <DetailRow label={t("admin.trade.shipments.carrierCode", "Carrier")}>{detail.carrierCode || t("admin.trade.common.noValue", "-")}</DetailRow>
                  <DetailRow label={t("admin.trade.shipments.trackingNo", "Tracking no.")}>{detail.trackingNo || t("admin.trade.common.noValue", "-")}</DetailRow>
                  <DetailRow label={t("admin.trade.shipments.fulfillmentId", "Fulfillment")}>{detail.fulfillmentId || t("admin.trade.common.noValue", "-")}</DetailRow>
                </dl>

                <section>
                  <div className="mb-2 flex items-center justify-between gap-3">
                    <h4 className="text-sm font-semibold text-[var(--sdk-color-text-primary)]">
                      {t("admin.trade.shipments.packages", "Packages")}
                    </h4>
                    {canManage ? (
                      <Button size="sm" type="button" variant="outline" onClick={openCreatePackage}>
                        <PackagePlus aria-hidden="true" className="mr-1.5 h-4 w-4" />
                        {t("admin.trade.shipments.addPackage", "Add package")}
                      </Button>
                    ) : null}
                  </div>
                  {packages.length === 0 ? (
                    <p className="rounded-lg border border-dashed border-[var(--sdk-color-border-default)] px-4 py-6 text-center text-sm text-[var(--sdk-color-text-muted)]">
                      {t("admin.trade.shipments.noPackages", "No packages on this shipment yet.")}
                    </p>
                  ) : (
                    <ul className="divide-y divide-[var(--sdk-color-border-default)] rounded-lg border border-[var(--sdk-color-border-default)]">
                      {packages.map((item) => (
                        <li key={item.packageId} className="flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
                          <div className="min-w-0">
                            <p className="truncate text-sm font-semibold text-[var(--sdk-color-text-primary)]">
                              {item.packageNo || item.packageId}
                            </p>
                            <p className="mt-0.5 truncate font-mono text-xs text-[var(--sdk-color-text-muted)]">
                              {item.trackingNo ? `Tracking: ${item.trackingNo}` : item.packageType}
                            </p>
                          </div>
                          <div className="flex shrink-0 items-center gap-3">
                            <TradeStatusBadge status={item.status} />
                            {canManage ? (
                              <Button aria-label={`${t("admin.trade.shipments.editPackage", "Edit package")} — ${item.packageNo}`} disabled={busy} size="sm" title={t("admin.trade.shipments.editPackage", "Edit package")} type="button" variant="ghost" onClick={() => openEditPackage(item)}>
                                <Pencil aria-hidden="true" className="h-3.5 w-3.5" />
                              </Button>
                            ) : null}
                          </div>
                        </li>
                      ))}
                    </ul>
                  )}
                </section>
              </div>
            ) : null}
          </DrawerBody>
          <DrawerFooter>
            <Button onClick={() => setSelectedId(null)} type="button" variant="secondary">{t("admin.trade.list.close", "Close")}</Button>
          </DrawerFooter>
        </DrawerContent>
      </Drawer>

      <Modal open={packageDialogOpen} onOpenChange={(open) => { if (!busy) setPackageDialogOpen(open); }}>
        <ModalContent size="sm">
          <ModalHeader>
            <ModalTitle>
              {packageDraft.packageId
                ? t("admin.trade.shipments.editPackage", "Edit package")
                : t("admin.trade.shipments.addPackage", "Add package")}
            </ModalTitle>
            <ModalDescription>{detail?.shipmentNo ?? ""}</ModalDescription>
          </ModalHeader>
          <form onSubmit={(event) => { event.preventDefault(); void savePackage(); }}>
            <ModalBody>
              <div className="space-y-4">
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.shipments.packageType", "Package type")}</span>
                  <Input
                    placeholder={t("admin.trade.shipments.packageTypePlaceholder", "e.g. standard")}
                    required
                    value={packageDraft.packageType}
                    onChange={(event) => setPackageDraft((draft) => ({ ...draft, packageType: event.target.value }))}
                  />
                </label>
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.shipments.packageNo", "Package no.")}</span>
                  <Input
                    value={packageDraft.packageNo}
                    onChange={(event) => setPackageDraft((draft) => ({ ...draft, packageNo: event.target.value }))}
                  />
                </label>
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.shipments.trackingNo", "Tracking no.")}</span>
                  <Input
                    placeholder={t("admin.trade.shipments.trackingNoPlaceholder", "Carrier tracking number")}
                    value={packageDraft.trackingNo}
                    onChange={(event) => setPackageDraft((draft) => ({ ...draft, trackingNo: event.target.value }))}
                  />
                </label>
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.trade.shipments.packageStatus", "Package status")}</span>
                  <TradeStatusSelect
                    ariaLabel={t("admin.trade.shipments.packageStatus", "Package status")}
                    options={PACKAGE_STATUS_OPTIONS}
                    placeholder={t("admin.trade.shipments.packageStatusPlaceholder", "e.g. created")}
                    value={packageDraft.status}
                    onChange={(status) => setPackageDraft((draft) => ({ ...draft, status }))}
                  />
                </label>
              </div>
            </ModalBody>
            <ModalFooter>
              <Button disabled={busy} type="button" variant="secondary" onClick={() => setPackageDialogOpen(false)}>
                {t("admin.trade.list.close", "Close")}
              </Button>
              <Button disabled={busy} loading={busy} type="submit">
                {t("admin.trade.review.confirm", "Confirm")}
              </Button>
            </ModalFooter>
          </form>
        </ModalContent>
      </Modal>
    </div>
  );
}
