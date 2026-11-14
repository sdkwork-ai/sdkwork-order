import { useSdkworkOrderIntl } from "../order-intl";
import {
  Button,
  DetailDrawer,
  DetailDrawerMetric,
  DetailDrawerMetrics,
  DetailDrawerSection,
} from "@sdkwork/ui-pc-react";
import type { SdkworkOrderController } from "../order-controller";
import { useSdkworkOrderControllerState } from "../order-controller";

export interface SdkworkOrderDetailDrawerProps {
  controller: SdkworkOrderController;
}

export function SdkworkOrderDetailDrawer({
  controller,
}: SdkworkOrderDetailDrawerProps) {
  const state = useSdkworkOrderControllerState(controller);
  const detail = state.detail;
  const {
    copy,
    formatCurrencyCny,
    formatDetailSummary,
    formatItemMeta,
    formatPaymentMethod,
    formatStatus,
    formatSubject,
    formatTimelineLabel,
    formatTimestamp,
  } = useSdkworkOrderIntl();

  const canPay = detail?.status === "pending-payment";
  const canCancel = detail?.status === "pending-payment";

  return (
    <DetailDrawer
      description={detail?.subject ? formatSubject(detail.subject) : copy.detail.description}
      footer={(
        <div className="flex flex-wrap justify-end gap-3">
          {canCancel ? (
            <Button
              disabled={state.isMutating}
              onClick={() => {
                if (!detail) {
                  return;
                }
                void controller.cancelOrder({ orderId: detail.id });
              }}
              type="button"
              variant="ghost"
            >
              {copy.actions.cancel}
            </Button>
          ) : null}
          {canPay ? (
            <Button
              disabled={state.isMutating}
              onClick={() => {
                if (!detail) {
                  return;
                }
                void controller.payOrder({
                  orderId: detail.id,
                  paymentMethod: detail.paymentMethod || "balance",
                });
              }}
              type="button"
            >
              {copy.actions.pay}
            </Button>
          ) : null}
          <Button onClick={() => controller.closeDetail()} type="button" variant="ghost">
            {copy.actions.close}
          </Button>
        </div>
      )}
      onOpenChange={(open) => {
        if (!open) {
          controller.closeDetail();
        }
      }}
      open={state.isDetailOpen}
      summary={detail ? formatDetailSummary(detail.id) : copy.detail.loading}
      title={copy.detail.title}
    >
      {state.isDetailLoading || !detail ? (
        <div className="text-sm text-[var(--sdk-color-text-secondary)]">{copy.detail.loading}</div>
      ) : (
        <>
          <DetailDrawerMetrics columns={3}>
            <DetailDrawerMetric label={copy.detail.totalAmount} value={formatCurrencyCny(detail.totalAmountCny)} />
            <DetailDrawerMetric label={copy.detail.paidAmount} value={formatCurrencyCny(detail.paidAmountCny)} />
            <DetailDrawerMetric
              label={copy.detail.status}
              tone={detail.status === "pending-payment" ? "warning" : "default"}
              value={formatStatus(detail.status, detail.statusLabel)}
            />
          </DetailDrawerMetrics>

          <DetailDrawerSection description={copy.overview.description} title={copy.overview.title}>
            <div className="grid gap-3 text-sm text-[var(--sdk-color-text-secondary)] sm:grid-cols-2">
              <div>{copy.overview.orderSn}: {detail.orderSn || copy.common.emptyValue}</div>
              <div>{copy.overview.outTradeNo}: {detail.outTradeNo || copy.common.emptyValue}</div>
              <div>{copy.overview.paymentMethod}: {formatPaymentMethod(detail.paymentMethod)}</div>
              <div>{copy.overview.transactionId}: {detail.transactionId || copy.common.emptyValue}</div>
              <div>{copy.overview.createdAt}: {formatTimestamp(detail.createdAt)}</div>
              <div>{copy.overview.paidAt}: {formatTimestamp(detail.payTime)}</div>
            </div>
          </DetailDrawerSection>

          <DetailDrawerSection description={copy.items.description} title={copy.items.title}>
            <div className="space-y-3">
              {detail.items.length === 0 ? (
                <div className="text-sm text-[var(--sdk-color-text-secondary)]">{copy.items.empty}</div>
              ) : detail.items.map((item) => (
                <div
                  className="rounded-[1rem] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel-muted)] px-4 py-3"
                  key={item.id}
                >
                  <div className="text-sm font-semibold text-[var(--sdk-color-text-primary)]">{item.name}</div>
                  <div className="mt-1 text-sm text-[var(--sdk-color-text-secondary)]">
                    {formatItemMeta(item.quantity, item.totalAmountCny)}
                  </div>
                </div>
              ))}
            </div>
          </DetailDrawerSection>

          <DetailDrawerSection description={copy.timeline.description} title={copy.timeline.title}>
            <div className="space-y-3">
              {detail.timeline.length === 0 ? (
                <div className="text-sm text-[var(--sdk-color-text-secondary)]">{copy.timeline.empty}</div>
              ) : detail.timeline.map((event, index) => (
                <div
                  className="rounded-[1rem] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel-muted)] px-4 py-3"
                  key={`${event.label}-${index}`}
                >
                  <div className="text-sm font-semibold text-[var(--sdk-color-text-primary)]">{formatTimelineLabel(event.label)}</div>
                  <div className="mt-1 text-sm text-[var(--sdk-color-text-secondary)]">
                    {formatTimestamp(event.occurredAt)}
                  </div>
                </div>
              ))}
            </div>
          </DetailDrawerSection>
        </>
      )}
    </DetailDrawer>
  );
}
