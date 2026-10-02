import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { PageLayout } from "@sdkwork/ui-mobile-react";

import { VoucherRedeemModal } from "../components/VoucherRedeemModal";

/**
 * Standalone voucher redemption screen. The redeem modal is the page body:
 * hosts route here from wallet/scan entries, and closing the modal returns
 * to the entry point instead of leaving an empty screen behind.
 */
export function VoucherCodePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <PageLayout title={t("orders.voucher_title", "券码核销")}>
      <VoucherRedeemModal isOpen onClose={() => navigate(-1)} />
    </PageLayout>
  );
}
