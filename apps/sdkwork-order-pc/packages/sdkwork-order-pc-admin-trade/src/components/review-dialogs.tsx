import { useState, type FormEvent } from "react";
import {
  Button,
  Input,
  Modal,
  ModalBody,
  ModalContent,
  ModalDescription,
  ModalFooter,
  ModalHeader,
  ModalTitle,
  Textarea,
} from "@sdkwork/ui-pc-react";
import { useTradeAdminI18n } from "../i18n/intl";
import type {
  AfterSalesReviewInput,
  TradeRequestReviewInput,
} from "../trade-admin-service";

function fieldLabelClass(): string {
  return "block space-y-1.5 text-xs font-medium text-[var(--sdk-color-text-secondary)]";
}

export function RequestReviewDialog({
  action,
  busy,
  label,
  onConfirm,
  onOpenChange,
  open,
  requestNo,
  title,
}: {
  action: "approve" | "reject" | "retry";
  busy: boolean;
  label: string;
  onConfirm: (input: TradeRequestReviewInput) => void;
  onOpenChange: (open: boolean) => void;
  open: boolean;
  requestNo: string;
  title: string;
}) {
  const { t } = useTradeAdminI18n();
  const [reasonCode, setReasonCode] = useState("");
  const [reviewComment, setReviewComment] = useState("");

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    onConfirm({
      reasonCode: reasonCode.trim() || undefined,
      reviewComment: reviewComment.trim() || undefined,
    });
  };

  return (
    <Modal open={open} onOpenChange={(next) => { if (!busy) onOpenChange(next); }}>
      <ModalContent size="sm">
        <ModalHeader>
          <ModalTitle>{title}</ModalTitle>
          <ModalDescription>{requestNo}</ModalDescription>
        </ModalHeader>
        <form onSubmit={submit}>
          <ModalBody>
            <div className="space-y-4">
              <p className="text-sm text-[var(--sdk-color-text-secondary)]">{label}</p>
              <label className={fieldLabelClass()}>
                <span>{t("admin.trade.refunds.reasonCode", "Reason code")}</span>
                <Input
                  placeholder={t("admin.trade.filter.statusPlaceholder", "e.g. submitted")}
                  value={reasonCode}
                  onChange={(event) => setReasonCode(event.target.value)}
                />
              </label>
              <label className={fieldLabelClass()}>
                <span>{t("admin.trade.refunds.reviewComment", "Review comment")}</span>
                <Textarea
                  placeholder={t("admin.trade.refunds.reviewCommentPlaceholder", "Internal comment for this review.")}
                  rows={3}
                  value={reviewComment}
                  onChange={(event) => setReviewComment(event.target.value)}
                />
              </label>
            </div>
          </ModalBody>
          <ModalFooter>
            <Button disabled={busy} type="button" variant="secondary" onClick={() => onOpenChange(false)}>
              {t("admin.trade.list.close", "Close")}
            </Button>
            <Button disabled={busy} loading={busy} type="submit">
              {t("admin.trade.review.confirm", "Confirm")}
            </Button>
          </ModalFooter>
        </form>
      </ModalContent>
    </Modal>
  );
}

export function AfterSalesReviewDialog({
  action,
  busy,
  label,
  onConfirm,
  onOpenChange,
  open,
  requestNo,
  title,
}: {
  action: "approve" | "reject";
  busy: boolean;
  label: string;
  onConfirm: (input: AfterSalesReviewInput) => void;
  onOpenChange: (open: boolean) => void;
  open: boolean;
  requestNo: string;
  title: string;
}) {
  const { t } = useTradeAdminI18n();
  const [approvedAmount, setApprovedAmount] = useState("");
  const [reasonDetail, setReasonDetail] = useState("");
  const [reviewComment, setReviewComment] = useState("");

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    onConfirm({
      action,
      approvedAmount: action === "approve" && approvedAmount.trim() ? approvedAmount.trim() : undefined,
      reasonDetail: reasonDetail.trim() || undefined,
      reviewComment: reviewComment.trim() || undefined,
    });
  };

  return (
    <Modal open={open} onOpenChange={(next) => { if (!busy) onOpenChange(next); }}>
      <ModalContent size="sm">
        <ModalHeader>
          <ModalTitle>{title}</ModalTitle>
          <ModalDescription>{requestNo}</ModalDescription>
        </ModalHeader>
        <form onSubmit={submit}>
          <ModalBody>
            <div className="space-y-4">
              <p className="text-sm text-[var(--sdk-color-text-secondary)]">{label}</p>
              {action === "approve" ? (
                <label className={fieldLabelClass()}>
                  <span>{t("admin.trade.afterSales.approvedAmount", "Approved amount")}</span>
                  <Input
                    placeholder={t("admin.trade.afterSales.approvedAmountHint", "Leave blank to approve the full requested amount.")}
                    value={approvedAmount}
                    onChange={(event) => setApprovedAmount(event.target.value)}
                  />
                </label>
              ) : null}
              <label className={fieldLabelClass()}>
                <span>{t("admin.trade.afterSales.reasonDetail", "Reason detail")}</span>
                <Input
                  value={reasonDetail}
                  onChange={(event) => setReasonDetail(event.target.value)}
                />
              </label>
              <label className={fieldLabelClass()}>
                <span>{t("admin.trade.afterSales.reviewComment", "Review comment")}</span>
                <Textarea
                  placeholder={t("admin.trade.afterSales.reviewCommentPlaceholder", "Visible to the buyer in the after-sales record.")}
                  rows={3}
                  value={reviewComment}
                  onChange={(event) => setReviewComment(event.target.value)}
                />
              </label>
            </div>
          </ModalBody>
          <ModalFooter>
            <Button disabled={busy} type="button" variant="secondary" onClick={() => onOpenChange(false)}>
              {t("admin.trade.list.close", "Close")}
            </Button>
            <Button disabled={busy} loading={busy} type="submit" variant={action === "approve" ? "primary" : "danger"}>
              {t("admin.trade.review.confirm", "Confirm")}
            </Button>
          </ModalFooter>
        </form>
      </ModalContent>
    </Modal>
  );
}
