import { useEffect, useMemo, useState, type FormEvent } from "react";
import {
  Archive,
  Ban,
  ChevronLeft,
  ChevronRight,
  Eye,
  RefreshCw,
  RotateCcw,
  Search,
} from "lucide-react";
import {
  AdminOrdersIntlProvider,
  useAdminOrdersI18n,
  type AdminOrdersIntlProps,
} from "../i18n/intl";
import {
  Button,
  ConfirmDialog,
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
  StatusBadge,
  StatusNotice,
  type StatusBadgeVariant,
} from "@sdkwork/ui-pc-react";
import {
  getSdkworkOrderBackendSdkClient,
  type OrderDetail,
  type OrderEvent,
  type OrderSummary,
  type ShipmentSummary,
} from "@sdkwork/order-pc-admin-core";
import { createOrderAdminService, type OrderAdminService } from "../order-admin-service";

const DEFAULT_PAGE_SIZE = 20;

type OrderMutation = {
  action: "cancel" | "close";
  orderId: string;
  orderLabel: string;
};

export interface SdkworkOrderAdminOrdersPageProps extends AdminOrdersIntlProps {
  capabilities: SdkworkOrderAdminCapabilities;
  service?: OrderAdminService;
}

export interface SdkworkOrderAdminCapabilities {
  canManageOrders: boolean;
}

const STATUS_FILTER_OPTIONS = [
  { labelKey: "admin.orders.status.pending_payment", value: "pending_payment" },
  { labelKey: "admin.orders.status.paid", value: "paid" },
  { labelKey: "admin.orders.status.fulfilled", value: "fulfilled" },
  { labelKey: "admin.orders.status.completed", value: "completed" },
  { labelKey: "admin.orders.status.cancelled", value: "cancelled" },
  { labelKey: "admin.orders.status.expired", value: "expired" },
  { labelKey: "admin.orders.status.refunding", value: "refunding" },
  { labelKey: "admin.orders.status.refunded", value: "refunded" },
];

function formatTimestamp(value?: string): string {
  if (!value) return "--";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN");
}

function resolveStatusVariant(status: string): StatusBadgeVariant {
  const normalized = status.toLowerCase();
  if (["paid", "completed", "succeeded", "success", "fulfilled"].includes(normalized)) return "success";
  if (["pending", "pending_payment", "processing", "shipping", "refunding"].includes(normalized)) return "warning";
  if (["failed", "cancelled", "canceled", "rejected", "expired"].includes(normalized)) return "danger";
  if (["closed", "refunded", "archived"].includes(normalized)) return "secondary";
  return "default";
}

function resolveActorLabel(actorType: string, t: (key: string, fallback: string) => string): string {
  const normalized = actorType.toLowerCase();
  if (["system", "platform"].includes(normalized)) return t("admin.orders.actor.system", "System");
  if (["user", "buyer", "customer"].includes(normalized)) return t("admin.orders.actor.user", "User");
  if (["partner"].includes(normalized)) return t("admin.orders.actor.partner", "Partner");
  if (["operator", "admin", "manager"].includes(normalized)) return t("admin.orders.actor.operator", "Operator");
  return actorType || t("admin.orders.actor.unknown", "Unknown actor");
}

export function SdkworkOrderAdminOrdersPage({
  capabilities,
  service: injectedService,
  locale,
  messages,
}: SdkworkOrderAdminOrdersPageProps) {
  return (
    <AdminOrdersIntlProvider locale={locale} messages={messages}>
      <AdminOrdersPageInner capabilities={capabilities} service={injectedService} />
    </AdminOrdersIntlProvider>
  );
}

