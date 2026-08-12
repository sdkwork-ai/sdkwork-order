import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { ReceiptText } from "lucide-react";

/**
 * Fail-closed page for order surfaces that have no composed owner SDK
 * surface in the current host release. Renders a typed unavailable state
 * instead of fabricated orders, vouchers, or payment results.
 */
export function CapabilityUnavailablePage({
  title,
  message,
}: {
  title?: string;
  message?: string;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <div className="flex h-full flex-col bg-bg-color">
      <header className="flex h-[56px] shrink-0 items-center border-b border-border-color px-1">
        <button
          type="button"
          className="px-2 text-[15px] text-text-main"
          onClick={() => navigate(-1)}
        >
          {t("common.back", "Back")}
        </button>
        <h2 className="flex-1 pr-10 text-center text-[17px] font-medium text-text-main">
          {title ?? t("order.unavailableTitle", "Orders")}
        </h2>
      </header>
      <main className="flex flex-1 flex-col items-center justify-center gap-3 px-8 text-center">
        <ReceiptText className="h-10 w-10 text-text-sub" />
        <p className="max-w-sm text-[15px] text-text-main">
          {message ?? t("order.unavailable", "Orders are unavailable until their owner SDK is composed.")}
        </p>
      </main>
    </div>
  );
}
