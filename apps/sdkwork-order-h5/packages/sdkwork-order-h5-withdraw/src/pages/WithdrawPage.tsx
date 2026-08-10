import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Landmark, Wallet, CheckCircle2 } from "lucide-react";
import { PageLayout } from "@sdkwork/ui-mobile-react";

import {
  createWithdrawalRequestService,
  type WithdrawalRequestPort,
  type WithdrawalRequestResult,
} from "../services/WithdrawalRequestService";

export interface WithdrawPageProps {
  service?: WithdrawalRequestPort;
  /** Host-provided withdrawable cash balance (account wallet portfolio). */
  getCashBalance?: () => Promise<string>;
  currencyCode?: string;
}

const DEFAULT_CURRENCY_CODE = "CNY";

function sanitizeAmount(value: string): string {
  const normalized = value.replaceAll(/[^\d.]/g, "");
  const [integerPart, ...fractionParts] = normalized.split(".");
  const fractionPart = fractionParts.join("").slice(0, 2);
  return fractionPart ? `${integerPart}.${fractionPart}` : integerPart;
}

export function WithdrawPage({
  service: serviceProp,
  getCashBalance,
  currencyCode = DEFAULT_CURRENCY_CODE,
}: WithdrawPageProps) {
  const { t } = useTranslation();
  const service = serviceProp ?? createWithdrawalRequestService();
  const [cashAvailable, setCashAvailable] = useState<string | null>(null);
  const [amountInput, setAmountInput] = useState("");
  const [accountName, setAccountName] = useState("");
  const [accountNo, setAccountNo] = useState("");
  const [bankName, setBankName] = useState("");
  const [agreed, setAgreed] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [submitted, setSubmitted] = useState<WithdrawalRequestResult | null>(null);

  const load = useCallback(() => {
    setIsLoading(true);
    setLoadFailed(false);
    if (!getCashBalance) {
      setIsLoading(false);
      return;
    }
    getCashBalance()
      .then(setCashAvailable)
      .catch(() => setLoadFailed(true))
      .finally(() => setIsLoading(false));
  }, [getCashBalance]);

  useEffect(() => {
    load();
  }, [load]);

  const amount = Number.parseFloat(amountInput || "0");
  const available = cashAvailable != null ? Number(cashAvailable) : 0;

  const canSubmit = useMemo(() => {
    const trimmedAccountName = accountName.trim();
    const trimmedAccountNo = accountNo.trim();
    const trimmedBankName = bankName.trim();
    return (
      Number.isFinite(amount)
      && amount > 0
      && amount <= available
      && Boolean(trimmedAccountName)
      && Boolean(trimmedAccountNo)
      && Boolean(trimmedBankName)
      && agreed
      && !submitting
    );
  }, [accountName, accountNo, agreed, amount, available, bankName, submitting]);

  const insufficient = Number.isFinite(amount) && amount > 0 && amount > available;

  const handleSubmit = async () => {
    if (!canSubmit) {
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      const result = await service.createWithdrawalRequest({
        amount: amount.toFixed(2),
        currencyCode,
        payoutMethod: "bank_account",
        payoutAccountRef: JSON.stringify({
          accountName: accountName.trim(),
          accountNo: accountNo.trim(),
          bankName: bankName.trim(),
        }),
      });
      setSubmitted(result);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSubmitting(false);
    }
  };

  if (submitted) {
    return (
      <PageLayout title={t("withdraw.title", "提现")} bgClass="bg-bg-color">
        <div className="flex-1 flex flex-col items-center justify-center p-8">
          <CheckCircle2 className="w-16 h-16 text-green-500 mb-4" strokeWidth={1.5} />
          <div className="text-[17px] font-bold text-text-main mb-2">
            {t("withdraw.submitted_title", "提现申请已提交")}
          </div>
          <div className="text-[13px] text-text-sub text-center leading-relaxed mb-6">
            {t("withdraw.submitted_desc", "提现金额")}：¥{amount.toFixed(2)}<br />
            {submitted.requestNo
              ? `${t("withdraw.request_no", "申请单号")}：${submitted.requestNo}`
              : t("withdraw.under_review", "申请正在审核中，请留意到账通知")}
          </div>
          <button
            type="button"
            onClick={() => setSubmitted(null)}
            className="h-[44px] px-8 bg-primary-blue active:opacity-80 text-white font-bold text-[15px] rounded-full transition-opacity"
          >
            {t("withdraw.back_to_form", "继续提现")}
          </button>
        </div>
      </PageLayout>
    );
  }

  return (
    <PageLayout title={t("withdraw.title", "提现")} bgClass="bg-bg-color">
      <div className="flex-1 overflow-y-auto p-4 pb-32">
        {/* Cash balance */}
        <div className="bg-gradient-to-br from-blue-600 to-indigo-700 dark:from-blue-900/80 dark:to-indigo-950 rounded-2xl p-5 text-white mb-4">
          <div className="flex items-center gap-2 text-white/90 mb-2">
            <Wallet className="w-5 h-5" strokeWidth={1.5} />
            <span className="text-[15px] font-bold">{t("withdraw.cash_available", "可提现余额")}</span>
          </div>
          {isLoading ? (
            <div className="text-[13px] text-white/70">{t("withdraw.loading_balance", "正在加载余额...")}</div>
          ) : loadFailed ? (
            <button type="button" className="text-[13px] text-white/90 underline" onClick={load}>
              {t("withdraw.load_failed", "余额加载失败，点击重试")}
            </button>
          ) : (
            <div className="text-[32px] font-bold font-mono leading-none">
              ¥{Number(cashAvailable ?? 0).toLocaleString("zh-CN", { minimumFractionDigits: 2 })}
            </div>
          )}
        </div>

        {/* Amount */}
        <div className="bg-chat-other-bg rounded-2xl p-5 shadow-sm border border-border-color mb-4">
          <label className="text-[14px] text-text-main font-medium" htmlFor="withdraw-amount">
            {t("withdraw.amount_label", "提现金额")}
          </label>
          <div className="mt-3 flex items-center gap-2 border-b border-border-color pb-3">
            <span className="text-[22px] font-bold text-text-main">¥</span>
            <input
              id="withdraw-amount"
              inputMode="decimal"
              type="text"
              value={amountInput}
              onChange={(event) => setAmountInput(sanitizeAmount(event.target.value))}
              placeholder={t("withdraw.amount_placeholder", "请输入提现金额")}
              className="flex-1 bg-transparent text-[22px] font-bold font-mono text-text-main outline-none placeholder:text-text-sub placeholder:font-normal placeholder:text-[14px]"
            />
            <button
              type="button"
              disabled={available <= 0}
              onClick={() => setAmountInput(available.toFixed(2))}
              className="shrink-0 text-[12px] text-primary-blue disabled:opacity-40"
            >
              {t("withdraw.all", "全部")}
            </button>
          </div>
          {insufficient ? (
            <div className="mt-2 text-[12px] text-red-500">
              {t("withdraw.insufficient", "提现金额不能超过可提现余额")}
            </div>
          ) : (
            <div className="mt-2 text-[12px] text-text-sub">
              {t("withdraw.projected_balance", "提现后余额")}：¥{Math.max(0, available - amount).toFixed(2)}
            </div>
          )}
        </div>

        {/* Payout destination */}
        <div className="bg-chat-other-bg rounded-2xl p-5 shadow-sm border border-border-color mb-4">
          <div className="flex items-center gap-2 mb-4">
            <Landmark className="w-5 h-5 text-text-sub" strokeWidth={1.5} />
            <span className="text-[15px] text-text-main font-bold">{t("withdraw.destination_title", "到账账户")}</span>
          </div>
          <div className="space-y-4">
            <div>
              <label className="text-[13px] text-text-sub" htmlFor="withdraw-account-name">
                {t("withdraw.account_name", "收款人姓名")}
              </label>
              <input
                id="withdraw-account-name"
                type="text"
                value={accountName}
                onChange={(event) => setAccountName(event.target.value)}
                placeholder={t("withdraw.account_name_placeholder", "请输入收款人姓名")}
                className="mt-1.5 w-full h-[44px] px-3 rounded-xl border border-border-color bg-chat-other-bg text-[14px] text-text-main outline-none focus:border-primary-blue/40"
              />
            </div>
            <div>
              <label className="text-[13px] text-text-sub" htmlFor="withdraw-account-no">
                {t("withdraw.account_no", "收款账号")}
              </label>
              <input
                id="withdraw-account-no"
                type="text"
                value={accountNo}
                onChange={(event) => setAccountNo(event.target.value)}
                placeholder={t("withdraw.account_no_placeholder", "请输入银行卡号")}
                className="mt-1.5 w-full h-[44px] px-3 rounded-xl border border-border-color bg-chat-other-bg text-[14px] text-text-main outline-none focus:border-primary-blue/40"
              />
            </div>
            <div>
              <label className="text-[13px] text-text-sub" htmlFor="withdraw-bank-name">
                {t("withdraw.bank_name", "开户银行")}
              </label>
              <input
                id="withdraw-bank-name"
                type="text"
                value={bankName}
                onChange={(event) => setBankName(event.target.value)}
                placeholder={t("withdraw.bank_name_placeholder", "请输入开户银行名称")}
                className="mt-1.5 w-full h-[44px] px-3 rounded-xl border border-border-color bg-chat-other-bg text-[14px] text-text-main outline-none focus:border-primary-blue/40"
              />
            </div>
          </div>
        </div>

        {/* Agreement */}
        <label className="flex items-start gap-2 px-1 cursor-pointer">
          <input
            type="checkbox"
            checked={agreed}
            onChange={(event) => setAgreed(event.target.checked)}
            className="mt-0.5 accent-[#FA5151]"
          />
          <span className="text-[12px] text-text-sub leading-relaxed">
            {t("withdraw.agreement", "提交前请阅读并同意《提现服务协议》，提现申请审核通过后到账")}
          </span>
        </label>

        {error ? (
          <div className="mt-3 px-4 py-3 rounded-xl bg-red-50 dark:bg-red-900/20 text-[13px] text-red-500">
            {error}
          </div>
        ) : null}
      </div>

      <div className="absolute bottom-0 inset-x-0 flex items-center gap-4 px-4 pt-4 pb-[calc(env(safe-area-inset-bottom,0px)+1rem)] bg-chat-other-bg border-t border-border-color shadow-[0_-4px_20px_rgba(0,0,0,0.05)] z-30">
        <div className="flex-1 min-w-0">
          <div className="text-[20px] font-bold text-primary-blue leading-none">
            ¥{Number.isFinite(amount) ? amount.toFixed(2) : "0.00"}
          </div>
          <div className="mt-1 text-[12px] text-text-sub truncate">
            {t("withdraw.fee_note", "提现到银行卡，审核通过后安排打款")}
          </div>
        </div>
        <button
          type="button"
          disabled={!canSubmit}
          onClick={() => void handleSubmit()}
          className="shrink-0 h-[46px] px-8 bg-primary-blue active:opacity-80 disabled:opacity-40 text-white font-bold text-[15px] rounded-full transition-opacity"
        >
          {submitting
            ? t("withdraw.submitting", "正在提交...")
            : t("withdraw.submit", "确认提现")}
        </button>
      </div>
    </PageLayout>
  );
}
