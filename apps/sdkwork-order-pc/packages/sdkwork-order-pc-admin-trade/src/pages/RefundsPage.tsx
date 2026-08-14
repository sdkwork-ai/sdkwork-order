import { useMemo } from "react";
import { getSdkworkOrderBackendSdkClient } from "@sdkwork/order-pc-admin-core";
import type { TradeAdminIntlProps } from "../i18n/intl";
import {
  createTradeAdminService,
  type TradeAdminService,
} from "../trade-admin-service";
import {
  AccountValueRequestListPage,
  type AccountValueRequestCopy,
} from "./account-value-request-page";

export interface SdkworkOrderRefundsPageProps extends TradeAdminIntlProps {
  canManage: boolean;
  service?: TradeAdminService;
}

export function SdkworkOrderRefundsPage({
  canManage,
  service: injectedService,
  locale,
  messages,
}: SdkworkOrderRefundsPageProps) {
  const service = useMemo(
    () => injectedService ?? createTradeAdminService(getSdkworkOrderBackendSdkClient()),
    [injectedService],
  );
  const copy = useMemo<AccountValueRequestCopy>(() => ({
    actionApprove: "admin.trade.refunds.action.approve",
    actionReject: "admin.trade.refunds.action.reject",
    actionRetry: "admin.trade.refunds.action.retry",
    amount: "admin.trade.refunds.amount",
    createdAt: "admin.trade.refunds.createdAt",
    detailTitle: "admin.trade.refunds.detailTitle",
    no: "admin.trade.refunds.no",
    originalOrder: "admin.trade.refunds.originalOrder",
    owner: "admin.trade.refunds.owner",
    providerReference: "admin.trade.refunds.providerReference",
    review: "admin.trade.refunds.review",
    reviewFailure: "admin.trade.refunds.reviewFailure",
    reviewSuccess: "admin.trade.refunds.reviewSuccess",
    reviewTitle: "admin.trade.refunds.reviewTitle",
    subject: "admin.trade.refunds.subject",
    targetAsset: "admin.trade.refunds.targetAsset",
    title: "admin.trade.refunds.title",
    updatedAt: "admin.trade.refunds.updatedAt",
  }), []);

  return (
    <AccountValueRequestListPage
      canManage={canManage}
      copy={copy}
      listKind="refunds"
      locale={locale}
      messages={messages}
      service={service}
    />
  );
}
