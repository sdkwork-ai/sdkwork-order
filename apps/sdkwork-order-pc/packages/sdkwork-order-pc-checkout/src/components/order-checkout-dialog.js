import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/// <reference path="../styles.d.ts" />
import { useCallback, useEffect, useRef, useState } from "react";
import "./order-checkout-dialog.css";
import { CheckCircle2, QrCode, ShieldCheck, Smartphone, Sparkles, X, } from "lucide-react";
import { toDataURL } from "qrcode";
import { Button, Modal, ModalBody, ModalClose, ModalContent, ModalHeader, ModalTitle, StatusNotice, } from "@sdkwork/ui-pc-react";
function isImageDataUrl(value) {
    return Boolean(value?.startsWith("data:image/"));
}
function parseExpirationTime(value) {
    if (!value) {
        return null;
    }
    const timestamp = Date.parse(value);
    return Number.isFinite(timestamp) ? timestamp : null;
}
function formatRemainingTime(totalSeconds) {
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
export function SdkworkOrderCheckoutDialog({ copy, driver, isOpen, onClose, summary, }) {
    const createPaymentRef = useRef(driver.createPayment);
    const getPaymentStatusRef = useRef(driver.getPaymentStatus);
    const onPaymentCompletedRef = useRef(driver.onPaymentCompleted);
    const pollIntervalRef = useRef(driver.pollIntervalMs);
    const copyRef = useRef(copy);
    const completedPaymentKeyRef = useRef(null);
    const summaryId = summary?.id;
    const [attempt, setAttempt] = useState(0);
    const [isCreatingPayment, setIsCreatingPayment] = useState(false);
    const [payment, setPayment] = useState(null);
    const [paymentError, setPaymentError] = useState(null);
    const [qrImageUrl, setQrImageUrl] = useState(null);
    const [qrImagePayload, setQrImagePayload] = useState(null);
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
    const notifyPaymentCompleted = useCallback((result) => {
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
        }
        catch {
            setPaymentError(copyRef.current.paymentUnavailableDescription);
        }
        finally {
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
            }
            else if (!normalizedResult.qrCode && normalizedResult.status !== "completed") {
                setPaymentError(copyRef.current.paymentUnavailableDescription);
            }
            else if (normalizedResult.status === "completed") {
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
        if (!isOpen
            || payment?.status !== "pending"
            || expirationTimeMs === null) {
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
        if (!isOpen
            || payment?.status !== "pending"
            || isExpired
            || !payment.orderId
            || !getPaymentStatusRef.current) {
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
                setPayment((current) => (current?.orderId === currentPayment.orderId ? nextPayment : current));
                if (nextPayment.status === "completed") {
                    notifyPaymentCompleted(nextPayment);
                }
                else if (nextPayment.status === "failed") {
                    setPaymentError(copyRef.current.paymentUnavailableDescription);
                }
            }
            catch {
                // Keep the valid QR code visible and retry a transient status read.
            }
            finally {
                isPolling = false;
            }
        };
        void poll();
        const interval = window.setInterval(() => {
            void poll();
        }, Math.max(1_000, Math.round(pollIntervalRef.current ?? 2_500)));
        return () => {
            active = false;
            window.clearInterval(interval);
        };
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
    const isPreparingQr = (payment?.status === "pending"
        && !isExpired
        && Boolean(payment.qrCode)
        && qrImagePayload !== payment.qrCode
        && !paymentError);
    const canScan = (payment?.status === "pending"
        && !isExpired
        && Boolean(qrImageUrl)
        && qrImagePayload === payment.qrCode);
    return (_jsx(Modal, { onOpenChange: (open) => {
            if (!open) {
                onClose();
            }
        }, open: isOpen, children: _jsxs(ModalContent, { "aria-describedby": undefined, "aria-labelledby": "sdkwork-order-checkout-title", className: "sdkwork-order-checkout-dialog", showCloseButton: false, size: "lg", children: [_jsxs(ModalHeader, { className: "sdkwork-order-checkout-dialog__header", children: [_jsx(ModalTitle, { className: "sdkwork-order-checkout-dialog__title", id: "sdkwork-order-checkout-title", children: dialogTitle }), _jsx(ModalClose, { "aria-label": copy.close, className: "sdkwork-order-checkout-dialog__close", children: _jsx(X, { "aria-hidden": "true", className: "sdkwork-order-checkout-dialog__close-icon" }) })] }), _jsxs(ModalBody, { className: "sdkwork-order-checkout-dialog__body", children: [_jsxs("div", { className: "sdkwork-order-checkout-dialog__summary", "data-sdk-region": "order-checkout-summary", children: [_jsx("div", { className: "sdkwork-order-checkout-dialog__summary-card", children: _jsxs("div", { className: "sdkwork-order-checkout-dialog__summary-header", children: [_jsxs("div", { children: [_jsx("div", { className: "sdkwork-order-checkout-dialog__label", children: copy.selectedItem }), _jsx("div", { className: "sdkwork-order-checkout-dialog__plan-name", children: summary.name }), summary.periodLabel ? (_jsx("div", { className: "sdkwork-order-checkout-dialog__period", children: summary.periodLabel })) : null] }), _jsxs("div", { className: "sdkwork-order-checkout-dialog__price-block", children: [_jsx("div", { className: "sdkwork-order-checkout-dialog__label", children: copy.price }), _jsx("div", { className: "sdkwork-order-checkout-dialog__price", children: summary.priceLabel }), summary.originalPriceLabel ? (_jsx("div", { className: "sdkwork-order-checkout-dialog__original-price", children: summary.originalPriceLabel })) : null] })] }) }), _jsxs("div", { className: "sdkwork-order-checkout-dialog__benefit-grid", children: [_jsxs("div", { className: "sdkwork-order-checkout-dialog__benefit-card", children: [_jsxs("div", { className: "sdkwork-order-checkout-dialog__benefit-title", children: [_jsx(ShieldCheck, { "aria-hidden": "true", className: "sdkwork-order-checkout-dialog__benefit-icon sdkwork-order-checkout-dialog__benefit-icon--secure" }), copy.secureTitle] }), _jsx("p", { className: "sdkwork-order-checkout-dialog__benefit-description", children: copy.secureDescription })] }), _jsxs("div", { className: "sdkwork-order-checkout-dialog__benefit-card", children: [_jsxs("div", { className: "sdkwork-order-checkout-dialog__benefit-title", children: [_jsx(Sparkles, { "aria-hidden": "true", className: "sdkwork-order-checkout-dialog__benefit-icon sdkwork-order-checkout-dialog__benefit-icon--activation" }), copy.activationTitle] }), _jsx("p", { className: "sdkwork-order-checkout-dialog__benefit-description", children: copy.activationDescription })] })] })] }), _jsxs("aside", { className: "sdkwork-order-checkout-dialog__payment-panel", "data-sdk-region": "order-checkout-payment", children: [_jsx("p", { className: "sdkwork-order-checkout-dialog__payment-label", children: copy.payByQr }), payment?.status === "pending" && remainingSeconds !== null && !isExpired ? (_jsxs("p", { className: "sdkwork-order-checkout-dialog__countdown", role: "timer", children: [_jsx("span", { children: expiresIn }), _jsx("strong", { children: formatRemainingTime(remainingSeconds) })] })) : null, isCreatingPayment || isPreparingQr ? (_jsxs("div", { className: "sdkwork-order-checkout-dialog__pending", children: [_jsx(QrCode, { "aria-hidden": "true", className: "sdkwork-order-checkout-dialog__pending-icon" }), _jsx("span", { children: copy.creatingPayment })] })) : null, !isCreatingPayment && !isPreparingQr && canScan ? (_jsxs("div", { className: "sdkwork-order-checkout-dialog__qr-wrap", children: [_jsx("img", { alt: copy.scanPrompt, className: "sdkwork-order-checkout-dialog__qr-image", src: qrImageUrl ?? undefined }), _jsxs("p", { className: "sdkwork-order-checkout-dialog__scan-prompt", children: [_jsx(Smartphone, { "aria-hidden": "true", className: "sdkwork-order-checkout-dialog__scan-icon" }), copy.scanPrompt] })] })) : null, !isCreatingPayment && !isPreparingQr && isCompleted ? (_jsxs("div", { className: "sdkwork-order-checkout-dialog__completed", children: [_jsx(CheckCircle2, { "aria-hidden": "true", className: "sdkwork-order-checkout-dialog__completed-icon" }), _jsx("span", { className: "sdkwork-order-checkout-dialog__completed-label", children: copy.completed }), _jsx(Button, { onClick: onClose, type: "button", children: copy.close })] })) : null, !isCreatingPayment && !isPreparingQr && !canScan && !isCompleted ? (_jsxs("div", { className: "sdkwork-order-checkout-dialog__unavailable", children: [_jsx(StatusNotice, { tone: "danger", title: isExpired ? expiredTitle : copy.paymentUnavailable, children: _jsx("span", { className: "sdkwork-order-checkout-dialog__error-copy", children: isExpired
                                                    ? expiredDescription
                                                    : paymentError ?? copy.paymentUnavailableDescription }) }), _jsx(Button, { className: "sdkwork-order-checkout-dialog__retry", onClick: () => void retryPayment(), type: "button", variant: "secondary", children: copy.retry })] })) : null] })] })] }) }));
}
//# sourceMappingURL=order-checkout-dialog.js.map