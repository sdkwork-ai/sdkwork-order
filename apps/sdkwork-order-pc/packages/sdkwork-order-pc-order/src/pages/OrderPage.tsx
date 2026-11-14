import { useEffect } from "react";
import {
  BadgeCheck,
  Clock3,
  ReceiptText,
} from "lucide-react";
import { Button, EmptyState, LoadingBlock, StatusNotice } from "@sdkwork/ui-pc-react";
import type { SdkworkOrderMessagesOverrides } from "../order-copy";
import type { SdkworkOrderController } from "../order-controller";
import {
  useSdkworkOrderController,
  useSdkworkOrderControllerState,
} from "../order-controller";
import {
  createSdkworkOrderBackdropStyle,
  createSdkworkOrderGlassStyle,
  createSdkworkOrderHeroStyle,
  createSdkworkOrderHeroTextStyle,
  createSdkworkOrderPanelStyle,
  createSdkworkOrderToneStyle,
} from "../order-appearance";
import {
  SdkworkOrderIntlProvider,
  useSdkworkOrderIntl,
} from "../order-intl";
import { SdkworkOrderDetailDrawer } from "../components/order-detail-drawer";
import { SdkworkOrderPagination } from "../components/order-pagination";
import { SdkworkOrderStatGrid } from "../components/order-stat-grid";

export interface SdkworkOrderPageProps {
  controller?: SdkworkOrderController;
  locale?: string | null;
  messages?: SdkworkOrderMessagesOverrides;
}

interface SdkworkOrderPageContentProps {
  controller?: SdkworkOrderController;
  locale?: string | null;
  messages?: SdkworkOrderMessagesOverrides;
}

function resolveStatusTone(status: string) {
  if (status === "pending-payment") {
    return "warning" as const;
  }

  if (status === "paid" || status === "completed") {
    return "success" as const;
  }

  if (status === "cancelled" || status === "expired") {
    return "danger" as const;
  }

  return "neutral" as const;
}

