/// <reference path="../styles.d.ts" />

import { useCallback, useEffect, useRef, useState } from "react";
import "./order-checkout-dialog.css";
import {
  CheckCircle2,
  QrCode,
  ShieldCheck,
  Smartphone,
  Sparkles,
  X,
} from "lucide-react";
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

export type SdkworkOrderCheckoutPaymentStatus = "completed" | "failed" | "pending";

export interface SdkworkOrderCheckoutPayment {
  amountCny?: number | null;
  cashierUrl?: string;
  expiresAt?: string;
  orderId?: string;
  qrCode?: string;
  status: SdkworkOrderCheckoutPaymentStatus;
}

export interface SdkworkOrderCheckoutSummary {
  id: string;
  name: string;
  originalPriceLabel?: string;
  periodLabel?: string;
  priceLabel: string;
}

export interface SdkworkOrderCheckoutDialogCopy {
  activationDescription: string;
  activationTitle: string;
  close: string;
  completed: string;
  creatingPayment: string;
  expired?: string;
  expiredDescription?: string;
  expiresIn?: string;
  openPaymentLink?: string;
  paymentUnavailable: string;
  paymentUnavailableDescription: string;
  payByQr: string;
  price: string;
  retry: string;
  scanPrompt: string;
  secureDescription: string;
  secureTitle: string;
  selectedItem: string;
  title?: string;
}

export interface SdkworkOrderCheckoutDriver {
  createPayment(): Promise<SdkworkOrderCheckoutPayment>;
  getPaymentStatus?(payment: SdkworkOrderCheckoutPayment): Promise<SdkworkOrderCheckoutPayment>;
  onPaymentCompleted?(payment: SdkworkOrderCheckoutPayment): Promise<void> | void;
  pollIntervalMs?: number;
}

export interface SdkworkOrderCheckoutDialogProps {
  copy: SdkworkOrderCheckoutDialogCopy;
  driver: SdkworkOrderCheckoutDriver;
  isOpen: boolean;
  onClose: () => void;
  summary: SdkworkOrderCheckoutSummary | null;
}

function isImageDataUrl(value: string | undefined): value is string {
  return Boolean(value?.startsWith("data:image/"));
}

/**
 * Only http(s) targets are scannable by a generic phone camera. Provider
 * deep links (a Stripe cashier URL) must surface as a launch
 * button instead of an unscannable QR code.
 */
function isScannableUrl(value: string | undefined): value is string {
  if (!value) {
    return false;
  }
  try {
    const url = new URL(value);
    return url.protocol === "https:" || url.protocol === "http:";
  } catch {
    return false;
  }
}

