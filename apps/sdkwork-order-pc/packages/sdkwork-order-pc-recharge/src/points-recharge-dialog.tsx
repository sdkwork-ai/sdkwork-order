/// <reference path="../styles.d.ts" />

import { useEffect, useId, useMemo, useRef, useState } from "react";
import { Check, CheckCircle2, QrCode, Sparkles, X } from "lucide-react";
import { toDataURL } from "qrcode";
import {
  Button,
  Modal,
  ModalBody,
  ModalClose,
  ModalContent,
  ModalHeader,
  ModalTitle,
  StatusNotice,
} from "@sdkwork/ui-pc-react";
import {
  sdkworkTokenBankPointsMicroToDecimal,
  createSdkworkPointsRechargeService,
  type SdkworkPointsRechargePackage,
  type SdkworkPointsRechargePayment,
  type SdkworkPointsRechargeService,
} from "@sdkwork/order-service";
import "./points-recharge-dialog.css";

export interface SdkworkPointsRechargeDialogCopy {
  account: string;
  agreement: string;
  agreementAccepted: string;
  agreementRequired: string;
  close: string;
  completed: string;
  confirmPayment: string;
  creatingPayment: string;
  emptyPackages: string;
  expired: string;
  expiredDescription: string;
  expiresIn: string;
  loadFailed: string;
  loadingPackages: string;
  myPoints: string;
  notice: string;
  paymentUnavailable: string;
  paymentUnavailableDescription: string;
  pointsUnit: string;
  retry: string;
  retryPayment: string;
  scanPrompt: string;
  title: string;
}

export interface SdkworkPointsRechargeProps {
  copy?: Partial<SdkworkPointsRechargeDialogCopy>;
  currentPoints?: number | null;
  onCompleted?: (payment: SdkworkPointsRechargePayment) => Promise<void> | void;
  paymentMethod?: string;
  service?: SdkworkPointsRechargeService;
}

export interface SdkworkPointsRechargeDialogProps extends SdkworkPointsRechargeProps {
  isOpen: boolean;
  onClose: () => void;
}

export interface SdkworkPointsRechargeInlineProps extends SdkworkPointsRechargeProps {
  className?: string;
}

interface SdkworkPointsRechargeCheckout {
  packageId: string;
  payment: SdkworkPointsRechargePayment;
}

const DEFAULT_COPY: SdkworkPointsRechargeDialogCopy = {
  account: "积分账户",
  agreement: "我已阅读并同意《积分充值服务协议》",
  agreementAccepted: "您已同意《积分充值服务协议》",
  agreementRequired: "请先同意积分充值服务协议",
  close: "关闭",
  completed: "支付完成，积分已到账",
  confirmPayment: "同意并支付",
  creatingPayment: "正在生成支付二维码...",
  emptyPackages: "暂无可用充值套餐",
  expired: "订单已过期",
  expiredDescription: "当前充值订单已过期，请重新创建订单后继续支付。",
  expiresIn: "订单剩余支付时间",
  loadFailed: "充值套餐加载失败",
  loadingPackages: "正在加载充值套餐...",
  myPoints: "我的积分",
  notice: "积分不可转赠、不可提现，充值后有效期以平台规则为准。",
  paymentUnavailable: "支付暂不可用",
  paymentUnavailableDescription: "暂时无法生成支付二维码，请稍后重试。",
  pointsUnit: "积分",
  retry: "重新加载",
  retryPayment: "重新创建订单",
  scanPrompt: "请扫码完成支付",
  title: "积分购买",
};

interface SdkworkPointsRechargeExperienceProps extends SdkworkPointsRechargeProps {
  active: boolean;
  className?: string;
  display: "dialog" | "inline";
  onClose?: () => void;
}

function parseExpirationTime(value: string | undefined): number | null {
  if (!value) return null;
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? timestamp : null;
}

function formatRemainingTime(totalSeconds: number): string {
  const hours = Math.floor(totalSeconds / 3_600);
  const minutes = Math.floor((totalSeconds % 3_600) / 60);
  const seconds = totalSeconds % 60;
  const segments = hours > 0 ? [hours, minutes, seconds] : [minutes, seconds];
  return segments.map((segment) => String(segment).padStart(2, "0")).join(":");
}

