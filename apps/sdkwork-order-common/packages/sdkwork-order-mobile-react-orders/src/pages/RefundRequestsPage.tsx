import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";
import { Loader2, ReceiptText } from "lucide-react";
import { PageLayout, showToast } from "@sdkwork/ui-mobile-react";

import { OrderService, formatAmountCny } from "../services/OrderService";
import {
  REFUND_TARGET_ASSETS,
  REFUND_TARGET_ASSET_LABELS,
  isRefundTargetAsset,
  type CreateRefundRequestInput,
  type RefundRequestView,
  type RefundTargetAsset,
} from "../services/RefundTypes";
import { toUserErrorMessage } from "../services/errorMessage";

/**
 * Buyer refund surface: "my refund requests" list plus an inline
 * create-request form (deep-linkable through `?orderId=<id>` from the order
 * detail/center refund buttons). Reads and writes go through
 * `OrderService` only — pages never touch the SDK client directly.
 */

/** Readable status badge fallbacks when a host ships no `orders.refund_status_*` resource. */
const REFUND_STATUS_FALLBACKS: Readonly<Record<string, string>> = {
  requested: "待审核",
  processing: "处理中",
  approved: "已同意",
  rejected: "已拒绝",
  refunded: "已退款",
  paid_out: "已打款",
  completed: "已完成",
};

/** Tailwind badge classes per status family; unknown statuses render neutral. */
const REFUND_STATUS_BADGE_CLASSES: Readonly<Record<string, string>> = {
  requested: "bg-primary-blue/10 text-primary-blue",
  processing: "bg-[#FF9800]/10 text-[#B26A00]",
  approved: "bg-[#07C160]/10 text-[#059048]",
  refunded: "bg-[#07C160]/10 text-[#059048]",
  paid_out: "bg-[#07C160]/10 text-[#059048]",
  completed: "bg-[#07C160]/10 text-[#059048]",
  rejected: "bg-[#FA5151]/10 text-[#FA5151]",
};

function refundStatusBadgeClass(status: string): string {
  return REFUND_STATUS_BADGE_CLASSES[status] ?? "bg-active-bg text-text-sub";
}

/** Renders the request amount in the unit its target asset implies. */
function RefundAmount({ request }: { request: RefundRequestView }) {
  const { t, i18n } = useTranslation();
  const asset = request.targetAsset;
  if (asset === "cash") {
    return (
      <span className="text-[14px] font-bold text-[#FA5151]">
        {formatAmountCny(request.amount, request.currencyCode, i18n.language)}
      </span>
    );
  }
  const assetLabel = isRefundTargetAsset(asset)
    ? t(`orders.refund_asset_${asset}`, REFUND_TARGET_ASSET_LABELS[asset])
    : asset;
  return (
    <span className="text-[14px] font-bold text-[#FA5151]">
      {request.amount} {assetLabel}
    </span>
  );
}

interface RefundFormState {
  orderId: string;
  targetAsset: RefundTargetAsset;
  amount: string;
  reasonCode: string;
  reasonDetail: string;
}