function parseExpirationTime(value: string | undefined): number | null {
  if (!value) {
    return null;
  }

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

/**
 * Domain-neutral order checkout UI. Product features provide the item summary
 * and checkout driver; this component owns payment QR presentation only.
 */
export function SdkworkOrderCheckoutDialog({
  copy,
  driver,
  isOpen,
  onClose,
  summary,
}: SdkworkOrderCheckoutDialogProps) {
  const createPaymentRef = useRef(driver.createPayment);
  const getPaymentStatusRef = useRef(driver.getPaymentStatus);
  const onPaymentCompletedRef = useRef(driver.onPaymentCompleted);
  const pollIntervalRef = useRef(driver.pollIntervalMs);
  const copyRef = useRef(copy);
  const completedPaymentKeyRef = useRef<string | null>(null);
  const summaryId = summary?.id;
  const [attempt, setAttempt] = useState(0);
  const [isCreatingPayment, setIsCreatingPayment] = useState(false);
  const [payment, setPayment] = useState<SdkworkOrderCheckoutPayment | null>(null);
  const [paymentError, setPaymentError] = useState<string | null>(null);
  const [qrImageUrl, setQrImageUrl] = useState<string | null>(null);
  const [qrImagePayload, setQrImagePayload] = useState<string | null>(null);
  const [currentTimeMs, setCurrentTimeMs] = useState(() => Date.now());
  const expirationTimeMs = parseExpirationTime(payment?.expiresAt);
  const remainingSeconds = expirationTimeMs === null
    ? null
    : Math.max(0, Math.ceil((expirationTimeMs - currentTimeMs) / 1_000));
  const isExpired = remainingSeconds === 0;
  const dialogTitle = copy.title ?? copy.payByQr;
  const expiredDescription = copy.expiredDescription ?? copy.paymentUnavailableDescription;
  const expiredTitle = copy.expired ?? copy.paymentUnavailable;
  const expiresIn = copy.expiresIn ?? "Order expires in";

  createPaymentRef.current = driver.createPayment;
  getPaymentStatusRef.current = driver.getPaymentStatus;
  onPaymentCompletedRef.current = driver.onPaymentCompleted;
  pollIntervalRef.current = driver.pollIntervalMs;
  copyRef.current = copy;

  const notifyPaymentCompleted = useCallback((result: SdkworkOrderCheckoutPayment) => {
    const paymentKey = result.orderId ?? summaryId;
    if (!paymentKey || completedPaymentKeyRef.current === paymentKey) {
      return;
    }

    completedPaymentKeyRef.current = paymentKey;
    const onPaymentCompleted = onPaymentCompletedRef.current;
    if (onPaymentCompleted) {
      void Promise.resolve(onPaymentCompleted(result)).catch(() => undefined);
    }
  }, [summaryId]);

  const retryPayment = useCallback(async () => {
    if (isExpired) {
      setAttempt((current) => current + 1);
      return;
    }

    const currentPayment = payment;
    const getPaymentStatus = getPaymentStatusRef.current;
    if (!currentPayment?.orderId || !getPaymentStatus) {
      setAttempt((current) => current + 1);
      return;
    }

    setIsCreatingPayment(true);
    setPaymentError(null);
    try {
      const update = await getPaymentStatus(currentPayment);
      const nextPayment = {
        ...currentPayment,
        ...update,
        orderId: update.orderId ?? currentPayment.orderId,
      };
      const qrCode = nextPayment.qrCode?.trim() || nextPayment.cashierUrl?.trim();
      const normalizedPayment = qrCode ? { ...nextPayment, qrCode } : nextPayment;
      setCurrentTimeMs(Date.now());
      setPayment(normalizedPayment);
      if (normalizedPayment.status === "completed") {
        notifyPaymentCompleted(normalizedPayment);
        return;
      }
      if (normalizedPayment.status === "pending" && normalizedPayment.qrCode) {
        return;
      }
      setAttempt((current) => current + 1);
    } catch {
      setPaymentError(copyRef.current.paymentUnavailableDescription);
    } finally {
      setIsCreatingPayment(false);
    }
  }, [isExpired, notifyPaymentCompleted, payment]);

  useEffect(() => {
    completedPaymentKeyRef.current = null;
    if (!isOpen) {
      setAttempt(0);
      setIsCreatingPayment(false);
      setPayment(null);
      setPaymentError(null);
      setQrImageUrl(null);
      setQrImagePayload(null);
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen || !summary) {
      return undefined;
    }

    let active = true;
    setIsCreatingPayment(true);
    setPayment(null);
    setPaymentError(null);
    setQrImageUrl(null);
    setQrImagePayload(null);

    void createPaymentRef.current()
      .then((result) => {
        if (!active) {
          return;
        }

        const qrCode = result.qrCode?.trim() || result.cashierUrl?.trim();
        const normalizedResult = qrCode
          ? { ...result, qrCode }
          : result;
        setCurrentTimeMs(Date.now());
        setPayment(normalizedResult);
        if (normalizedResult.status === "failed") {
          setPaymentError(copyRef.current.paymentUnavailableDescription);
        } else if (!normalizedResult.qrCode && normalizedResult.status !== "completed") {
          setPaymentError(copyRef.current.paymentUnavailableDescription);
        } else if (normalizedResult.status === "completed") {
          notifyPaymentCompleted(normalizedResult);
        }
      })
      .catch((error) => {
        if (active) {
          // Provider details are not safe or locale-stable UI copy. Keep the
          // checkout surface on the package-owned localized error message.
          void error;
          setPaymentError(copyRef.current.paymentUnavailableDescription);
        }
      })
      .finally(() => {
        if (active) {
          setIsCreatingPayment(false);
        }
      });

    return () => {
      active = false;
    };
  }, [attempt, isOpen, notifyPaymentCompleted, summaryId]);

  useEffect(() => {
    if (
      !isOpen
      || payment?.status !== "pending"
      || expirationTimeMs === null
    ) {
      return undefined;
    }

    const updateCurrentTime = () => setCurrentTimeMs(Date.now());
    updateCurrentTime();
    if (expirationTimeMs <= Date.now()) {
      return undefined;
    }

    const interval = window.setInterval(updateCurrentTime, 1_000);
    return () => window.clearInterval(interval);
  }, [expirationTimeMs, isOpen, payment?.status]);

  useEffect(() => {
    if (
      !isOpen
      || payment?.status !== "pending"
      || isExpired
      || !payment.orderId
      || !getPaymentStatusRef.current
    ) {
      return undefined;
    }

    let active = true;
    let isPolling = false;
    const currentPayment = payment;
    const poll = async () => {
      if (isPolling) {
        return;
      }

      const getPaymentStatus = getPaymentStatusRef.current;
      if (!getPaymentStatus) {
        return;
      }

      isPolling = true;
      try {
        const update = await getPaymentStatus(currentPayment);
        if (!active) {
          return;
        }

        const nextPayment = {
          ...currentPayment,
          ...update,
          orderId: update.orderId ?? currentPayment.orderId,
        };
        setCurrentTimeMs(Date.now());
        setPayment((current) => (
          current?.orderId === currentPayment.orderId ? nextPayment : current
        ));

        if (nextPayment.status === "completed") {
          notifyPaymentCompleted(nextPayment);
        } else if (nextPayment.status === "failed") {
          setPaymentError(copyRef.current.paymentUnavailableDescription);
        }
      } catch {
        // Keep the valid QR code visible and retry a transient status read.
      } finally {
        isPolling = false;
      }
    };

    void poll();
    const interval = window.setInterval(
      () => {
        void poll();
      },
      Math.max(1_000, Math.round(pollIntervalRef.current ?? 2_500)),
    );

    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [isExpired, isOpen, notifyPaymentCompleted, payment?.expiresAt, payment?.orderId, payment?.status]);

  // One final authoritative check when the countdown expires: a payment
  // confirmed at the last second must not be presented as expired. Runs
  // once per payment attempt (keyed by the attempt's expiresAt).
  const finalCheckForRef = useRef<string | null>(null);
  useEffect(() => {
    if (
      !isOpen
      || !isExpired
      || payment?.status !== "pending"
      || !payment.orderId
      || !getPaymentStatusRef.current
    ) {
      return;
    }
    if (finalCheckForRef.current === (payment.expiresAt ?? null)) {
      return;
    }
    finalCheckForRef.current = payment.expiresAt ?? null;
    const getPaymentStatus = getPaymentStatusRef.current;
    const currentPayment = payment;
    void (async () => {
      try {
        const update = await getPaymentStatus(currentPayment);
        const nextPayment = {
          ...currentPayment,
          ...update,
          orderId: update.orderId ?? currentPayment.orderId,
        };
        setPayment(nextPayment);
        if (nextPayment.status === "completed") {
          notifyPaymentCompleted(nextPayment);
        }
      } catch {
        // Keep the expired presentation when the final check also fails.
      }
    })();
  }, [isExpired, isOpen, notifyPaymentCompleted, payment?.expiresAt, payment?.orderId, payment?.status]);

  useEffect(() => {
    if (!payment?.qrCode) {
      setQrImageUrl(null);
      setQrImagePayload(null);
      return undefined;
    }

    if (payment.status === "pending") {
      setPaymentError(null);
    }

    if (isImageDataUrl(payment.qrCode)) {
      setQrImageUrl(payment.qrCode);
      setQrImagePayload(payment.qrCode);
      return undefined;
    }

    let active = true;
    const qrCodePayload = payment.qrCode;
    setQrImageUrl(null);
    setQrImagePayload(null);
    void toDataURL(payment.qrCode, {
      errorCorrectionLevel: "M",
      margin: 1,
      width: 256,
    })
      .then((value) => {
        if (active) {
          setQrImageUrl(value);
          setQrImagePayload(qrCodePayload);
        }
      })
      .catch(() => {
        if (active) {
          setPaymentError(copyRef.current.paymentUnavailableDescription);
        }
      });

    return () => {
      active = false;
    };
  }, [payment?.qrCode]);

  if (!isOpen || !summary) {
    return null;
  }

  const isCompleted = payment?.status === "completed";
  const isPreparingQr = (
    payment?.status === "pending"
    && !isExpired
    && Boolean(payment.qrCode)
    && qrImagePayload !== payment.qrCode
    && !paymentError
  );
  const canScan = (
    payment?.status === "pending"
    && !isExpired
    && Boolean(qrImageUrl)
    && qrImagePayload === payment.qrCode
  );

  return (
    <Modal
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      open={isOpen}
    >
      <ModalContent
        aria-describedby={undefined}
        aria-labelledby="sdkwork-order-checkout-title"
        className="sdkwork-order-checkout-dialog"
        showCloseButton={false}
        size="lg"
      >
        <ModalHeader className="sdkwork-order-checkout-dialog__header">
          <ModalTitle className="sdkwork-order-checkout-dialog__title" id="sdkwork-order-checkout-title">
            {dialogTitle}
          </ModalTitle>
          <ModalClose
            aria-label={copy.close}
            className="sdkwork-order-checkout-dialog__close"
          >
            <X aria-hidden="true" className="sdkwork-order-checkout-dialog__close-icon" />
          </ModalClose>
        </ModalHeader>
        <ModalBody className="sdkwork-order-checkout-dialog__body">
          <div
            className="sdkwork-order-checkout-dialog__summary"
            data-sdk-region="order-checkout-summary"
          >
            <div className="sdkwork-order-checkout-dialog__summary-card">
              <div className="sdkwork-order-checkout-dialog__summary-header">
                <div>
                  <div className="sdkwork-order-checkout-dialog__label">{copy.selectedItem}</div>
                  <div className="sdkwork-order-checkout-dialog__plan-name">{summary.name}</div>
                  {summary.periodLabel ? (
                    <div className="sdkwork-order-checkout-dialog__period">{summary.periodLabel}</div>
                  ) : null}
                </div>
                <div className="sdkwork-order-checkout-dialog__price-block">
                  <div className="sdkwork-order-checkout-dialog__label">{copy.price}</div>
                  <div className="sdkwork-order-checkout-dialog__price">
                    {summary.priceLabel}
                  </div>
                  {summary.originalPriceLabel ? (
                    <div className="sdkwork-order-checkout-dialog__original-price">
                      {summary.originalPriceLabel}
                    </div>
                  ) : null}
                </div>
              </div>
            </div>
            <div className="sdkwork-order-checkout-dialog__benefit-grid">
              <div className="sdkwork-order-checkout-dialog__benefit-card">
                <div className="sdkwork-order-checkout-dialog__benefit-title">
                  <ShieldCheck aria-hidden="true" className="sdkwork-order-checkout-dialog__benefit-icon sdkwork-order-checkout-dialog__benefit-icon--secure" />
                  {copy.secureTitle}
                </div>
                <p className="sdkwork-order-checkout-dialog__benefit-description">{copy.secureDescription}</p>
              </div>
              <div className="sdkwork-order-checkout-dialog__benefit-card">
                <div className="sdkwork-order-checkout-dialog__benefit-title">
                  <Sparkles aria-hidden="true" className="sdkwork-order-checkout-dialog__benefit-icon sdkwork-order-checkout-dialog__benefit-icon--activation" />
                  {copy.activationTitle}
                </div>
                <p className="sdkwork-order-checkout-dialog__benefit-description">{copy.activationDescription}</p>
              </div>
            </div>
          </div>
          <aside
            className="sdkwork-order-checkout-dialog__payment-panel"
            data-sdk-region="order-checkout-payment"
          >
            <p className="sdkwork-order-checkout-dialog__payment-label">{copy.payByQr}</p>
            {payment?.status === "pending" && remainingSeconds !== null && !isExpired ? (
              <p className="sdkwork-order-checkout-dialog__countdown" role="timer">
                <span>{expiresIn}</span>
                <strong>{formatRemainingTime(remainingSeconds)}</strong>
              </p>
            ) : null}
            {isCreatingPayment || isPreparingQr ? (
              <div className="sdkwork-order-checkout-dialog__pending">
                <QrCode aria-hidden="true" className="sdkwork-order-checkout-dialog__pending-icon" />
                <span>{copy.creatingPayment}</span>
              </div>
            ) : null}
            {!isCreatingPayment && !isPreparingQr && canScan ? (
              <div className="sdkwork-order-checkout-dialog__qr-wrap">
                <img
                  alt={copy.scanPrompt}
                  className="sdkwork-order-checkout-dialog__qr-image"
                  src={qrImageUrl ?? undefined}
                />
                <p className="sdkwork-order-checkout-dialog__scan-prompt">
                  <Smartphone aria-hidden="true" className="sdkwork-order-checkout-dialog__scan-icon" />
                  {copy.scanPrompt}
                </p>
              </div>
            ) : null}
            {!isCreatingPayment && !isPreparingQr && isCompleted ? (
              <div className="sdkwork-order-checkout-dialog__completed">
                <CheckCircle2 aria-hidden="true" className="sdkwork-order-checkout-dialog__completed-icon" />
                <span className="sdkwork-order-checkout-dialog__completed-label">{copy.completed}</span>
                <Button onClick={onClose} type="button">
                  {copy.close}
                </Button>
              </div>
            ) : null}
            {!isCreatingPayment && !isPreparingQr && !canScan && !isCompleted ? (
              <div className="sdkwork-order-checkout-dialog__unavailable">
                <StatusNotice tone="danger" title={isExpired ? expiredTitle : copy.paymentUnavailable}>
                  <span className="sdkwork-order-checkout-dialog__error-copy">
                    {isExpired
                      ? expiredDescription
                      : paymentError ?? copy.paymentUnavailableDescription}
                  </span>
                </StatusNotice>
                {payment?.qrCode && !isScannableUrl(payment.qrCode) ? (
                  <Button
                    className="sdkwork-order-checkout-dialog__retry"
                    onClick={() => window.open(payment.qrCode, "_blank", "noopener,noreferrer")}
                    type="button"
                  >
                    {copy.openPaymentLink ?? "Open payment page"}
                  </Button>
                ) : null}
                <Button
                  className="sdkwork-order-checkout-dialog__retry"
                  onClick={() => void retryPayment()}
                  type="button"
                  variant="secondary"
                >
                  {copy.retry}
                </Button>
              </div>
            ) : null}
          </aside>
        </ModalBody>
      </ModalContent>
    </Modal>
  );
}
