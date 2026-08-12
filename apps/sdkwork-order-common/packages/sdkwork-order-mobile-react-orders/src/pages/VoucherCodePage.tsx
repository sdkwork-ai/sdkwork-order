import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { PageLayout } from "@sdkwork/ui-mobile-react";

import { VoucherRedeemModal } from "../components/VoucherRedeemModal";

import { CapabilityUnavailablePage } from "../components/CapabilityUnavailablePage";

export function VoucherCodePage() {
  return (
    <CapabilityUnavailablePage />
  );
}