function SdkworkOrderPageContent({
  controller: controllerProp,
  locale,
  messages,
}: SdkworkOrderPageContentProps) {
  const controller = useSdkworkOrderController(controllerProp, {
    locale,
    messages,
  });
  const state = useSdkworkOrderControllerState(controller);
  const {
    copy,
    formatCurrencyCny,
    formatFilter,
    formatStatus,
    formatSubject,
    formatTimestamp,
  } = useSdkworkOrderIntl();
  const filters = ["all", "pending-payment", "paid", "completed", "cancelled"] as const;
  const heroHighlights = [
    {
      icon: ReceiptText,
      label: copy.stats.totalOrders,
      tone: "brand" as const,
      value: state.dashboard.statistics.totalOrders,
    },
    {
      icon: Clock3,
      label: copy.stats.pendingPayment,
      tone: "warning" as const,
      value: state.dashboard.statistics.pendingPayment,
    },
    {
      icon: BadgeCheck,
      label: copy.stats.completed,
      tone: "success" as const,
      value: state.dashboard.statistics.completed,
    },
  ];

  useEffect(() => {
    if (!state.isBootstrapped && !state.isLoading && !state.lastError) {
      void controller.bootstrap().catch(() => undefined);
    }
  }, [controller, state.isBootstrapped, state.isLoading, state.lastError]);

  return (
    <div className="relative h-full overflow-y-auto">
      <div
        className="pointer-events-none absolute inset-x-0 top-0 h-72"
        style={createSdkworkOrderBackdropStyle()}
      />

      <div className="relative px-4 py-4 sm:px-5 sm:py-5">
        <div className="mx-auto max-w-[88rem] space-y-5">
          <section
            className="overflow-hidden rounded-[2rem] border px-6 py-7 shadow-[var(--sdk-shadow-lg)]"
            style={{
              ...createSdkworkOrderHeroStyle(),
              ...createSdkworkOrderHeroTextStyle(),
            }}
          >
            <div className="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
              <div className="max-w-3xl">
                <div
                  className="inline-flex items-center gap-2 rounded-full border px-3 py-1 text-[0.7rem] font-semibold uppercase tracking-[0.18em] shadow-[var(--sdk-shadow-soft)]"
                  style={createSdkworkOrderToneStyle("accent", {
                    backgroundWeight: 16,
                    borderWeight: 26,
                  })}
                >
                  <ReceiptText className="h-3.5 w-3.5" />
                  {copy.page.eyebrow}
                </div>
                <h1 className="mt-4 text-4xl font-semibold tracking-tight">{copy.page.title}</h1>
                <p className="mt-3 text-sm leading-7" style={createSdkworkOrderHeroTextStyle("muted")}>
                  {copy.page.description}
                </p>
                <div className="mt-4">
                  <Button
                    disabled={state.isLoading}
                    onClick={() => {
                      void controller.refresh();
                    }}
                    type="button"
                    variant="ghost"
                  >
                    {copy.actions.refresh}
                  </Button>
                </div>
              </div>

              <div
                className="inline-flex flex-wrap gap-2 rounded-[1.25rem] border p-2 shadow-[var(--sdk-shadow-sm)] backdrop-blur-xl"
                style={createSdkworkOrderGlassStyle("neutral", {
                  backgroundWeight: 12,
                  borderWeight: 22,
                })}
              >
                {filters.map((filter) => (
                  <Button
                    className="rounded-full px-4"
                    key={filter}
                    onClick={() => controller.setFilter(filter)}
                    size="sm"
                    type="button"
                    variant={state.activeFilter === filter ? "secondary" : "ghost"}
                  >
                    {formatFilter(filter)}
                  </Button>
                ))}
              </div>
            </div>

            <div className="mt-8 grid gap-4 lg:grid-cols-3">
              {heroHighlights.map((highlight) => {
                const Icon = highlight.icon;

                return (
                  <div
                    className="rounded-[1.5rem] border p-5 shadow-[var(--sdk-shadow-sm)] backdrop-blur-xl"
                    key={highlight.label}
                    style={createSdkworkOrderGlassStyle(highlight.tone, {
                      backgroundWeight: 14,
                      borderWeight: 26,
                    })}
                  >
                    <div className="flex items-center justify-between gap-4">
                      <div>
                        <div className="text-sm" style={createSdkworkOrderHeroTextStyle("subtle")}>{highlight.label}</div>
                        <div className="mt-3 text-4xl font-semibold tracking-tight">
                          {highlight.value}
                        </div>
                      </div>
                      <div
                        className="flex h-12 w-12 items-center justify-center rounded-[1rem] border"
                        style={createSdkworkOrderToneStyle(highlight.tone, {
                          backgroundWeight: 20,
                          borderWeight: 34,
                        })}
                      >
                        <Icon className="h-5 w-5" />
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </section>

          <SdkworkOrderStatGrid statistics={state.dashboard.statistics} />

          {state.isLoading && !state.isBootstrapped ? <LoadingBlock label={copy.page.loading} /> : null}

          {state.lastError ? (
            <StatusNotice title={copy.page.errorTitle} tone="danger">
              {state.lastError}
            </StatusNotice>
          ) : null}

          <section
            className="rounded-[1.5rem] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] shadow-[var(--sdk-shadow-sm)]"
            style={createSdkworkOrderPanelStyle("neutral", {
              backgroundWeight: 6,
              borderWeight: 16,
            })}
          >
            <div className="border-b border-[var(--sdk-color-border-subtle)] px-6 py-5">
              <div className="text-xs font-semibold uppercase tracking-[0.18em] text-[var(--sdk-color-text-muted)]">{copy.views.eyebrow}</div>
              <h2 className="mt-2 text-xl font-semibold text-[var(--sdk-color-text-primary)]">{copy.views.title}</h2>
            </div>

            <div className="divide-y divide-[var(--sdk-color-border-subtle)]">
              {state.visibleOrders.length === 0 ? (
                <div className="px-6 py-10">
                  <EmptyState
                    description={copy.views.empty}
                    title={copy.views.title}
                  />
                </div>
              ) : state.visibleOrders.map((order) => (
                <article className="flex flex-col gap-4 px-6 py-5 lg:flex-row lg:items-center lg:justify-between" key={order.id}>
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-3">
                      <div className="text-base font-semibold text-[var(--sdk-color-text-primary)]">{formatSubject(order.subject)}</div>
                      <span
                        className="rounded-full border px-2.5 py-1 text-[0.68rem] font-semibold uppercase tracking-[0.16em]"
                        style={createSdkworkOrderToneStyle(resolveStatusTone(order.status))}
                      >
                        {formatStatus(order.status, order.statusLabel)}
                      </span>
                    </div>
                    <div className="mt-2 flex flex-wrap items-center gap-3 text-sm text-[var(--sdk-color-text-secondary)]">
                      <span>{formatTimestamp(order.createdAt)}</span>
                      <span>{formatCurrencyCny(order.totalAmountCny)}</span>
                    </div>
                  </div>
                  <div className="flex flex-wrap gap-3">
                    <Button onClick={() => void controller.openDetail(order.id)} type="button" variant="outline">
                      {copy.actions.viewDetails}
                    </Button>
                  </div>
                </article>
              ))}
            </div>

            <SdkworkOrderPagination controller={controller} />
          </section>
        </div>
      </div>

      <SdkworkOrderDetailDrawer controller={controller} />
    </div>
  );
}

export function SdkworkOrderPage({
  locale,
  messages,
  ...props
}: SdkworkOrderPageProps) {
  const content = (
    <SdkworkOrderPageContent
      {...props}
      locale={locale}
      messages={messages}
    />
  );

  if (locale || messages) {
    return (
      <SdkworkOrderIntlProvider locale={locale} messages={messages}>
        {content}
      </SdkworkOrderIntlProvider>
    );
  }

  return content;
}
