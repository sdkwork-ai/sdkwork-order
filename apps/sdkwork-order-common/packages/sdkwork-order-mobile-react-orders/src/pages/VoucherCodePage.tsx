import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { PageLayout } from "@sdkwork/ui-mobile-react";

import { VoucherRedeemModal } from "../components/VoucherRedeemModal";

export function VoucherCodePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [isOpen, setIsOpen] = useState(true);

  return (
    <PageLayout title={t("orders.voucher_title", "券码核销")}>
      <VoucherRedeemModal
        isOpen={isOpen}
        onClose={() => {
          setIsOpen(false);
          navigate(-1);
        }}
      />
    </PageLayout>
  );
}
