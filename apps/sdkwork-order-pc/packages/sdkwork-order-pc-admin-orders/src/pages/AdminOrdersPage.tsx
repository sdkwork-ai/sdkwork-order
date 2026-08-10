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
  type OrderSummary,
} from "@sdkwork/order-pc-admin-core";
import { createOrderAdminService, type OrderAdminService } from "../order-admin-service";

const DEFAULT_PAGE_SIZE = 20;

type OrderMutation = {
  action: "cancel" | "close";
  orderId: string;
  orderLabel: string;
};

export interface SdkworkOrderAdminOrdersPageProps {
  capabilities: SdkworkOrderAdminCapabilities;
  service?: OrderAdminService;
}

export interface SdkworkOrderAdminCapabilities {
  canManageOrders: boolean;
}

function formatTimestamp(value?: string): string {
  if (!value) return "--";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN");
}

function resolveStatusVariant(status: string): StatusBadgeVariant {
  const normalized = status.toLowerCase();
  if (["paid", "completed", "succeeded", "success"].includes(normalized)) return "success";
  if (["pending", "pending_payment", "processing", "shipping"].includes(normalized)) return "warning";
  if (["failed", "cancelled", "canceled", "rejected"].includes(normalized)) return "danger";
  if (["closed", "refunded", "archived"].includes(normalized)) return "secondary";
  return "default";
}

