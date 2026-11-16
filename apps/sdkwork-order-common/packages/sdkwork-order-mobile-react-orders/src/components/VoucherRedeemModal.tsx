import { useState } from "react";
import { Search, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { showToast } from "@sdkwork/ui-mobile-react";

import { OrderService } from "../services/OrderService";

interface VoucherRedeemModalProps {
  isOpen: boolean;
  onClose: () => void;
}

/**
 * Voucher (coupon code) redemption surface.
 *
 * Input + confirm only; redemption goes through the canonical order coupon
 * redemption port (`OrderService.redeemVoucher`). The earlier build shipped a
 * mock scanner viewport that could never open a camera, so it was removed
 * instead of pretending to scan; a host with camera support can add its own
 * scan entry and call the same service.
 */
export const VoucherRedeemModal: React.FC<VoucherRedeemModalProps> = ({
  isOpen,
  onClose,
}) => {
  const { t } = useTranslation();
  const [code, setCode] = useState("");
  const [submitting, setSubmitting] = useState(false);

  if (!isOpen) {
    return null;
  }

  const handleRedeem = async () => {
    if (!code.trim()) {
      showToast(t("orders.voucher_code_required", "请输入有效的券码"));
      return;
    }
    setSubmitting(true);
    try {
      const result = await OrderService.redeemVoucher(code.trim().toUpperCase());
      if (result.success) {
        showToast(t("orders.voucher_redeem_success", "核销成功"));
        setCode("");
        onClose();
        return;
      }
      showToast(t("orders.voucher_redeem_failed", "核销失败，请重试"));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-bg-color">
      <header className="flex h-[56px] shrink-0 items-center justify-between border-b border-border-color px-2 pt-safe">
        <button
          aria-label={t("orders.close", "关闭")}
          className="p-2 active:opacity-70"
          onClick={onClose}
          type="button"
        >
          <X className="h-6 w-6 text-text-main" />
        </button>
        <h2 className="text-[17px] font-semibold text-text-main">
          {t("orders.voucher_title", "券码核销")}
        </h2>
        <div className="w-10" />
      </header>

      <div className="flex flex-1 flex-col items-center overflow-y-auto p-4">
        <div className="mt-4 flex w-full max-w-[320px] flex-col gap-4 rounded-xl bg-chat-other-bg p-4">
          <p className="text-[14px] font-medium text-text-main">
            {t("orders.voucher_input_label", "输入券码号核销")}
          </p>
          <div className="flex items-center rounded-lg bg-bg-color px-3 py-2">
            <Search className="h-4 w-4 shrink-0 text-text-sub" />
            <input
              autoComplete="off"
              className="min-w-0 flex-1 border-none bg-transparent px-2 text-[15px] text-text-main outline-none"
              inputMode="text"
              onChange={(event) => setCode(event.target.value)}
              placeholder={t("orders.voucher_input_placeholder", "请输入例如 ABC12345")}
              value={code}
            />
          </div>
          <button
            className="w-full rounded-lg py-2.5 text-[15px] font-medium text-white active:opacity-80 disabled:opacity-60"
            disabled={submitting}
            onClick={() => void handleRedeem()}
            style={{ backgroundColor: "var(--order-accent,#FA5151)" }}
            type="button"
          >
            {submitting
              ? t("orders.voucher_submitting", "核销中...")
              : t("orders.voucher_confirm", "确认核销")}
          </button>
          <p className="text-center text-[12px] text-text-sub">
            {t("orders.voucher_hint", "券码核销后不可撤销，请确认输入无误")}
          </p>
        </div>
      </div>
    </div>
  );
};