function SdkworkPointsRechargeExperience({
  active,
  className,
  copy: copyOverrides,
  currentPoints,
  display,
  onClose,
  onCompleted,
  paymentMethod = "wechat_pay",
  service: serviceProp,
}: SdkworkPointsRechargeExperienceProps) {
  const titleId = useId();
  const copy = useMemo(() => ({ ...DEFAULT_COPY, ...copyOverrides }), [copyOverrides]);
  const service = useMemo(
    () => serviceProp ?? createSdkworkPointsRechargeService(),
    [serviceProp],
  );
  const [packages, setPackages] = useState<SdkworkPointsRechargePackage[]>([]);
  const [selectedPackageId, setSelectedPackageId] = useState<string | null>(null);
  const [checkout, setCheckout] = useState<SdkworkPointsRechargeCheckout | null>(null);
  const [qrImageUrl, setQrImageUrl] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isPaying, setIsPaying] = useState(false);
  const [hasAcceptedAgreement, setHasAcceptedAgreement] = useState(false);
  const [loadAttempt, setLoadAttempt] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [currentTimeMs, setCurrentTimeMs] = useState(() => Date.now());
  const completedOrderRef = useRef<string | null>(null);
  const hasAcceptedAgreementRef = useRef(false);
  const isPayingRef = useRef(false);
  const paymentRequestSequenceRef = useRef(0);
  const selectedPackageIdRef = useRef<string | null>(null);

  const selectedPackage = packages.find((item) => item.id === selectedPackageId) ?? null;
  const payment = checkout?.packageId === selectedPackageId ? checkout.payment : null;
  const expirationTimeMs = parseExpirationTime(payment?.expiresAt);
  const remainingSeconds = expirationTimeMs === null
    ? null
    : Math.max(0, Math.ceil((expirationTimeMs - currentTimeMs) / 1_000));
  const isExpired = payment?.status === "pending" && remainingSeconds === 0;
  const hasActivePayment = payment !== null && payment.status !== "failed" && !isExpired;

  useEffect(() => {
    if (!active) {
      paymentRequestSequenceRef.current += 1;
      hasAcceptedAgreementRef.current = false;
      isPayingRef.current = false;
      setHasAcceptedAgreement(false);
      return undefined;
    }
    let mounted = true;
    paymentRequestSequenceRef.current += 1;
    isPayingRef.current = false;
    setIsLoading(true);
    setIsPaying(false);
    setError(null);
    setCheckout(null);
    setQrImageUrl(null);
    completedOrderRef.current = null;
    void service.listPackages()
      .then((items) => {
        if (!mounted) return;
        setPackages(items);
        setSelectedPackageId((current) => {
          const next = current && items.some((item) => item.id === current)
            ? current
            : items[0]?.id ?? null;
          selectedPackageIdRef.current = next;
          return next;
        });
      })
      .catch((cause) => {
        if (mounted) setError(cause instanceof Error ? cause.message : copy.loadFailed);
      })
      .finally(() => {
        if (mounted) setIsLoading(false);
      });
    return () => {
      mounted = false;
      paymentRequestSequenceRef.current += 1;
      isPayingRef.current = false;
    };
  }, [active, copy.loadFailed, loadAttempt, service]);

  useEffect(() => {
    if (!payment?.qrCode) {
      setQrImageUrl(null);
      return undefined;
    }
    if (payment.qrCode.startsWith("data:image/")) {
      setQrImageUrl(payment.qrCode);
      return undefined;
    }
    let qrActive = true;
    void toDataURL(payment.qrCode, { errorCorrectionLevel: "M", margin: 1, width: 252 })
      .then((value) => {
        if (qrActive) setQrImageUrl(value);
      })
      .catch(() => {
        if (qrActive) setError(copy.paymentUnavailableDescription);
      });
    return () => {
      qrActive = false;
    };
  }, [copy.paymentUnavailableDescription, payment?.qrCode]);

  useEffect(() => {
    if (!active || payment?.status !== "pending" || expirationTimeMs === null) {
      return undefined;
    }

    const updateCurrentTime = () => setCurrentTimeMs(Date.now());
    updateCurrentTime();
    if (expirationTimeMs <= Date.now()) return undefined;
    const interval = window.setInterval(updateCurrentTime, 1_000);
    return () => window.clearInterval(interval);
  }, [active, expirationTimeMs, payment?.status]);

  useEffect(() => {
    if (!active || !checkout || payment?.status !== "pending" || isExpired || !payment.orderId) {
      return undefined;
    }
    const orderId = payment.orderId;
    const packageId = checkout.packageId;
    const paymentSessionSequence = paymentRequestSequenceRef.current;
    let pollingActive = true;
    const poll = async () => {
      try {
        const next = await service.getOrderStatus(orderId);
        if (!pollingActive
          || paymentRequestSequenceRef.current !== paymentSessionSequence
          || selectedPackageIdRef.current !== packageId) return;
        setCurrentTimeMs(Date.now());
        setCheckout((current) => current?.packageId === packageId
          && current.payment.orderId === orderId
          ? { packageId, payment: { ...current.payment, ...next } }
          : current);
        if (next.status === "completed" && completedOrderRef.current !== orderId) {
          completedOrderRef.current = orderId;
          await onCompleted?.(next);
        }
        if (next.status === "failed") setError(copy.paymentUnavailableDescription);
      } catch {
        // Keep the current QR code visible while a transient status request fails.
      }
    };
    void poll();
    const interval = window.setInterval(() => void poll(), 2_500);
    return () => {
      pollingActive = false;
      window.clearInterval(interval);
    };
  }, [active, checkout?.packageId, copy.paymentUnavailableDescription, isExpired, onCompleted, payment?.expiresAt, payment?.orderId, payment?.status, service]);

  function selectPackage(packageId: string) {
    if (isPayingRef.current) return;
    if (packageId === selectedPackageIdRef.current) {
      if (isExpired && hasAcceptedAgreementRef.current) {
        void createPayment(packageId);
      }
      return;
    }
    paymentRequestSequenceRef.current += 1;
    selectedPackageIdRef.current = packageId;
    completedOrderRef.current = null;
    setSelectedPackageId(packageId);
    setCheckout(null);
    setQrImageUrl(null);
    setError(null);
    if (hasAcceptedAgreementRef.current) {
      void createPayment(packageId);
    }
  }

  function closeDialog() {
    paymentRequestSequenceRef.current += 1;
    isPayingRef.current = false;
    onClose?.();
  }

  async function createPayment(packageId: string) {
    if (isPayingRef.current) return;
    const requestSequence = paymentRequestSequenceRef.current + 1;
    paymentRequestSequenceRef.current = requestSequence;
    isPayingRef.current = true;
    setIsPaying(true);
    setError(null);
    try {
      const result = await service.createOrder({ packageId, paymentMethod });
      if (paymentRequestSequenceRef.current !== requestSequence
        || selectedPackageIdRef.current !== packageId) return;
      setCurrentTimeMs(Date.now());
      setCheckout({ packageId, payment: result });
      if (result.status === "completed") {
        const key = result.orderId ?? packageId;
        if (completedOrderRef.current !== key) {
          completedOrderRef.current = key;
          await onCompleted?.(result);
        }
      }
    } catch (cause) {
      if (paymentRequestSequenceRef.current === requestSequence
        && selectedPackageIdRef.current === packageId) {
        setError(cause instanceof Error ? cause.message : copy.paymentUnavailableDescription);
      }
    } finally {
      if (paymentRequestSequenceRef.current === requestSequence) {
        isPayingRef.current = false;
        setIsPaying(false);
      }
    }
  }

  function acceptAgreementAndCreatePayment() {
    if (!selectedPackage || isPayingRef.current) return;
    hasAcceptedAgreementRef.current = true;
    setHasAcceptedAgreement(true);
    void createPayment(selectedPackage.id);
  }

  const content = (
    <>
        <ModalHeader className="sdkwork-points-recharge-dialog__header">
          <div className="sdkwork-points-recharge-dialog__identity">
            <Sparkles aria-hidden="true" />
            {display === "dialog" ? (
              <ModalTitle id={titleId}>{copy.title}</ModalTitle>
            ) : (
              <h2 id={titleId}>{copy.account}</h2>
            )}
          </div>
          <div className="sdkwork-points-recharge-dialog__header-actions">
            <div className="sdkwork-points-recharge-dialog__balance">
              {copy.myPoints} <strong>{currentPoints ?? "--"}</strong>
            </div>
            {display === "dialog" ? (
              <ModalClose aria-label={copy.close} className="sdkwork-points-recharge-dialog__close"><X aria-hidden="true" /></ModalClose>
            ) : null}
          </div>
        </ModalHeader>
        <ModalBody className="sdkwork-points-recharge-dialog__body">
          <section className="sdkwork-points-recharge-dialog__packages" aria-label={copy.title}>
            {display === "inline" ? (
              <h3 className="sdkwork-points-recharge-dialog__section-title">{copy.title}</h3>
            ) : null}
            {isLoading ? <p className="sdkwork-points-recharge-dialog__muted">{copy.loadingPackages}</p> : null}
            {!isLoading && packages.length === 0 && !error ? <p className="sdkwork-points-recharge-dialog__muted">{copy.emptyPackages}</p> : null}
            {!isLoading && packages.length === 0 && error ? (
              <div className="sdkwork-points-recharge-dialog__load-error">
                <StatusNotice tone="danger" title={copy.loadFailed}>{error}</StatusNotice>
                <Button onClick={() => setLoadAttempt((current) => current + 1)} type="button" variant="secondary">{copy.retry}</Button>
              </div>
            ) : null}
            {!isLoading && packages.length > 0 ? (
              <div aria-busy={isPaying} className="sdkwork-points-recharge-dialog__grid">
                {packages.map((item) => {
                  const selected = item.id === selectedPackageId;
                  return (
                    <button
                      aria-pressed={selected}
                      className={`sdkwork-points-recharge-dialog__package ${selected ? "is-selected" : ""}`}
                      disabled={isLoading || isPaying}
                      key={item.id}
                      onClick={() => selectPackage(item.id)}
                      type="button"
                    >
                      <span className="sdkwork-points-recharge-dialog__points"><Sparkles aria-hidden="true" />{sdkworkTokenBankPointsMicroToDecimal(item.points)} <small>{copy.pointsUnit}</small></span>
                      <span className="sdkwork-points-recharge-dialog__price">{item.currencyCode} {item.priceAmount.toFixed(2)}</span>
                    </button>
                  );
                })}
              </div>
            ) : null}
            <p className="sdkwork-points-recharge-dialog__hint">{copy.notice}</p>
          </section>
          <aside aria-live="polite" className="sdkwork-points-recharge-dialog__payment">
            {error && packages.length > 0 ? <StatusNotice tone="danger" title={copy.paymentUnavailable}>{error}</StatusNotice> : null}
            {isPaying || (!isExpired && (!payment || payment.status === "failed")) ? (
              <div className="sdkwork-points-recharge-dialog__payment-empty">
                <QrCode aria-hidden="true" />
                {hasAcceptedAgreement ? (
                  <p className="sdkwork-points-recharge-dialog__agreement-accepted">
                    <span className="sdkwork-points-recharge-dialog__agreement-check">
                      <Check aria-hidden="true" />
                    </span>
                    {copy.agreementAccepted}
                  </p>
                ) : <p>{copy.agreement}</p>}
                <Button
                  disabled={!selectedPackage || isPaying || isLoading || hasActivePayment}
                  loading={isPaying}
                  onClick={acceptAgreementAndCreatePayment}
                  type="button"
                >
                  {isPaying ? copy.creatingPayment : copy.confirmPayment}
                </Button>
              </div>
            ) : null}
            {!isPaying && payment?.status === "pending" && !isExpired && qrImageUrl ? (
              <div className="sdkwork-points-recharge-dialog__qr">
                <h3>{copy.scanPrompt}</h3>
                {remainingSeconds !== null ? (
                  <p className="sdkwork-points-recharge-dialog__countdown" role="timer">
                    <span>{copy.expiresIn}</span>
                    <strong>{formatRemainingTime(remainingSeconds)}</strong>
                  </p>
                ) : null}
                <img alt={copy.scanPrompt} src={qrImageUrl} />
                <div className="sdkwork-points-recharge-dialog__agreement-accepted">
                  <span className="sdkwork-points-recharge-dialog__agreement-check">
                    <Check aria-hidden="true" />
                  </span>
                  {copy.agreementAccepted}
                </div>
              </div>
            ) : null}
            {!isPaying && isExpired ? (
              <div className="sdkwork-points-recharge-dialog__expired">
                <StatusNotice tone="danger" title={copy.expired}>{copy.expiredDescription}</StatusNotice>
                <Button
                  disabled={!selectedPackage}
                  onClick={() => selectedPackage && void createPayment(selectedPackage.id)}
                  type="button"
                  variant="secondary"
                >
                  {copy.retryPayment}
                </Button>
              </div>
            ) : null}
            {payment?.status === "completed" ? <div className="sdkwork-points-recharge-dialog__completed"><CheckCircle2 aria-hidden="true" /><strong>{copy.completed}</strong>{display === "dialog" ? <Button onClick={closeDialog} type="button">{copy.close}</Button> : null}</div> : null}
            {payment?.status === "failed" && !error ? <StatusNotice tone="danger" title={copy.paymentUnavailable}>{copy.paymentUnavailableDescription}</StatusNotice> : null}
          </aside>
        </ModalBody>
    </>
  );

  if (display === "inline") {
    return (
      <section
        aria-labelledby={titleId}
        className={["sdkwork-points-recharge-dialog", "sdkwork-points-recharge-inline", className].filter(Boolean).join(" ")}
      >
        {content}
      </section>
    );
  }

  return (
    <Modal onOpenChange={(open: boolean) => !open && closeDialog()} open={active}>
      <ModalContent aria-describedby={undefined} aria-labelledby={titleId} className="sdkwork-points-recharge-dialog" showCloseButton={false} size="lg">
        {content}
      </ModalContent>
    </Modal>
  );
}

export function SdkworkPointsRechargeDialog({
  isOpen,
  onClose,
  ...props
}: SdkworkPointsRechargeDialogProps) {
  return (
    <SdkworkPointsRechargeExperience
      {...props}
      active={isOpen}
      display="dialog"
      onClose={onClose}
    />
  );
}

export function SdkworkPointsRechargeInline({
  className,
  ...props
}: SdkworkPointsRechargeInlineProps) {
  return (
    <SdkworkPointsRechargeExperience
      {...props}
      active
      className={className}
      display="inline"
    />
  );
}
