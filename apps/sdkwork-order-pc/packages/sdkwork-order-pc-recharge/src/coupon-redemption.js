import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
/// <reference path="../styles.d.ts" />
import { useEffect, useId, useMemo, useState } from "react";
import { CalendarDays, CheckCircle2, Gift, LoaderCircle, TicketCheck, X } from "lucide-react";
import { Button, Modal, ModalBody, ModalClose, ModalContent, ModalHeader, ModalTitle, StatusNotice, } from "@sdkwork/ui-pc-react";
import { createSdkworkCouponRedemptionService, } from "@sdkwork/order-service";
import "./coupon-redemption.css";
const DEFAULT_COPY = {
    cashCredited: "Cash balance credited",
    close: "Close",
    codeLabel: "Coupon code",
    codePlaceholder: "Enter your coupon code",
    dailyQuota: "Daily quota",
    description: "Redeem Token Bank credit or activate a quota-limited subscription.",
    expiresAt: "Valid until",
    invalidCode: "Enter a valid coupon code.",
    pointsCredited: "Points credited",
    redeem: "Redeem",
    redeeming: "Redeeming...",
    subscriptionActivated: "Subscription activated",
    title: "Redeem coupon",
    tokenBankCredited: "Token Bank credited",
    totalQuota: "Total quota",
};
/** 现金券最小单位（分）→ 元金额字符串（最多两位小数）。 */
function minorUnitsToYuan(value) {
    const padded = String(value).padStart(3, "0");
    const whole = padded.slice(0, -2).replace(/^0+(?=\d)/, "") || "0";
    return `${whole}.${padded.slice(-2)}`;
}
function CouponRedemptionExperience({ active, className, copy: copyOverrides, display, initialCode = "", onClose, onCompleted, service: serviceProp, }) {
    const titleId = useId();
    const inputId = useId();
    const copy = useMemo(() => ({ ...DEFAULT_COPY, ...copyOverrides }), [copyOverrides]);
    const service = useMemo(() => serviceProp ?? createSdkworkCouponRedemptionService(), [serviceProp]);
    const [code, setCode] = useState(initialCode);
    const [error, setError] = useState(null);
    const [isRedeeming, setIsRedeeming] = useState(false);
    const [result, setResult] = useState(null);
    useEffect(() => {
        if (!active)
            return;
        setCode(initialCode);
        setError(null);
        setIsRedeeming(false);
        setResult(null);
    }, [active, initialCode]);
    async function redeem(event) {
        event.preventDefault();
        if (isRedeeming)
            return;
        const normalizedCode = code.trim();
        if (!normalizedCode) {
            setError(copy.invalidCode);
            return;
        }
        setError(null);
        setIsRedeeming(true);
        try {
            const next = await service.redeem(normalizedCode);
            setResult(next);
            await onCompleted?.(next);
        }
        catch (cause) {
            setError(cause instanceof Error ? cause.message : copy.invalidCode);
        }
        finally {
            setIsRedeeming(false);
        }
    }
    const content = (_jsxs(_Fragment, { children: [_jsxs(ModalHeader, { className: "sdkwork-coupon-redemption__header", children: [_jsxs("div", { className: "sdkwork-coupon-redemption__heading", children: [_jsx(TicketCheck, { "aria-hidden": "true" }), _jsxs("div", { children: [display === "dialog" ? (_jsx(ModalTitle, { id: titleId, children: copy.title })) : (_jsx("h2", { id: titleId, children: copy.title })), _jsx("p", { children: copy.description })] })] }), display === "dialog" ? (_jsx(ModalClose, { "aria-label": copy.close, className: "sdkwork-coupon-redemption__close", onClick: onClose, children: _jsx(X, { "aria-hidden": "true" }) })) : null] }), _jsxs(ModalBody, { className: "sdkwork-coupon-redemption__body", children: [_jsxs("form", { className: "sdkwork-coupon-redemption__form", onSubmit: redeem, children: [_jsx("label", { htmlFor: inputId, children: copy.codeLabel }), _jsxs("div", { className: "sdkwork-coupon-redemption__input-row", children: [_jsx("input", { autoComplete: "off", disabled: isRedeeming, id: inputId, maxLength: 128, onChange: (event) => {
                                            setCode(event.target.value);
                                            setError(null);
                                            setResult(null);
                                        }, placeholder: copy.codePlaceholder, spellCheck: false, value: code }), _jsxs(Button, { disabled: isRedeeming || !code.trim(), type: "submit", children: [isRedeeming ? (_jsx(LoaderCircle, { "aria-hidden": "true", className: "sdkwork-coupon-redemption__spinner" })) : (_jsx(Gift, { "aria-hidden": "true" })), isRedeeming ? copy.redeeming : copy.redeem] })] })] }), error ? _jsx(StatusNotice, { tone: "danger", children: error }) : null, result ? _jsx(CouponRedemptionResult, { copy: copy, result: result }) : null] })] }));
    if (display === "inline") {
        return (_jsx("section", { "aria-labelledby": titleId, className: ["sdkwork-coupon-redemption", "sdkwork-coupon-redemption--inline", className]
                .filter(Boolean)
                .join(" "), children: content }));
    }
    return (_jsx(Modal, { open: active, onOpenChange: (open) => { if (!open)
            onClose?.(); }, children: _jsx(ModalContent, { "aria-labelledby": titleId, className: "sdkwork-coupon-redemption sdkwork-coupon-redemption--dialog", showCloseButton: false, children: content }) }));
}
function CouponRedemptionResult({ copy, result, }) {
    if (result.benefitKind === "token_bank_credit") {
        return (_jsxs("div", { className: "sdkwork-coupon-redemption__result", role: "status", children: [_jsx(CheckCircle2, { "aria-hidden": "true" }), _jsxs("div", { children: [_jsx("strong", { children: copy.tokenBankCredited }), _jsx("span", { children: result.grantAmount.toLocaleString() })] })] }));
    }
    if (result.benefitKind === "points_credit") {
        return (_jsxs("div", { className: "sdkwork-coupon-redemption__result", role: "status", children: [_jsx(CheckCircle2, { "aria-hidden": "true" }), _jsxs("div", { children: [_jsx("strong", { children: copy.pointsCredited }), _jsx("span", { children: result.grantPoints.toLocaleString() })] })] }));
    }
    if (result.benefitKind === "cash_credit") {
        return (_jsxs("div", { className: "sdkwork-coupon-redemption__result", role: "status", children: [_jsx(CheckCircle2, { "aria-hidden": "true" }), _jsxs("div", { children: [_jsx("strong", { children: copy.cashCredited }), _jsx("span", { children: minorUnitsToYuan(result.grantAmount) })] })] }));
    }
    return (_jsxs("div", { className: "sdkwork-coupon-redemption__result sdkwork-coupon-redemption__result--subscription", role: "status", children: [_jsx(CalendarDays, { "aria-hidden": "true" }), _jsxs("div", { className: "sdkwork-coupon-redemption__result-content", children: [_jsx("strong", { children: copy.subscriptionActivated }), _jsxs("dl", { children: [_jsxs("div", { children: [_jsx("dt", { children: copy.dailyQuota }), _jsx("dd", { children: result.dailyQuota.toLocaleString() })] }), _jsxs("div", { children: [_jsx("dt", { children: copy.totalQuota }), _jsx("dd", { children: result.totalQuota.toLocaleString() })] }), _jsxs("div", { children: [_jsx("dt", { children: copy.expiresAt }), _jsx("dd", { children: result.expiresAt })] })] })] })] }));
}
export function SdkworkCouponRedemptionDialog(props) {
    return (_jsx(CouponRedemptionExperience, { ...props, active: props.isOpen, display: "dialog" }));
}
export function SdkworkCouponRedemptionInline(props) {
    return _jsx(CouponRedemptionExperience, { ...props, active: true, display: "inline" });
}
//# sourceMappingURL=coupon-redemption.js.map