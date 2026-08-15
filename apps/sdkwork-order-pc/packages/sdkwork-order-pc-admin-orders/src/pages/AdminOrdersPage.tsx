import { useEffect, useMemo, useState, type FormEvent } from "react";
import {
  Archive,
  BadgeCheck,
  Ban,
  Check,
  Copy,
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  Download,
  Eye,
  RefreshCw,
  RotateCcw,
  Search,
  Undo2,
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
  Modal,
  ModalBody,
  ModalContent,
  ModalDescription,
  ModalFooter,
  ModalHeader,
  ModalTitle,
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
import { useOrderAdminLink } from "../navigation";

const DEFAULT_PAGE_SIZE = 20;

type OrderMutation = {
  action: "cancel" | "close";
  orderId: string;
  orderLabel: string;
};

const REFUND_REASON_OPTIONS = [
  { labelKey: "admin.orders.refund.quick.user_request", value: "user_request" },
  { labelKey: "admin.orders.refund.quick.product_issue", value: "product_issue" },
  { labelKey: "admin.orders.refund.quick.compensation", value: "compensation" },
  { labelKey: "admin.orders.refund.quick.system_error", value: "system_error" },
];

const QUICK_MUTATION_REASONS: Record<"cancel" | "close", { labelKey: string; value: string }[]> = {
  cancel: [
    { labelKey: "admin.orders.mutation.quick.user_request", value: "user_request" },
    { labelKey: "admin.orders.mutation.quick.timeout", value: "timeout" },
    { labelKey: "admin.orders.mutation.quick.admin_operation", value: "admin_operation" },
    { labelKey: "admin.orders.mutation.quick.system_error", value: "system_error" },
  ],
  close: [
    { labelKey: "admin.orders.mutation.quick.admin_operation", value: "admin_operation" },
    { labelKey: "admin.orders.mutation.quick.close_archived", value: "close_archived" },
    { labelKey: "admin.orders.mutation.quick.close_correction", value: "close_correction" },
  ],
};

type PaymentConfirmationTarget = {
  orderId: string;
  orderLabel: string;
};

export interface SdkworkOrderAdminOrdersPageProps extends AdminOrdersIntlProps {
  capabilities: SdkworkOrderAdminCapabilities;
  service?: OrderAdminService;
}

export interface SdkworkOrderAdminCapabilities {
  canManageOrders: boolean;
  /** `commerce.orders.fulfill` — reconcile a provider payment on an order. */
  canConfirmPayment?: boolean;
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

function formatAmount(value?: string, locale?: string, currencyCode?: string): string {
  if (!value) return "-";
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return value;
  const formatted = numeric.toLocaleString(locale ?? "zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
  return currencyCode ? `${formatted} ${currencyCode}` : formatted;
}

function csvCell(cell: string | number | null | undefined): string {
  const text = String(cell ?? "");
  return /[",\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

function exportOrdersCsv(headers: string[], rows: (string | number | null | undefined)[][]): void {
  const lines = [headers, ...rows].map((row) => row.map(csvCell).join(","));
  // UTF-8 BOM keeps Chinese headers readable in Excel.
  const blob = new Blob([`\uFEFF${lines.join("\r\n")}`], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "orders-" + new Date().toISOString().slice(0, 10).replaceAll("-", "") + ".csv";
  anchor.click();
  URL.revokeObjectURL(url);
}

function formatTimestamp(value?: string, locale?: string): string {
  if (!value) return "--";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString(locale ?? "zh-CN");
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

/**
 * Resolves a localized order or shipment status label.
 *
 * Order status keys (`admin.orders.status.<status>`) win over the backend
 * `statusName` snapshot so the admin surface stays consistent in the active
 * locale; unknown statuses fall back to `statusName` then the raw value.
 */

function readOrdersUrlFilter(key: string): string {
  if (typeof window === "undefined") return "";
  try {
    return new URLSearchParams(window.location.search).get(key) ?? "";
  } catch {
    return "";
  }
}

function resolveOrderStatusLabel(status: string, statusName: string | undefined, t: (key: string, fallback?: string) => string): string {
  const normalized = String(status ?? "").toLowerCase();
  if (!normalized) return "-";
  const label = t(`admin.orders.status.${normalized}`, "");
  return label !== "" ? label : statusName ?? status;
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
  const { t, locale } = useAdminOrdersI18n();
  const OrderLink = useOrderAdminLink();
  const service = useMemo(
    () => injectedService ?? createOrderAdminService(getSdkworkOrderBackendSdkClient()),
    [injectedService],
  );
  const [orders, setOrders] = useState<OrderSummary[]>([]);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(DEFAULT_PAGE_SIZE);
  const [pageInput, setPageInput] = useState("1");
  const [draftStatus, setDraftStatus] = useState("");
  const [draftOrderNo, setDraftOrderNo] = useState("");
  const [draftQuery, setDraftQuery] = useState("");
  const [draftCreatedFrom, setDraftCreatedFrom] = useState("");
  const [draftCreatedTo, setDraftCreatedTo] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [createdFromFilter, setCreatedFromFilter] = useState("");
  const [createdToFilter, setCreatedToFilter] = useState("");
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
  const [mutationReason, setMutationReason] = useState("");
  const [mutationDetail, setMutationDetail] = useState("");
  // One idempotency key per mutation intent: reused across retries of the
  // same submission so a double click never executes the command twice.
  const [mutationIdempotencyKey, setMutationIdempotencyKey] = useState("");
  const [paymentIdempotencyKey, setPaymentIdempotencyKey] = useState("");
  const [paymentTarget, setPaymentTarget] = useState<PaymentConfirmationTarget | null>(null);
  const [paymentRequestNo, setPaymentRequestNo] = useState("");
  const [copiedField, setCopiedField] = useState<string | null>(null);
  const [refundTarget, setRefundTarget] = useState<{ orderId: string; orderLabel: string; paidAmount?: string } | null>(null);
  const [refundAmount, setRefundAmount] = useState("");
  const [refundReason, setRefundReason] = useState("");
  const [refundDetail, setRefundDetail] = useState("");
  // One idempotency key per refund intent: reused across retries of the same
  // submission so a double click can never create two refund requests.
  const [refundIdempotencyKey, setRefundIdempotencyKey] = useState("");
  const [refreshKey, setRefreshKey] = useState(0);
  const urlStatus = readOrdersUrlFilter("status");
  const urlQuery = readOrdersUrlFilter("q");
  useEffect(() => {
    if (urlStatus) {
      setStatusFilter(urlStatus);
      setDraftStatus(urlStatus);
    }
    if (urlQuery) {
      setSearchQuery(urlQuery);
      setDraftQuery(urlQuery);
    }
    // Apply deep links only when the URL changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [urlQuery, urlStatus]);


  useEffect(() => {
    let active = true;
    setLoading(true);
    setListError(null);
    void service.listOrders({
      page,
      pageSize,
      status: statusFilter || undefined,
      q: searchQuery || undefined,
      createdFrom: createdFromFilter || undefined,
      createdTo: createdToFilter || undefined,
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
  }, [createdFromFilter, createdToFilter, page, pageSize, refreshKey, searchQuery, service, statusFilter, t]);

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
    // The order number input takes precedence; the keyword input covers
    // order number, subject, and reference searches through the same `q`.
    setSearchQuery(draftOrderNo.trim() || draftQuery.trim());
    // Native date inputs submit `yyyy-MM-dd`; normalize into full-day
    // timestamps so the backend range comparison is inclusive.
    setCreatedFromFilter(draftCreatedFrom ? `${draftCreatedFrom}T00:00:00` : "");
    setCreatedToFilter(draftCreatedTo ? `${draftCreatedTo}T23:59:59` : "");
    setRefreshKey((current) => current + 1);
  };

  const resetFilters = () => {
    setDraftStatus("");
    setDraftOrderNo("");
    setDraftQuery("");
    setDraftCreatedFrom("");
    setDraftCreatedTo("");
    setStatusFilter("");
    setSearchQuery("");
    setCreatedFromFilter("");
    setCreatedToFilter("");
    setPage(1);
    setRefreshKey((current) => current + 1);
  };

  const changePageSize = (nextPageSize: number) => {
    setPageSize(nextPageSize);
    setPage(1);
  };

  const applyQuickRange = (days: number) => {
    const to = new Date();
    const from = new Date();
    from.setDate(from.getDate() - (days - 1));
    const formatDate = (date: Date) => date.toISOString().slice(0, 10);
    setDraftCreatedFrom(formatDate(from));
    setDraftCreatedTo(formatDate(to));
  };

  const openRefund = (target: { orderId: string; orderLabel: string; paidAmount?: string }) => {
    setRefundTarget(target);
    setRefundAmount(target.paidAmount ?? "");
    setRefundReason("");
    setRefundDetail("");
    setRefundIdempotencyKey(crypto.randomUUID());
  };

  async function submitRefund() {
    if (!refundTarget) return;
    const target = refundTarget;
    setBusyId(target.orderId);
    setMessage(null);
    setListError(null);
    try {
      await service.createRefundRequest(
        target.orderId,
        {
          amount: refundAmount,
          ...(refundReason ? { reasonCode: refundReason } : {}),
          ...(refundDetail.trim() ? { reasonMessage: refundDetail.trim() } : {}),
        },
        refundIdempotencyKey,
      );
      setMessage(t("admin.orders.refund.success", "Refund request created and pending review."));
      setRefundTarget(null);
      setRefundAmount("");
      setRefundReason("");
      setRefundDetail("");
      setRefundIdempotencyKey("");
      setRefreshKey((current) => current + 1);
    } catch {
      setListError(t("admin.orders.refund.failed", "Refund request creation failed. Check the amount and order state."));
    } finally {
      setBusyId(null);
    }
  }

  const handleExport = () => {
    exportOrdersCsv(
      [
        t("admin.orders.column.order", "Order"),
        t("admin.orders.column.status", "Status"),
        t("admin.orders.column.amount", "Amount"),
        t("admin.orders.column.partner", "Partner"),
        t("admin.orders.column.createdAt", "Created at"),
      ],
      orders.map((order) => [
        order.orderSn ?? order.orderId,
        resolveOrderStatusLabel(order.status, order.statusName, t),
        order.totalAmount,
        order.partnerName ?? order.partnerId ?? "",
        order.createdAt ?? "",
      ]),
    );
  };

  useEffect(() => setPageInput(String(page)), [page]);

  const copyValue = async (key: string, value: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setCopiedField(key);
      window.setTimeout(() => setCopiedField((current) => (current === key ? null : current)), 1500);
    } catch {
      // Clipboard unavailable (non-secure context); keep the value visible.
    }
  };

  const copyButton = (key: string, value: string) => (
    <button
      aria-label={copiedField === key ? t("admin.orders.detail.copied", "Copied") : t("admin.orders.detail.copy", "Copy")}
      className="ml-2 inline-flex h-6 w-6 items-center justify-center rounded text-[var(--sdk-color-text-muted)] transition-colors hover:bg-[var(--sdk-color-surface-muted)] hover:text-[var(--sdk-color-text-primary)]"
      title={copiedField === key ? t("admin.orders.detail.copied", "Copied") : t("admin.orders.detail.copy", "Copy")}
      type="button"
      onClick={() => void copyValue(key, value)}
    >
      {copiedField === key ? <Check aria-hidden="true" className="h-3.5 w-3.5 text-[var(--sdk-color-state-success)]" /> : <Copy aria-hidden="true" className="h-3.5 w-3.5" />}
    </button>
  );

  const commitPageJump = () => {
    const next = Number(pageInput);
    if (Number.isInteger(next) && next >= 1 && next <= totalPages) {
      setPage(next);
    } else {
      setPageInput(String(page));
    }
  };

  async function mutateOrder(target: OrderMutation) {
    setBusyId(target.orderId);
    setMessage(null);
    setListError(null);
    try {
      const reason = mutationDetail.trim() || mutationReason;
      if (target.action === "cancel") {
        await service.cancelOrder(target.orderId, {
          reason: reason || undefined,
          cancelType: mutationReason || undefined,
        }, mutationIdempotencyKey);
      } else {
        await service.closeOrder(target.orderId, {
          reason: reason || undefined,
          closeType: mutationReason || undefined,
        }, mutationIdempotencyKey);
      }
      setMessage(t(
        target.action === "cancel" ? "admin.orders.message.cancelled" : "admin.orders.message.closed",
        target.action === "cancel" ? "Order {{label}} has been cancelled." : "Order {{label}} has been closed.",
        { label: target.orderLabel },
      ));
      setRefreshKey((current) => current + 1);
      setMutationIdempotencyKey("");
    } catch {
      setListError(t("admin.orders.message.actionFailed", "Operation failed. Check commerce.orders.manage permission and the current order status."));
    } finally {
      setBusyId(null);
      setMutationTarget(null);
    }
  }

  async function submitPaymentConfirmation(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!paymentTarget) return;
    const target = paymentTarget;
    const requestNo = paymentRequestNo.trim();
    if (!requestNo) return;
    setBusyId(target.orderId);
    setMessage(null);
    setListError(null);
    try {
      await service.confirmOrderPayment(target.orderId, requestNo, paymentIdempotencyKey);
      setMessage(t("admin.orders.confirmPayment.success", "Payment reconciliation for order {{label}} has been confirmed.", {
        label: target.orderLabel,
      }));
      setPaymentTarget(null);
      setPaymentRequestNo("");
      setPaymentIdempotencyKey("");
      setRefreshKey((current) => current + 1);
    } catch {
      setListError(t("admin.orders.confirmPayment.failed", "Payment confirmation failed. Check permissions and the payment state."));
    } finally {
      setBusyId(null);
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
          label={resolveOrderStatusLabel(order.status, order.statusName, t)}
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
          {formatAmount(order.totalAmount, locale)}
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
          {formatTimestamp(order.createdAt, locale)}
        </time>
      ),
    },
  ], [locale, t]);

  const activeFilterCount = Number(Boolean(statusFilter)) + Number(Boolean(searchQuery)) + Number(Boolean(createdFromFilter)) + Number(Boolean(createdToFilter));

  return (
    <div aria-label={t("admin.orders.title", "Order supervision")} className="flex min-h-0 flex-1 flex-col">
      <form onSubmit={applyFilters}>
        <FilterBar>
          <FilterBarSection wrap={false}>
            <label className="flex shrink-0 items-center gap-2 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span className="whitespace-nowrap">{t("admin.orders.filter.status", "Status")}</span>
              <select
                aria-label={t("admin.orders.filter.status", "Status")}
                className="h-9 w-36 rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-2.5 text-sm text-[var(--sdk-color-text-primary)] outline-none transition-colors focus-visible:border-[var(--sdk-color-brand-primary)]"
                value={draftStatus}
                onChange={(event) => setDraftStatus(event.target.value)}
              >
                <option value="">{t("admin.orders.filter.statusAll", "All statuses")}</option>
                {STATUS_FILTER_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>{t(option.labelKey, option.value)}</option>
                ))}
              </select>
            </label>
            <label className="flex shrink-0 items-center gap-2 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span className="whitespace-nowrap">{t("admin.orders.filter.orderNo", "Order no.")}</span>
              <div className="w-44">
                <Input
                  placeholder={t("admin.orders.filter.orderNoPlaceholder", "Exact order number")}
                  value={draftOrderNo}
                  onChange={(event) => setDraftOrderNo(event.target.value)}
                />
              </div>
            </label>
            <label className="flex shrink-0 items-center gap-2 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span className="whitespace-nowrap">{t("admin.orders.filter.search", "Search")}</span>
              <div className="relative w-56">
                <Search aria-hidden="true" className="pointer-events-none absolute left-3 inset-y-0 my-auto h-4 w-4 text-[var(--sdk-color-text-muted)]" />
                <Input
                  className="pl-9"
                  placeholder={t("admin.orders.filter.searchPlaceholder", "Order no., subject or reference")}
                  value={draftQuery}
                  onChange={(event) => setDraftQuery(event.target.value)}
                />
              </div>
            </label>
            <label className="flex shrink-0 items-center gap-2 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span className="whitespace-nowrap">{t("admin.orders.filter.createdAt", "Created at")}</span>
              <button
                className="whitespace-nowrap text-xs font-medium text-[var(--sdk-color-brand-primary)] transition-colors hover:underline"
                type="button"
                onClick={() => applyQuickRange(7)}
              >
                {t("admin.orders.filter.last7Days", "Last 7 days")}
              </button>
              <button
                className="whitespace-nowrap text-xs font-medium text-[var(--sdk-color-brand-primary)] transition-colors hover:underline"
                type="button"
                onClick={() => applyQuickRange(30)}
              >
                {t("admin.orders.filter.last30Days", "Last 30 days")}
              </button>
              <input
                aria-label={t("admin.orders.filter.createdFrom", "Created from")}
                className="h-9 w-36 rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-2 text-sm text-[var(--sdk-color-text-primary)] outline-none transition-colors focus-visible:border-[var(--sdk-color-brand-primary)]"
                max={draftCreatedTo || undefined}
                type="date"
                value={draftCreatedFrom}
                onChange={(event) => setDraftCreatedFrom(event.target.value)}
              />
              <span className="whitespace-nowrap text-[var(--sdk-color-text-muted)]">{t("admin.orders.filter.rangeTo", "to")}</span>
              <input
                aria-label={t("admin.orders.filter.createdTo", "Created to")}
                className="h-9 w-36 rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-2 text-sm text-[var(--sdk-color-text-primary)] outline-none transition-colors focus-visible:border-[var(--sdk-color-brand-primary)]"
                min={draftCreatedFrom || undefined}
                type="date"
                value={draftCreatedTo}
                onChange={(event) => setDraftCreatedTo(event.target.value)}
              />
            </label>
          </FilterBarSection>
          <FilterBarActions>
            <Button aria-label={t("admin.orders.export", "Export")} disabled={loading} size="icon" title={t("admin.orders.export", "Export")} type="button" variant="outline" onClick={handleExport}>
              <Download aria-hidden="true" className="h-4 w-4" />
            </Button>
            <Button aria-label={t("admin.orders.refresh", "Refresh")} disabled={loading} size="icon" title={t("admin.orders.refresh", "Refresh")} type="button" variant="outline" onClick={() => setRefreshKey((current) => current + 1)}>
              <RefreshCw aria-hidden="true" className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
            </Button>
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
        className="min-h-0 flex-1"
        columns={columns}
        density="compact"
        emptyDescription={activeFilterCount ? t("admin.orders.table.filteredEmptyDescription", "No orders match. Adjust the filters and retry.") : t("admin.orders.table.emptyDescription", "Platform orders will be listed here.")}
        emptyTitle={t("admin.orders.table.emptyTitle", "No orders")}
        footer={(
          <div className="flex min-w-0 flex-1 flex-nowrap items-center justify-between gap-3">
            <span className="min-w-0 flex-1 truncate text-sm tabular-nums text-[var(--sdk-color-text-secondary)]">
              {t("admin.orders.table.pageRange", "{{from}}-{{to}} of {{total}} orders", { from: totalItems === 0 ? 0 : (page - 1) * pageSize + 1, to: Math.min(page * pageSize, totalItems), total: totalItems })}
            </span>
            <div className="flex shrink-0 flex-nowrap items-center gap-2">
              <label className="flex shrink-0 items-center gap-1.5 text-sm text-[var(--sdk-color-text-secondary)]">
                <span className="whitespace-nowrap">{t("admin.orders.table.pageSizeLabel", "Rows per page")}</span>
                <select
                  aria-label={t("admin.orders.table.pageSizeLabel", "Rows per page")}
                  className="h-9 w-16 rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-2 text-sm text-[var(--sdk-color-text-primary)] outline-none transition-colors focus-visible:border-[var(--sdk-color-brand-primary)]"
                  value={pageSize}
                  onChange={(event) => changePageSize(Number(event.target.value))}
                >
                  {[10, 20, 50, 100].map((option) => (
                    <option key={option} value={option}>{option}</option>
                  ))}
                </select>
              </label>
              <div className="grid shrink-0 grid-cols-[2.25rem_2.25rem_minmax(4.5rem,auto)_2.25rem_2.25rem] items-center gap-1">
                <Button
                  aria-label={t("admin.orders.table.firstPage", "First page")}
                  className="h-9 w-9 p-0"
                  disabled={page <= 1 || loading}
                  size="icon"
                  title={t("admin.orders.table.firstPage", "First page")}
                  type="button"
                  variant="outline"
                  onClick={() => setPage(1)}
                >
                  <ChevronsLeft aria-hidden="true" className="h-4 w-4" />
                </Button>
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
                <input
                  aria-label={t("admin.orders.gotoPage", "Go to page")}
                  className="h-9 w-12 rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] text-center text-sm tabular-nums text-[var(--sdk-color-text-primary)] outline-none transition-colors focus-visible:border-[var(--sdk-color-brand-primary)]"
                  inputMode="numeric"
                  value={pageInput}
                  onBlur={commitPageJump}
                  onChange={(event) => setPageInput(event.target.value.replace(/\D/g, ""))}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      commitPageJump();
                    }
                  }}
                />
                <span aria-hidden="true" className="whitespace-nowrap text-sm text-[var(--sdk-color-text-muted)]">
                  / {totalPages}
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
                <Button
                  aria-label={t("admin.orders.table.lastPage", "Last page")}
                  className="h-9 w-9 p-0"
                  disabled={page >= totalPages || loading}
                  size="icon"
                  title={t("admin.orders.table.lastPage", "Last page")}
                  type="button"
                  variant="outline"
                  onClick={() => setPage(totalPages)}
                >
                  <ChevronsRight aria-hidden="true" className="h-4 w-4" />
                </Button>
              </div>
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
                <Button disabled={Boolean(busyId)} size="sm" title={t("admin.orders.action.cancel", "Cancel")} type="button" variant="outline" onClick={() => {
                      setMutationTarget({ action: "cancel", orderId: order.orderId, orderLabel: order.orderSn || order.orderId });
                      setMutationIdempotencyKey(crypto.randomUUID());
                    }}>
                  <Ban aria-hidden="true" className="mr-1.5 h-4 w-4" />{t("admin.orders.action.cancel", "Cancel")}
                </Button>
                <Button disabled={Boolean(busyId)} size="sm" title={t("admin.orders.action.close", "Close")} type="button" variant="outline" onClick={() => {
                      setMutationTarget({ action: "close", orderId: order.orderId, orderLabel: order.orderSn || order.orderId });
                      setMutationIdempotencyKey(crypto.randomUUID());
                    }}>
                  <Archive aria-hidden="true" className="mr-1.5 h-4 w-4" />{t("admin.orders.action.close", "Close")}
                </Button>
              </>
            ) : null}
          </div>
        )}
        rowActionsLabel={t("admin.orders.action.detail", "Details")}
        rows={orders}
        slotProps={{
          surface: { className: "min-h-0 flex-1 flex flex-col" },
          viewport: { className: "min-h-0 flex-1" },
        }}
        stickyHeader
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
                  <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.orderSn", "Order no.")}</dt><dd className="mt-1 flex items-start break-all font-mono text-sm text-[var(--sdk-color-text-primary)]">{detail.orderSn}{copyButton("orderSn", detail.orderSn)}</dd></div>
                  <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.status", "Status")}</dt><dd className="mt-1"><StatusBadge label={resolveOrderStatusLabel(detail.status, detail.statusName, t)} showIcon status={detail.status} variant={resolveStatusVariant(detail.status)} /></dd></div>
                  <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.amount", "Amount")}</dt><dd className="mt-1 font-mono text-sm font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">{formatAmount(detail.totalAmount, locale)}</dd></div>
                  {detail.paidAmount ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.paidAmount", "Paid amount")}</dt><dd className="mt-1 font-mono text-sm tabular-nums text-[var(--sdk-color-text-primary)]">{formatAmount(detail.paidAmount, locale)}</dd></div> : null}
                  {detail.discountAmount ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.discountAmount", "Discount")}</dt><dd className="mt-1 font-mono text-sm tabular-nums text-[var(--sdk-color-text-primary)]">{formatAmount(detail.discountAmount, locale)}</dd></div> : null}
                  <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.quantity", "Quantity")}</dt><dd className="mt-1 text-sm tabular-nums text-[var(--sdk-color-text-primary)]">{detail.quantity}</dd></div>
                  {detail.paymentMethod ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.paymentMethod", "Payment method")}</dt><dd className="mt-1 text-sm text-[var(--sdk-color-text-primary)]">{detail.paymentMethod}</dd></div> : null}
                  {detail.outTradeNo ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.outTradeNo", "Out trade no.")}</dt><dd className="mt-1 flex items-start break-all font-mono text-sm text-[var(--sdk-color-text-primary)]">{detail.outTradeNo}{copyButton("outTradeNo", detail.outTradeNo)}</dd></div> : null}
                  {detail.transactionId ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.transactionId", "Transaction ID")}</dt><dd className="mt-1 flex items-start break-all font-mono text-sm text-[var(--sdk-color-text-primary)]">{detail.transactionId}{copyButton("transactionId", detail.transactionId)}</dd></div> : null}
                  {detail.payTime ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.payTime", "Paid at")}</dt><dd className="mt-1 text-sm text-[var(--sdk-color-text-primary)]">{formatTimestamp(detail.payTime, locale)}</dd></div> : null}
                  {detail.expireTime ? <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">{t("admin.orders.detail.expireTime", "Expires at")}</dt><dd className="mt-1 text-sm text-[var(--sdk-color-text-primary)]">{formatTimestamp(detail.expireTime, locale)}</dd></div> : null}
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
                              <td className="px-4 py-2.5 text-right font-mono tabular-nums text-[var(--sdk-color-text-secondary)]">{formatAmount(item.unitPrice, locale)}</td>
                              <td className="px-4 py-2.5 text-right tabular-nums text-[var(--sdk-color-text-secondary)]">{item.quantity}</td>
                              <td className="px-4 py-2.5 text-right font-mono font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">{formatAmount(item.totalAmount, locale)}</td>
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
                            <OrderLink
                              href={`/admin/trade/shipments?orderId=${detail.orderId}`}
                              title={t("admin.orders.detail.fulfillment.jump", "Open shipment list for this order")}
                            >
                              <p className="truncate font-mono text-sm font-semibold text-[var(--sdk-color-text-primary)] transition-colors hover:text-[var(--sdk-color-brand-primary)]">
                                {shipment.shipmentNo}
                              </p>
                            </OrderLink>
                            <p className="mt-0.5 truncate text-xs text-[var(--sdk-color-text-muted)]">
                              {[shipment.carrierCode, shipment.trackingNo].filter(Boolean).join(" · ") || shipment.fulfillmentId}
                            </p>
                          </div>
                          <StatusBadge label={resolveOrderStatusLabel(shipment.status, shipment.status, t)} showIcon status={shipment.status} variant={resolveStatusVariant(shipment.status)} />
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
                            {event.message || (
                              event.fromStatus
                                ? `${resolveOrderStatusLabel(event.fromStatus, undefined, t)} → ${resolveOrderStatusLabel(event.toStatus, undefined, t)}`
                                : resolveOrderStatusLabel(event.toStatus, undefined, t)
                            )}
                          </p>
                          <p className="mt-0.5 flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-[var(--sdk-color-text-muted)]">
                            <span>{resolveActorLabel(event.actorType, t)}</span>
                            {event.eventType ? <span className="font-mono">{event.eventType}</span> : null}
                            <time dateTime={event.createdAt}>{formatTimestamp(event.createdAt, locale)}</time>
                          </p>
                        </li>
                      ))}
                    </ol>
                  )}
                </section>
              </div>
            ) : null}
          </DrawerBody>
          <DrawerFooter>
            {capabilities.canManageOrders && detail ? (
              <Button
                disabled={Boolean(busyId)}
                type="button"
                variant="outline"
                onClick={() => openRefund({ orderId: detail.orderId, orderLabel: detail.orderSn, paidAmount: detail.paidAmount })}
              >
                <Undo2 aria-hidden="true" className="mr-1.5 h-4 w-4" />
                {t("admin.orders.action.refund", "Refund")}
              </Button>
            ) : null}
            {capabilities.canConfirmPayment && detail ? (
              <Button
                disabled={Boolean(busyId)}
                type="button"
                variant="outline"
                onClick={() => {
                  setPaymentRequestNo("");
                  setPaymentTarget({ orderId: detail.orderId, orderLabel: detail.orderSn });
                  setPaymentIdempotencyKey(crypto.randomUUID());
                }}
              >
                <BadgeCheck aria-hidden="true" className="mr-1.5 h-4 w-4" />
                {t("admin.orders.action.confirmPayment", "Confirm payment")}
              </Button>
            ) : null}
            <Button onClick={() => setSelectedId(null)} type="button" variant="secondary">{t("admin.orders.detail.close", "Close")}</Button>
          </DrawerFooter>
        </DrawerContent>
      </Drawer>

      <Modal open={Boolean(mutationTarget)} onOpenChange={(open) => { if (!open && !busyId) { setMutationTarget(null); setMutationReason(""); setMutationDetail(""); setMutationIdempotencyKey(""); } }}>
        <ModalContent size="sm">
          <ModalHeader>
            <ModalTitle>
              {mutationTarget?.action === "cancel" ? t("admin.orders.confirm.cancelTitle", "Cancel order") : t("admin.orders.confirm.closeTitle", "Close order")}
            </ModalTitle>
            <ModalDescription>{mutationTarget?.orderLabel}</ModalDescription>
          </ModalHeader>
          <form onSubmit={(event) => { event.preventDefault(); if (mutationTarget) void mutateOrder(mutationTarget); }}>
            <ModalBody>
              <div className="space-y-4">
                <p className="text-sm text-[var(--sdk-color-text-secondary)]">
                  {t(
                    mutationTarget?.action === "cancel" ? "admin.orders.confirm.cancelDescription" : "admin.orders.confirm.closeDescription",
                    mutationTarget?.action === "cancel" ? "Cancelling order {{label}} affects downstream fulfillment." : "Closing order {{label}} affects downstream fulfillment.",
                    { label: mutationTarget?.orderLabel ?? "" },
                  )}
                </p>
                <fieldset>
                  <legend className="text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                    {t("admin.orders.mutation.reason", "Cancellation reason")}
                  </legend>
                  <div className="mt-2 flex flex-wrap gap-2">
                    {QUICK_MUTATION_REASONS[mutationTarget?.action ?? "cancel"].map((option) => (
                      <button
                        className={
                          option.value === mutationReason
                            ? "rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-brand-primary)] bg-[var(--sdk-color-brand-primary-soft)] px-3 py-1.5 text-xs font-medium text-[var(--sdk-color-brand-primary)]"
                            : "rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-3 py-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)] transition-colors hover:border-[var(--sdk-color-border-focus)]"
                        }
                        key={option.value}
                        type="button"
                        onClick={() => setMutationReason(option.value === mutationReason ? "" : option.value)}
                      >
                        {t(option.labelKey, option.value)}
                      </button>
                    ))}
                  </div>
                </fieldset>
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.orders.mutation.detailPlaceholder", "Additional details (optional)")}</span>
                  <Input
                    placeholder={t("admin.orders.mutation.reasonPlaceholder", "Select or enter a reason")}
                    value={mutationDetail}
                    onChange={(event) => setMutationDetail(event.target.value)}
                  />
                </label>
              </div>
            </ModalBody>
            <ModalFooter>
              <Button disabled={Boolean(busyId)} type="button" variant="secondary" onClick={() => { setMutationTarget(null); setMutationReason(""); setMutationDetail(""); }}>
                {t("admin.orders.detail.close", "Close")}
              </Button>
              <Button disabled={Boolean(busyId)} loading={Boolean(busyId)} type="submit">
                {mutationTarget?.action === "cancel" ? t("admin.orders.confirm.cancelLabel", "Confirm cancel") : t("admin.orders.confirm.closeLabel", "Confirm close")}
              </Button>
            </ModalFooter>
          </form>
        </ModalContent>
      </Modal>

      <Modal open={Boolean(paymentTarget)} onOpenChange={(open) => { if (!open && !busyId) { setPaymentTarget(null); setPaymentRequestNo(""); setPaymentIdempotencyKey(""); } }}>
        <ModalContent size="sm">
          <ModalHeader>
            <ModalTitle>{t("admin.orders.confirmPayment.title", "Confirm payment reconciliation")}</ModalTitle>
            <ModalDescription>{paymentTarget?.orderLabel}</ModalDescription>
          </ModalHeader>
          <form onSubmit={submitPaymentConfirmation}>
            <ModalBody>
              <div className="space-y-4">
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.orders.confirmPayment.requestNo", "Payment request no.")}</span>
                  <Input
                    autoFocus
                    placeholder={t("admin.orders.confirmPayment.requestNoPlaceholder", "Provider payment request number")}
                    required
                    value={paymentRequestNo}
                    onChange={(event) => setPaymentRequestNo(event.target.value)}
                  />
                </label>
                <p className="text-sm text-[var(--sdk-color-text-secondary)]">
                  {t("admin.orders.confirmPayment.description", "Enter the provider payment request number to reconcile and run the settlement flow.")}
                </p>
              </div>
            </ModalBody>
            <ModalFooter>
              <Button disabled={Boolean(busyId)} type="button" variant="secondary" onClick={() => { setPaymentTarget(null); setPaymentRequestNo(""); }}>
                {t("admin.orders.detail.close", "Close")}
              </Button>
              <Button disabled={Boolean(busyId)} loading={Boolean(busyId)} type="submit">
                {t("admin.orders.confirmPayment.confirm", "Confirm reconciliation")}
              </Button>
            </ModalFooter>
          </form>
        </ModalContent>
      </Modal>

      <Modal open={Boolean(refundTarget)} onOpenChange={(open) => { if (!open && !busyId) { setRefundTarget(null); setRefundIdempotencyKey(""); } }}>
        <ModalContent size="sm">
          <ModalHeader>
            <ModalTitle>{t("admin.orders.refund.title", "Create refund")}</ModalTitle>
            <ModalDescription>{refundTarget?.orderLabel}</ModalDescription>
          </ModalHeader>
          <form onSubmit={(event) => { event.preventDefault(); void submitRefund(); }}>
            <ModalBody>
              <div className="space-y-4">
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.orders.refund.amount", "Refund amount")}</span>
                  <div className="flex items-center gap-2">
                    <Input
                      aria-label={t("admin.orders.refund.amount", "Refund amount")}
                      inputMode="decimal"
                      placeholder={t("admin.orders.refund.amountPlaceholder", "0.00")}
                      required
                      value={refundAmount}
                      onChange={(event) => setRefundAmount(event.target.value)}
                    />
                    <Button disabled={busyId !== null} size="sm" type="button" variant="outline" onClick={() => setRefundAmount(refundTarget?.paidAmount ?? "")}>
                      {t("admin.orders.refund.fullAmount", "Full amount")}
                    </Button>
                  </div>
                  <p className="text-xs text-[var(--sdk-color-text-muted)]">
                    {t("admin.orders.refund.amountHint", "Enter less than the paid amount for a partial refund; full refund returns {{amount}}.", { amount: formatAmount(refundTarget?.paidAmount, locale) })}
                  </p>
                </label>
                <fieldset>
                  <legend className="text-xs font-medium text-[var(--sdk-color-text-secondary)]">{t("admin.orders.refund.reason", "Refund reason")}</legend>
                  <div className="mt-2 flex flex-wrap gap-2">
                    {REFUND_REASON_OPTIONS.map((option) => (
                      <button
                        className={
                          option.value === refundReason
                            ? "rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-brand-primary)] bg-[var(--sdk-color-brand-primary-soft)] px-3 py-1.5 text-xs font-medium text-[var(--sdk-color-brand-primary)]"
                            : "rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-3 py-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)] transition-colors hover:border-[var(--sdk-color-border-focus)]"
                        }
                        key={option.value}
                        type="button"
                        onClick={() => setRefundReason(option.value === refundReason ? "" : option.value)}
                      >
                        {t(option.labelKey, option.value)}
                      </button>
                    ))}
                  </div>
                </fieldset>
                <label className="block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
                  <span>{t("admin.orders.refund.detailPlaceholder", "Additional details (optional)")}</span>
                  <Input
                    value={refundDetail}
                    onChange={(event) => setRefundDetail(event.target.value)}
                  />
                </label>
              </div>
            </ModalBody>
            <ModalFooter>
              <Button disabled={busyId !== null} type="button" variant="secondary" onClick={() => { setRefundTarget(null); setRefundIdempotencyKey(""); }}>
                {t("admin.orders.detail.close", "Close")}
              </Button>
              <Button disabled={busyId !== null} loading={busyId !== null} type="submit">
                {t("admin.orders.refund.title", "Create refund")}
              </Button>
            </ModalFooter>
          </form>
        </ModalContent>
      </Modal>
    </div>
  );
}