export function RefundRequestsPage() {
  const { t } = useTranslation();
  const [searchParams, setSearchParams] = useSearchParams();
  const linkedOrderId = searchParams.get("orderId") ?? "";

  const [requests, setRequests] = useState<readonly RefundRequestView[]>([]);
  const [listLoading, setListLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);

  const [form, setForm] = useState<RefundFormState>({
    orderId: linkedOrderId,
    targetAsset: "cash",
    amount: "",
    reasonCode: "",
    reasonDetail: "",
  });
  const [submitting, setSubmitting] = useState(false);

  const load = useCallback(async () => {
    setListLoading(true);
    setListError(null);
    try {
      const page = await OrderService.listRefundRequests({ page: 1, pageSize: 20 });
      setRequests(page.items);
    } catch (err) {
      setListError(toUserErrorMessage(t, err));
    } finally {
      setListLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  // Keep the form order id in sync when the deep link changes.
  useEffect(() => {
    setForm((previous) => ({ ...previous, orderId: linkedOrderId }));
  }, [linkedOrderId]);

  const amountPlaceholder = useMemo(() => {    return form.targetAsset === "cash"
      ? t("orders.refund_amount_cash_hint", "输入退款金额（元），如 10.50")
      : t("orders.refund_amount_asset_hint", "输入退款额度（正整数）");
  }, [form.targetAsset, t]);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (submitting) {
      return;
    }
    const input: CreateRefundRequestInput = {
      orderId: form.orderId,
      targetAsset: form.targetAsset,
      amount: form.amount,
      currencyCode: "CNY",
      reasonCode: form.reasonCode || undefined,
      reasonDetail: form.reasonDetail || undefined,
    };
    setSubmitting(true);
    try {
      await OrderService.createRefundRequest(input);
      showToast(t("orders.refund_submitted_toast", "退款申请已提交"));
      setForm((previous) => ({
        ...previous,
        amount: "",
        reasonCode: "",
        reasonDetail: "",
      }));
      if (linkedOrderId) {
        setSearchParams({}, { replace: true });
      }
      await load();
    } catch (err) {
      showToast(toUserErrorMessage(t, err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <PageLayout title={t("orders.refund_title", "我的退款")}>
      <div className="flex flex-1 flex-col gap-3 overflow-y-auto px-3 py-3 pb-6">
        <section className="flex flex-col gap-3">
          <h3 className="text-[14px] font-bold text-text-main">
            {t("orders.refund_list_title", "退款记录")}
          </h3>
          {listLoading ? (
            <div className="flex h-24 items-center justify-center">
              <Loader2 className="h-6 w-6 animate-spin text-text-sub" />
            </div>
          ) : listError ? (
            <div className="flex flex-col items-center gap-3 rounded-xl bg-chat-other-bg p-6 text-center">
              <p className="text-[13px] text-text-sub">{listError}</p>
              <button
                type="button"
                className="rounded-full border border-border-color px-4 py-1.5 text-[13px] text-text-main active:bg-active-bg"
                onClick={() => void load()}
              >
                {t("orders.retry", "重试")}
              </button>
            </div>
          ) : requests.length === 0 ? (
            <div className="flex flex-col items-center gap-3 rounded-xl bg-chat-other-bg p-6 text-center">
              <ReceiptText className="h-8 w-8 text-text-sub/60" />
              <p className="text-[13px] text-text-sub">
                {t("orders.refund_empty", "暂无退款申请")}
              </p>
            </div>
          ) : (
            requests.map((request) => (
              <div
                className="flex flex-col gap-2 rounded-xl bg-chat-other-bg p-4 shadow-sm"
                key={request.requestId}
              >
                <div className="flex items-center justify-between">
                  <span className="text-[13px] font-medium text-text-main">
                    {request.subject || t("orders.refund_request_subject", "退款申请")}
                  </span>
                  <span
                    className={`rounded-full px-2 py-0.5 text-[11px] ${refundStatusBadgeClass(request.status)}`}
                  >
                    {t(
                      `orders.refund_status_${request.status}`,
                      REFUND_STATUS_FALLBACKS[request.status] ?? request.status,
                    )}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-[12px] text-text-sub">
                    {t("orders.refund_request_no", "申请单号")}
                  </span>
                  <span className="text-[12px] text-text-main">{request.requestNo}</span>
                </div>
                {request.originalOrderId && (
                  <div className="flex items-center justify-between">
                    <span className="text-[12px] text-text-sub">
                      {t("orders.refund_original_order", "关联订单")}
                    </span>
                    <span className="text-[12px] text-text-main">{request.originalOrderId}</span>
                  </div>
                )}
                <div className="flex items-center justify-between">
                  <RefundAmount request={request} />
                  <span className="text-[12px] text-text-sub">{request.createdAt}</span>
                </div>
              </div>
            ))
          )}
        </section>

        <section className="mt-2 flex flex-col gap-3 rounded-xl bg-chat-other-bg p-4 shadow-sm">
          <h3 className="text-[14px] font-bold text-text-main">
            {t("orders.refund_form_title", "发起退款")}
          </h3>
          <form className="flex flex-col gap-3" onSubmit={(event) => void handleSubmit(event)}>
            <label className="flex flex-col gap-1">
              <span className="text-[12px] text-text-sub">
                {t("orders.refund_original_order", "关联订单")}
              </span>
              <input
                className="rounded-lg border border-border-color bg-bg-color px-3 py-2 text-[13px] text-text-main outline-none focus:border-primary-blue"
                onChange={(event) =>
                  setForm((previous) => ({ ...previous, orderId: event.target.value }))
                }
                placeholder={t("orders.refund_order_placeholder", "输入要退款的原订单号")}
                required
                value={form.orderId}
              />
            </label>

            <div className="flex flex-col gap-1">
              <span className="text-[12px] text-text-sub">
                {t("orders.refund_target_asset", "退款到")}
              </span>
              <div className="flex gap-2">
                {REFUND_TARGET_ASSETS.map((asset) => (
                  <button
                    className={
                      form.targetAsset === asset
                        ? "flex-1 rounded-full border border-primary-blue bg-primary-blue/10 px-3 py-1.5 text-[13px] text-primary-blue font-medium"
                        : "flex-1 rounded-full border border-border-color px-3 py-1.5 text-[13px] text-text-main active:bg-active-bg"
                    }
                    key={asset}
                    onClick={() => setForm((previous) => ({ ...previous, targetAsset: asset }))}
                    type="button"
                  >
                    {t(`orders.refund_asset_${asset}`, REFUND_TARGET_ASSET_LABELS[asset])}
                  </button>
                ))}
              </div>
            </div>

            <label className="flex flex-col gap-1">
              <span className="text-[12px] text-text-sub">
                {t("orders.refund_amount", "退款金额")}
              </span>
              <input
                className="rounded-lg border border-border-color bg-bg-color px-3 py-2 text-[13px] text-text-main outline-none focus:border-primary-blue"
                inputMode="decimal"
                onChange={(event) =>
                  setForm((previous) => ({ ...previous, amount: event.target.value }))
                }
                placeholder={amountPlaceholder}
                required
                value={form.amount}
              />
            </label>

            <label className="flex flex-col gap-1">
              <span className="text-[12px] text-text-sub">
                {t("orders.refund_reason_code", "原因类型（选填）")}
              </span>
              <input
                className="rounded-lg border border-border-color bg-bg-color px-3 py-2 text-[13px] text-text-main outline-none focus:border-primary-blue"
                onChange={(event) =>
                  setForm((previous) => ({ ...previous, reasonCode: event.target.value }))
                }
                placeholder={t("orders.refund_reason_code_placeholder", "如 quality / not_received")}
                value={form.reasonCode}
              />
            </label>

            <label className="flex flex-col gap-1">
              <span className="text-[12px] text-text-sub">
                {t("orders.refund_reason_detail", "原因说明（选填）")}
              </span>
              <textarea
                className="min-h-[64px] rounded-lg border border-border-color bg-bg-color px-3 py-2 text-[13px] text-text-main outline-none focus:border-primary-blue"
                onChange={(event) =>
                  setForm((previous) => ({ ...previous, reasonDetail: event.target.value }))
                }
                placeholder={t("orders.refund_reason_detail_placeholder", "补充描述你的退款原因")}
                value={form.reasonDetail}
              />
            </label>

            <button
              className="mt-1 rounded-full border border-primary-blue bg-primary-blue px-4 py-2 text-[14px] font-medium text-white active:opacity-80 transition-opacity disabled:opacity-50"
              disabled={submitting}
              type="submit"
            >
              {submitting
                ? t("orders.refund_submitting", "提交中…")
                : t("orders.refund_submit", "提交退款申请")}
            </button>
          </form>
        </section>
      </div>
    </PageLayout>
  );
}
