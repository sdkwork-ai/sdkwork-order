import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router";
import { Loader2, ReceiptText } from "lucide-react";
import { PageLayout, showToast } from "@sdkwork/ui-mobile-react";

import { OrderActionButtons } from "../components/OrderActionButtons";
import { OrderInfoCards } from "../components/OrderInfoCards";
import { OrderItemsCard } from "../components/OrderItemsCard";
import { OrderService, type Order } from "../services/OrderService";
import { localizeOrderTitle } from "../services/orderTitle";
import { toUserErrorMessage } from "../services/errorMessage";
import {
  ORDER_MOBILE_ROUTE_DEFINITIONS,
  resolveHostRoutePath,
} from "../routes";

/** Host-overridable order route template (path with `:orderId`). */
export interface OrderDetailProps {
  orderCashierPath?: string;
}

import { CapabilityUnavailablePage } from "../components/CapabilityUnavailablePage";

export function OrderDetail() {
  return (
    <CapabilityUnavailablePage />
  );
}