export function SdkworkOrderAdminOrdersPage({ capabilities, service: injectedService }: SdkworkOrderAdminOrdersPageProps) {
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
      setListError("订单列表加载失败，请检查 commerce.orders.read 权限与网络连接。");
      setOrders([]);
      setTotalItems(0);
    }).finally(() => {
      if (active) setLoading(false);
    });
    return () => { active = false; };
  }, [page, refreshKey, searchQuery, service, statusFilter]);

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      setDetailError(null);
      return;
    }
    let active = true;
    setDetailLoading(true);
    setDetailError(null);
    void service.getOrder(selectedId)
      .then((value) => { if (active) setDetail(value); })
      .catch(() => {
        if (!active) return;
        setDetail(null);
        setDetailError("订单详情加载失败，请稍后重试。");
      })
      .finally(() => { if (active) setDetailLoading(false); });
    return () => { active = false; };
  }, [selectedId, service]);

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
      setMessage(`订单 ${target.orderLabel} 已${target.action === "cancel" ? "取消" : "关闭"}。`);
      setRefreshKey((current) => current + 1);
    } catch {
      setListError("操作失败，请检查 commerce.orders.manage 权限与订单当前状态。");
    } finally {
      setBusyId(null);
      setMutationTarget(null);
    }
  }

  const columns = useMemo<DataTableColumn<OrderSummary>[]>(() => [
    {
      id: "order",
      header: "订单",
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
      header: "状态",
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
      header: "金额",
      width: "20%",
      cell: (order) => (
        <span className="font-mono text-sm font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">
          {order.totalAmount}
        </span>
      ),
    },
    {
      id: "createdAt",
      header: "创建时间",
      width: "28%",
      cell: (order) => (
        <time className="whitespace-nowrap text-sm text-[var(--sdk-color-text-secondary)]" dateTime={order.createdAt}>
          {formatTimestamp(order.createdAt)}
        </time>
      ),
    },
  ], []);

  const activeFilterCount = Number(Boolean(statusFilter)) + Number(Boolean(searchQuery));

  return (
    <div aria-label="订单监管" className="space-y-4">
      <form onSubmit={applyFilters}>
        <FilterBar
          summary={activeFilterCount ? `已应用 ${activeFilterCount} 个筛选条件` : undefined}
          title="筛选条件"
        >
          <FilterBarSection>
            <label className="min-w-[12rem] flex-1 space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span>状态</span>
              <Input
                placeholder="例如 pending_payment"
                value={draftStatus}
                onChange={(event) => setDraftStatus(event.target.value)}
              />
            </label>
            <label className="min-w-[16rem] flex-[1.5] space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]">
              <span>搜索</span>
              <div className="relative">
                <Search aria-hidden="true" className="pointer-events-none absolute left-3 inset-y-0 my-auto h-4 w-4 text-[var(--sdk-color-text-muted)]" />
                <Input
                  className="pl-9"
                  placeholder="订单号、主题或关联标识"
                  value={draftQuery}
                  onChange={(event) => setDraftQuery(event.target.value)}
                />
              </div>
            </label>
          </FilterBarSection>
          <FilterBarActions>
            <Button disabled={loading} type="button" variant="outline" onClick={resetFilters}>
              <RotateCcw aria-hidden="true" className="mr-2 h-4 w-4" />
              重置
            </Button>
            <Button disabled={loading} type="submit">
              <Search aria-hidden="true" className="mr-2 h-4 w-4" />
              查询
            </Button>
          </FilterBarActions>
        </FilterBar>
      </form>

      {listError ? <StatusNotice tone="danger">{listError}</StatusNotice> : null}
      {message ? <StatusNotice tone="success">{message}</StatusNotice> : null}

      <DataTable
        columns={columns}
        density="compact"
        description={`当前显示 ${orders.length} 条，共 ${totalItems} 条订单`}
        emptyDescription={activeFilterCount ? "请调整筛选条件后重试。" : "平台订单将在这里集中展示。"}
        emptyTitle={activeFilterCount ? "没有符合条件的订单" : "暂无订单"}
        footer={(
          <div className="flex min-w-0 flex-1 flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <span className="min-w-0 truncate text-sm tabular-nums text-[var(--sdk-color-text-secondary)]">
              共 {totalItems.toLocaleString()} 条记录
            </span>
            <div className="grid w-full grid-cols-[2.25rem_minmax(4.5rem,1fr)_2.25rem] items-center gap-1 sm:w-auto sm:grid-cols-[2.25rem_minmax(4.5rem,auto)_2.25rem]">
              <Button
                aria-label="上一页"
                className="h-9 w-9 p-0"
                disabled={page <= 1 || loading}
                size="icon"
                title="上一页"
                type="button"
                variant="outline"
                onClick={() => setPage((value) => value - 1)}
              >
                <ChevronLeft aria-hidden="true" className="h-4 w-4" />
              </Button>
              <span
                aria-label={`第 ${page} 页，共 ${totalPages} 页`}
                className="flex h-9 items-center justify-center gap-2 rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-3 text-sm tabular-nums text-[var(--sdk-color-text-muted)]"
              >
                <strong className="font-semibold text-[var(--sdk-color-text-primary)]">{page}</strong>
                <span aria-hidden="true">/</span>
                <span aria-hidden="true">{totalPages}</span>
              </span>
              <Button
                aria-label="下一页"
                className="h-9 w-9 p-0"
                disabled={page >= totalPages || loading}
                size="icon"
                title="下一页"
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
        loadingLabel="正在加载订单..."
        onRowClick={(order) => setSelectedId(order.orderId)}
        rowActions={(order) => (
          <div className="flex items-center justify-end gap-1">
            <Button aria-label={`查看订单详情：${order.subject || order.orderSn || order.orderId}`} disabled={busyId === order.orderId} size="sm" title="查看详情" type="button" variant="ghost" onClick={() => setSelectedId(order.orderId)}>
              <Eye aria-hidden="true" className="mr-1.5 h-4 w-4" />详情
            </Button>
            {capabilities.canManageOrders ? (
              <>
                <Button disabled={Boolean(busyId)} size="sm" title="取消订单" type="button" variant="outline" onClick={() => setMutationTarget({ action: "cancel", orderId: order.orderId, orderLabel: order.orderSn || order.orderId })}>
                  <Ban aria-hidden="true" className="mr-1.5 h-4 w-4" />取消
                </Button>
                <Button disabled={Boolean(busyId)} size="sm" title="关闭订单" type="button" variant="outline" onClick={() => setMutationTarget({ action: "close", orderId: order.orderId, orderLabel: order.orderSn || order.orderId })}>
                  <Archive aria-hidden="true" className="mr-1.5 h-4 w-4" />关闭
                </Button>
              </>
            ) : null}
          </div>
        )}
        rowActionsLabel="操作"
        rows={orders}
        stickyHeader
        title="订单列表"
        toolbar={(
          <Button aria-label="刷新订单" disabled={loading} size="icon" title="刷新订单" type="button" variant="outline" onClick={() => setRefreshKey((current) => current + 1)}>
            <RefreshCw aria-hidden="true" className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        )}
      />

      <Drawer open={Boolean(selectedId)} onOpenChange={(open) => { if (!open) setSelectedId(null); }}>
        <DrawerContent size="md">
          <DrawerHeader>
            <DrawerTitle>订单详情</DrawerTitle>
            <DrawerDescription>{detail?.orderSn ?? selectedId}</DrawerDescription>
          </DrawerHeader>
          <DrawerBody>
            {detailLoading ? <LoadingBlock label="正在加载订单详情..." /> : null}
            {detailError ? <StatusNotice tone="danger">{detailError}</StatusNotice> : null}
            {detail ? (
              <dl className="grid grid-cols-1 gap-x-6 gap-y-5 sm:grid-cols-2">
                <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">订单号</dt><dd className="mt-1 break-all font-mono text-sm text-[var(--sdk-color-text-primary)]">{detail.orderSn}</dd></div>
                <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">状态</dt><dd className="mt-1"><StatusBadge label={detail.statusName || detail.status} showIcon status={detail.status} variant={resolveStatusVariant(detail.status)} /></dd></div>
                <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">金额</dt><dd className="mt-1 font-mono text-sm font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">{detail.totalAmount}</dd></div>
                <div><dt className="text-xs font-medium text-[var(--sdk-color-text-muted)]">数量</dt><dd className="mt-1 text-sm tabular-nums text-[var(--sdk-color-text-primary)]">{detail.quantity}</dd></div>
              </dl>
            ) : null}
          </DrawerBody>
          <DrawerFooter><Button onClick={() => setSelectedId(null)} type="button" variant="secondary">关闭</Button></DrawerFooter>
        </DrawerContent>
      </Drawer>

      <ConfirmDialog
        cancelLabel="返回"
        closeOnConfirm={false}
        confirmLabel={mutationTarget?.action === "cancel" ? "确认取消" : "确认关闭"}
        confirmLoading={Boolean(busyId)}
        description={mutationTarget ? `${mutationTarget.action === "cancel" ? "取消" : "关闭"}订单 ${mutationTarget.orderLabel} 后，后续履约流程将受到影响。` : undefined}
        onConfirm={() => { if (mutationTarget) void mutateOrder(mutationTarget); }}
        onOpenChange={(open) => { if (!open && !busyId) setMutationTarget(null); }}
        open={Boolean(mutationTarget)}
        title={mutationTarget?.action === "cancel" ? "取消订单" : "关闭订单"}
        tone="warning"
      />
    </div>
  );
}