function AdminOrdersPageInner({
  capabilities,
  service: injectedService,
}: {
  capabilities: SdkworkOrderAdminCapabilities;
  service?: OrderAdminService;
}) {
  const { t } = useAdminOrdersI18n();
  const service = useMemo(
    () => injectedService ?? createOrderAdminService(getSdkworkOrderBackendSdkClient()),
    [injectedService],
  );
  const [orders, setOrders] = useState<OrderSummary[]>([]);
  const [page, setPage] = useState(1);
  const [draftStatus, setDraftStatus] = useState("");
  const [draftQuery, setDraftQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [totalItems, setTotalItems] = useState(0);
  const [totalPages, setTotalPages] = useState(1);
  const [loading, setLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<OrderDetail | null>(null);
  const [events, setEvents] = useState<OrderEvent[]>([]);
  const [shipments, setShipments] = useState<ShipmentSummary[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [mutationTarget, setMutationTarget] = useState<OrderMutation | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setListError(null);
    void service.listOrders({
      page,
      pageSize: DEFAULT_PAGE_SIZE,
      status: statusFilter || undefined,
      q: searchQuery || undefined,
    }).then((result) => {
      if (!active) return;
      setOrders(result.items);
      setTotalItems(result.totalItems);
      setTotalPages(Math.max(1, result.totalPages));
    }).catch(() => {
      if (!active) return;
      setListError(t("admin.orders.message.listFailed", "Order list loading failed. Check commerce.orders.read permission and network."));
      setOrders([]);
      setTotalItems(0);
    }).finally(() => {
      if (active) setLoading(false);
    });
    return () => { active = false; };
  }, [page, refreshKey, searchQuery, service, statusFilter, t]);

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      setEvents([]);
      setShipments([]);
      setDetailError(null);
      return;
    }
    let active = true;
    setDetailLoading(true);
    setDetailError(null);
    void Promise.all([
      service.getOrder(selectedId),
      service.getOrderEvents(selectedId).catch(() => [] as OrderEvent[]),
      service.getOrderShipments(selectedId).catch(() => [] as ShipmentSummary[]),
    ]).then(([value, orderEvents, orderShipments]) => {
      if (!active) return;
      setDetail(value);
      setEvents(orderEvents);
      setShipments(orderShipments);
    }).catch(() => {
      if (!active) return;
      setDetail(null);
      setEvents([]);
      setShipments([]);
      setDetailError(t("admin.orders.message.detailFailed", "Order detail loading failed. Please retry later."));
    }).finally(() => {
      if (active) setDetailLoading(false);
    });
    return () => { active = false; };
  }, [selectedId, service, t]);

  const applyFilters = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setPage(1);
    setStatusFilter(draftStatus.trim());
    setSearchQuery(draftQuery.trim());
    setRefreshKey((current) => current + 1);
  };

  const resetFilters = () => {
    setDraftStatus("");
    setDraftQuery("");
    setStatusFilter("");
    setSearchQuery("");
    setPage(1);
    setRefreshKey((current) => current + 1);
  };

  async function mutateOrder(target: OrderMutation) {
    setBusyId(target.orderId);
    setMessage(null);
    setListError(null);
    try {
      if (target.action === "cancel") {
        await service.cancelOrder(target.orderId);
      } else {
        await service.closeOrder(target.orderId);
      }
      setMessage(t(
        target.action === "cancel" ? "admin.orders.message.cancelled" : "admin.orders.message.closed",
        target.action === "cancel" ? "Order {{label}} has been cancelled." : "Order {{label}} has been closed.",
        { label: target.orderLabel },
      ));
      setRefreshKey((current) => current + 1);
    } catch {
      setListError(t("admin.orders.message.actionFailed", "Operation failed. Check commerce.orders.manage permission and the current order status."));
    } finally {
      setBusyId(null);
      setMutationTarget(null);
    }
  }

  const columns = useMemo<DataTableColumn<OrderSummary>[]>(() => [
    {
      id: "order",
      header: t("admin.orders.column.order", "Order"),
      width: "34%",
      cell: (order) => (
        <button
          className="min-w-0 text-left"
          onClick={() => setSelectedId(order.orderId)}
          type="button"
        >
          <span className="block truncate text-sm font-semibold text-[var(--sdk-color-text-primary)]">
            {order.subject || order.orderSn || order.orderId}
          </span>
          <span className="mt-1 block truncate font-mono text-xs text-[var(--sdk-color-text-muted)]">
            {order.orderSn || order.orderId}
          </span>
        </button>
      ),
    },
    {
      id: "status",
      header: t("admin.orders.column.status", "Status"),
      width: "18%",
      cell: (order) => (
        <StatusBadge
          label={order.statusName || order.status}
          showIcon
          status={order.status}
          variant={resolveStatusVariant(order.status)}
        />
      ),
    },
    {
      align: "right",
      id: "amount",
      header: t("admin.orders.column.amount", "Amount"),
      width: "20%",
      cell: (order) => (
        <span className="font-mono text-sm font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">
          {order.totalAmount}
        </span>
      ),
    },
    {
      id: "partner",
      header: t("admin.orders.column.partner", "Partner"),
      width: "16%",
      cell: (order) =>
        order.partnerName ? (
          <span className="block min-w-0">
            <span className="block truncate text-sm font-medium text-[var(--sdk-color-text-primary)]">
              {order.partnerName}
            </span>
            <span className="mt-0.5 block truncate font-mono text-xs text-[var(--sdk-color-text-muted)]">
              #{order.partnerId}
            </span>
          </span>
        ) : (
          <span className="text-sm text-[var(--sdk-color-text-muted)]">-</span>
        ),
    },
    {
      id: "createdAt",
      header: t("admin.orders.column.createdAt", "Created at"),
      width: "28%",
      cell: (order) => (
        <time className="whitespace-nowrap text-sm text-[var(--sdk-color-text-secondary)]" dateTime={order.createdAt}>
          {formatTimestamp(order.createdAt)}
        </time>
      ),
    },
  ], [t]);

  const activeFilterCount = Number(Boolean(statusFilter)) + Number(Boolean(searchQuery));

  return (
    <div aria-label={t("admin.orders.title", "Order supervision")} className="space-y-4">
      <form onSubmit={applyFilters}>
        <FilterBar
          summary={activeFilterCount ? t("admin.orders.filter.applied", "{{count}} filter(s) applied", { count: activeFilterCount }) : undefined}
          title={t("admin.orders.filter.title", "Filters")}
        >
          <FilterBarSection>
            <label className="min-w-[12rem] flex-1 space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span>{t("admin.orders.filter.status", "Status")}</span>
              <select
                aria-label={t("admin.orders.filter.status", "Status")}
                className="h-9 w-full rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-3 text-sm text-[var(--sdk-color-text-primary)] outline-none transition-colors focus-visible:border-[var(--sdk-color-brand-primary)]"
                value={draftStatus}
                onChange={(event) => setDraftStatus(event.target.value)}
              >
                <option value="">{t("admin.orders.filter.status", "Status")}</option>
                {STATUS_FILTER_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>{t(option.labelKey, option.value)}</option>
                ))}
              </select>
            </label>
            <label className="min-w-[16rem] flex-[1.5] space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span>{t("admin.orders.filter.search", "Search")}</span>
              <div className="relative">
                <Search aria-hidden="true" className="pointer-events-none absolute left-3 inset-y-0 my-auto h-4 w-4 text-[var(--sdk-color-text-muted)]" />
                <Input
                  className="pl-9"
                  placeholder={t("admin.orders.filter.searchPlaceholder", "Order no., subject or reference")}
                  value={draftQuery}
                  onChange={(event) => setDraftQuery(event.target.value)}
                />
              </div>
            </label>
          </FilterBarSection>
          <FilterBarActions>
            <Button disabled={loading} type="button" variant="outline" onClick={resetFilters}>
              <RotateCcw aria-hidden="true" className="mr-2 h-4 w-4" />
              {t("admin.orders.filter.reset", "Reset")}
            </Button>
            <Button disabled={loading} type="submit">
              <Search aria-hidden="true" className="mr-2 h-4 w-4" />
              {t("admin.orders.filter.query", "Query")}
            </Button>
          </FilterBarActions>
        </FilterBar>
      </form>

      {listError ? <StatusNotice tone="danger">{listError}</StatusNotice> : null}
      {message ? <StatusNotice tone="success">{message}</StatusNotice> : null}

      <DataTable
        columns={columns}
        density="compact"
        description={t("admin.orders.table.description", "Showing {{count}} of {{total}} orders", { count: orders.length, total: totalItems })}
        emptyDescription={activeFilterCount ? t("admin.orders.table.filteredEmptyDescription", "No orders match. Adjust the filters and retry.") : t("admin.orders.table.emptyDescription", "Platform orders will be listed here.")}
        emptyTitle={t("admin.orders.table.emptyTitle", "No orders")}
        footer={(
          <div className="flex min-w-0 flex-1 flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <span className="min-w-0 truncate text-sm tabular-nums text-[var(--sdk-color-text-secondary)]">
              {t("admin.orders.table.totalRecords", "{{count}} records in total", { count: totalItems })}
            </span>
            <div className="grid w-full grid-cols-[2.25rem_minmax(4.5rem,1fr)_2.25rem] items-center gap-1 sm:w-auto sm:grid-cols-[2.25rem_minmax(4.5rem,auto)_2.25rem]">
              <Button
                aria-label={t("admin.orders.table.prevPage", "Previous page")}
                className="h-9 w-9 p-0"
                disabled={page <= 1 || loading}
                size="icon"
                title={t("admin.orders.table.prevPage", "Previous page")}
                type="button"
                variant="outline"
                onClick={() => setPage((value) => value - 1)}
              >
                <ChevronLeft aria-hidden="true" className="h-4 w-4" />
              </Button>
              <span
                aria-label={t("admin.orders.table.pageInfo", "Page {{page}} of {{totalPages}}", { page, totalPages })}
                className="flex h-9 items-center justify-center gap-2 rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-3 text-sm tabular-nums text-[var(--sdk-color-text-muted)]"
              >
                <strong className="font-semibold text-[var(--sdk-color-text-primary)]">{page}</strong>
                <span aria-hidden="true">/</span>
                <span aria-hidden="true">{totalPages}</span>
              </span>
              <Button
                aria-label={t("admin.orders.table.nextPage", "Next page")}
                className="h-9 w-9 p-0"
                disabled={page >= totalPages || loading}
                size="icon"
                title={t("admin.orders.table.nextPage", "Next page")}
                type="button"
                variant="outline"
                onClick={() => setPage((value) => value + 1)}
              >
                <ChevronRight aria-hidden="true" className="h-4 w-4" />
              </Button>
            </div>
          </div>
        )}
        getRowId={(order) => order.orderId}
        loading={loading && orders.length === 0}
        loadingLabel={t("admin.orders.loading", "Loading orders...")}
        onRowClick={(order) => setSelectedId(order.orderId)}
        rowActions={(order) => (
          <div className="flex items-center justify-end gap-1">
            <Button aria-label={`${t("admin.orders.action.detail", "Details")} — ${order.subject || order.orderSn || order.orderId}`} disabled={busyId === order.orderId} size="sm" title={t("admin.orders.action.detail", "Details")} type="button" variant="ghost" onClick={() => setSelectedId(order.orderId)}>
              <Eye aria-hidden="true" className="mr-1.5 h-4 w-4" />{t("admin.orders.action.detail", "Details")}
            </Button>
            {capabilities.canManageOrders ? (
              <>
                <Button disabled={Boolean(busyId)} size="sm" title={t("admin.orders.action.cancel", "Cancel")} type="button" variant="outline" onClick={() => setMutationTarget({ action: "cancel", orderId: order.orderId, orderLabel: order.orderSn || order.orderId })}>
                  <Ban aria-hidden="true" className="mr-1.5 h-4 w-4" />{t("admin.orders.action.cancel", "Cancel")}
                </Button>
                <Button disabled={Boolean(busyId)} size="sm" title={t("admin.orders.action.close", "Close")} type="button" variant="outline" onClick={() => setMutationTarget({ action: "close", orderId: order.orderId, orderLabel: order.orderSn || order.orderId })}>
                  <Archive aria-hidden="true" className="mr-1.5 h-4 w-4" />{t("admin.orders.action.close", "Close")}
                </Button>
              </>
            ) : null}
          </div>
        )}
        rowActionsLabel={t("admin.orders.action.detail", "Details")}
        rows={orders}
        stickyHeader
        title={t("admin.orders.table.title", "Order list")}
        toolbar={(
          <Button aria-label={t("admin.orders.refresh", "Refresh")} disabled={loading} size="icon" title={t("admin.orders.refresh", "Refresh")} type="button" variant="outline" onClick={() => setRefreshKey((current) => current + 1)}>
            <RefreshCw aria-hidden="true" className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        )}
      />

      <Drawer open={Boolean(selectedId)} onOpenChange={(open) => { if (!open) setSelectedId(null); }}>
        <DrawerContent size="xl">
          <DrawerHeader>
            <DrawerTitle>{t("admin.orders.detail.title", "Order detail")}</DrawerTitle>
            <DrawerDescription>{detail?.orderSn ?? selectedId}</DrawerDescription>
          </DrawerHeader>
          <DrawerBody>
            {detailLoading ? <LoadingBlock label={t("admin.orders.loadingDetail", "Loading order detail...")} /> : null}
            {detailError ? <StatusNotice tone="danger">{detailError}</StatusNotice> : null}
            {detail ? (
              <div className="space-y-6">
                <dl className="grid grid-cols-1 gap-x-6 gap-y-5 sm:grid-cols-2 lg:grid-cols-3">
                  <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.orderSn", "Order no.")}</dt><dd className="mt-1 break-all font-mono text-sm text-[var(--sdk-color-text-primary)]">{detail.orderSn}</dd></div>
                  <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.status", "Status")}</dt><dd className="mt-1"><StatusBadge label={detail.statusName || detail.status} showIcon status={detail.status} variant={resolveStatusVariant(detail.status)} /></dd></div>
                  <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.amount", "Amount")}</dt><dd className="mt-1 font-mono text-sm font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">{detail.totalAmount}</dd></div>
                  {detail.paidAmount ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.paidAmount", "Paid amount")}</dt><dd className="mt-1 font-mono text-sm tabular-nums text-[var(--sdk-color-text-primary)]">{detail.paidAmount}</dd></div> : null}
                  {detail.discountAmount ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.discountAmount", "Discount")}</dt><dd className="mt-1 font-mono text-sm tabular-nums text-[var(--sdk-color-text-primary)]">{detail.discountAmount}</dd></div> : null}
                  <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.quantity", "Quantity")}</dt><dd className="mt-1 text-sm tabular-nums text-[var(--sdk-color-text-primary)]">{detail.quantity}</dd></div>
                  {detail.paymentMethod ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.paymentMethod", "Payment method")}</dt><dd className="mt-1 text-sm text-[var(--sdk-color-text-primary)]">{detail.paymentMethod}</dd></div> : null}
                  {detail.outTradeNo ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.outTradeNo", "Out trade no.")}</dt><dd className="mt-1 break-all font-mono text-sm text-[var(--sdk-color-text-primary)]">{detail.outTradeNo}</dd></div> : null}
                  {detail.transactionId ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.transactionId", "Transaction ID")}</dt><dd className="mt-1 break-all font-mono text-sm text-[var(--sdk-color-text-primary)]">{detail.transactionId}</dd></div> : null}
                  {detail.payTime ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.payTime", "Paid at")}</dt><dd className="mt-1 text-sm text-[var(--sdk-color-text-primary)]">{formatTimestamp(detail.payTime)}</dd></div> : null}
                  {detail.expireTime ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.expireTime", "Expires at")}</dt><dd className="mt-1 text-sm text-[var(--sdk-color-text-primary)]">{formatTimestamp(detail.expireTime)}</dd></div> : null}
                  {detail.partnerName ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.partner", "Partner")}</dt><dd className="mt-1 text-sm text-[var(--sdk-color-text-primary)]">{detail.partnerName}</dd></div> : null}
                </dl>

                {detail.items.length > 0 ? (
                  <section>
                    <h4 className="mb-2 text-sm font-semibold text-[var(--sdk-color-text-primary)]">{t("admin.orders.detail.items", "Items")}</h4>
                    <div className="overflow-x-auto rounded-lg border border-[var(--sdk-color-border-default)]">
                      <table className="w-full min-w-[32rem] text-left text-sm">
                        <thead className="border-b border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-muted)]">
                          <tr className="text-xs font-medium text-[var(--sdk-color-text-muted)]">
                            <th className="px-4 py-2.5">{t("admin.orders.detail.item.product", "Product")}</th>
                            <th className="px-4 py-2.5 text-right">{t("admin.orders.detail.item.price", "Unit price")}</th>
                            <th className="px-4 py-2.5 text-right">{t("admin.orders.detail.item.quantity", "Qty")}</th>
                            <th className="px-4 py-2.5 text-right">{t("admin.orders.detail.item.subtotal", "Subtotal")}</th>
                          </tr>
                        </thead>
                        <tbody className="divide-y divide-[var(--sdk-color-border-default)]">
                          {detail.items.map((item) => (
                            <tr key={item.id}>
                              <td className="px-4 py-2.5 text-[var(--sdk-color-text-primary)]">{item.productName}</td>
                              <td className="px-4 py-2.5 text-right font-mono tabular-nums text-[var(--sdk-color-text-secondary)]">{item.unitPrice}</td>
                              <td className="px-4 py-2.5 text-right tabular-nums text-[var(--sdk-color-text-secondary)]">{item.quantity}</td>
                              <td className="px-4 py-2.5 text-right font-mono font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">{item.totalAmount}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  </section>
                ) : null}

                <section>
                  <h4 className="mb-2 text-sm font-semibold text-[var(--sdk-color-text-primary)]">
                    {t("admin.orders.detail.fulfillment", "Fulfillment")}
                  </h4>
                  {shipments.length === 0 ? (
                    <p className="rounded-lg border border-dashed border-[var(--sdk-color-border-default)] px-4 py-4 text-center text-sm text-[var(--sdk-color-text-muted)]">
                      {t("admin.orders.detail.fulfillment.empty", "No shipments for this order yet.")}
                    </p>
                  ) : (
                    <ul className="divide-y divide-[var(--sdk-color-border-default)] rounded-lg border border-[var(--sdk-color-border-default)]">
                      {shipments.map((shipment) => (
                        <li key={shipment.shipmentId} className="flex flex-col gap-1 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
                          <div className="min-w-0">
                            <p className="truncate font-mono text-sm font-semibold text-[var(--sdk-color-text-primary)]">
                              {shipment.shipmentNo}
                            </p>
                            <p className="mt-0.5 truncate text-xs text-[var(--sdk-color-text-muted)]">
                              {[shipment.carrierCode, shipment.trackingNo].filter(Boolean).join(" · ") || shipment.fulfillmentId}
                            </p>
                          </div>
                          <StatusBadge label={shipment.status} showIcon status={shipment.status} variant={resolveStatusVariant(shipment.status)} />
                        </li>
                      ))}
                    </ul>
                  )}
                </section>

                <section>
                  <h4 className="mb-2 text-sm font-semibold text-[var(--sdk-color-text-primary)]">{t("admin.orders.detail.events", "Order timeline")}</h4>
                  {events.length === 0 ? (
                    <p className="rounded-lg border border-dashed border-[var(--sdk-color-border-default)] px-4 py-6 text-center text-sm text-[var(--sdk-color-text-muted)]">
                      {t("admin.orders.detail.events.empty", "No events recorded yet.")}
                    </p>
                  ) : (
                    <ol className="relative ml-2 space-y-4 border-l border-[var(--sdk-color-border-default)] pl-4">
                      {[...events]
                        .sort((left, right) => right.createdAt.localeCompare(left.createdAt))
                        .map((event) => (
                        <li key={event.id} className="relative">
                          <span aria-hidden="true" className="absolute -left-[1.31rem] top-1 h-2 w-2 rounded-full bg-[var(--sdk-color-brand-primary)]" />
                          <p className="text-sm font-medium text-[var(--sdk-color-text-primary)]">
                            {event.message || `${event.fromStatus ?? ""} → ${event.toStatus}`}
                          </p>
                          <p className="mt-0.5 flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-[var(--sdk-color-text-muted)]">
                            <span>{resolveActorLabel(event.actorType, t)}</span>
                            {event.eventType ? <span className="font-mono">{event.eventType}</span> : null}
                            <time dateTime={event.createdAt}>{formatTimestamp(event.createdAt)}</time>
                          </p>
                        </li>
                      ))}
                    </ol>
                  )}
                </section>
              </div>
            ) : null}
          </DrawerBody>
          <DrawerFooter><Button onClick={() => setSelectedId(null)} type="button" variant="secondary">{t("admin.orders.detail.close", "Close")}</Button></DrawerFooter>
        </DrawerContent>
      </Drawer>

      <ConfirmDialog
        cancelLabel={t("admin.orders.confirm.back", "Back")}
        closeOnConfirm={false}
        confirmLabel={mutationTarget?.action === "cancel" ? t("admin.orders.confirm.cancelLabel", "Confirm cancel") : t("admin.orders.confirm.closeLabel", "Confirm close")}
        confirmLoading={Boolean(busyId)}
        description={mutationTarget ? t(
          mutationTarget.action === "cancel" ? "admin.orders.confirm.cancelDescription" : "admin.orders.confirm.closeDescription",
          mutationTarget.action === "cancel" ? "Cancelling order {{label}} affects downstream fulfillment." : "Closing order {{label}} affects downstream fulfillment.",
          { label: mutationTarget.orderLabel },
        ) : undefined}
        onConfirm={() => { if (mutationTarget) void mutateOrder(mutationTarget); }}
        onOpenChange={(open) => { if (!open && !busyId) setMutationTarget(null); }}
        open={Boolean(mutationTarget)}
        title={mutationTarget?.action === "cancel" ? t("admin.orders.confirm.cancelTitle", "Cancel order") : t("admin.orders.confirm.closeTitle", "Close order")}
        tone="warning"
      />
    </div>
  );
}
