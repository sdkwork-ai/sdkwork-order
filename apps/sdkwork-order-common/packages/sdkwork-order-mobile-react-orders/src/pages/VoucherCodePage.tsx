import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import { VoucherRedeemModal } from "../components/VoucherRedeemModal";

/**
 * Standalone voucher redemption screen.
 *
 * The redeem modal is the whole page body (it owns the header and close
 * affordance), so hosts route here from wallet/scan entries and the shell must
 * not paint a second navigation bar. Closing returns to the entry point.
 */
export function VoucherCodePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <>
      <h1 className="sr-only">{t("orders.voucher_title", "券码核销")}</h1>
      <VoucherRedeemModal isOpen onClose={() => navigate(-1)} />
    </>
  );
}
