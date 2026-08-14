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

export interface SdkworkOrderWithdrawalsPageProps extends TradeAdminIntlProps {
  canManage: boolean;
  service?: TradeAdminService;
}

export function SdkworkOrderWithdrawalsPage({
  canManage,
  service: injectedService,
  locale,
  messages,
}: SdkworkOrderWithdrawalsPageProps) {
  const service = useMemo(
    () => injectedService ?? createTradeAdminService(getSdkworkOrderBackendSdkClient()),
    [injectedService],
  );
  const copy = useMemo<AccountValueRequestCopy>(() => ({
    actionApprove: "admin.trade.withdrawals.action.approve",
    actionReject: "admin.trade.withdrawals.action.reject",
    actionRetry: "admin.trade.withdrawals.action.retry",
    amount: "admin.trade.withdrawals.amount",
    createdAt: "admin.trade.withdrawals.createdAt",
    detailTitle: "admin.trade.withdrawals.detailTitle",
    no: "admin.trade.withdrawals.no",
    originalOrder: "admin.trade.withdrawals.originalOrder",
    owner: "admin.trade.withdrawals.owner",
    providerReference: "admin.trade.withdrawals.providerReference",
    review: "admin.trade.withdrawals.review",
    reviewFailure: "admin.trade.withdrawals.reviewFailure",
    reviewSuccess: "admin.trade.withdrawals.reviewSuccess",
    reviewTitle: "admin.trade.withdrawals.reviewTitle",
    subject: "admin.trade.withdrawals.subject",
    targetAsset: "admin.trade.withdrawals.targetAsset",
    title: "admin.trade.withdrawals.title",
    updatedAt: "admin.trade.withdrawals.updatedAt",
  }), []);

  return (
    <AccountValueRequestListPage
      canManage={canManage}
      copy={copy}
      listKind="withdrawals"
      locale={locale}
      messages={messages}
      service={service}
    />
  );
}
