/// <reference path="../styles.d.ts" />

import { useEffect, useId, useMemo, useRef, useState, type FormEvent } from "react";
import { CalendarDays, CheckCircle2, Gift, LoaderCircle, TicketCheck, X } from "lucide-react";
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
  createSdkworkCouponRedemptionService,
  createSdkworkIdempotencyParams,
  type SdkworkCouponRedemptionResult,
  type SdkworkCouponRedemptionService,
} from "@sdkwork/order-service";
import "./coupon-redemption.css";

export interface SdkworkCouponRedemptionCopy {
  close: string;
  codeLabel: string;
  codePlaceholder: string;
  cashCredited: string;
  dailyQuota: string;
  description: string;
  expiresAt: string;
  invalidCode: string;
  pointsCredited: string;
  redeem: string;
  redeeming: string;
  subscriptionActivated: string;
  title: string;
  tokenBankCredited: string;
  totalQuota: string;
}

export interface SdkworkCouponRedemptionProps {
  copy?: Partial<SdkworkCouponRedemptionCopy>;
  initialCode?: string;
  onCompleted?: (result: SdkworkCouponRedemptionResult) => Promise<void> | void;
  service?: SdkworkCouponRedemptionService;
}

export interface SdkworkCouponRedemptionDialogProps extends SdkworkCouponRedemptionProps {
  isOpen: boolean;
  onClose: () => void;
}

export interface SdkworkCouponRedemptionInlineProps extends SdkworkCouponRedemptionProps {
  className?: string;
}

const DEFAULT_COPY: SdkworkCouponRedemptionCopy = {
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
function minorUnitsToYuan(value: number): string {
  const padded = String(value).padStart(3, "0");
  const whole = padded.slice(0, -2).replace(/^0+(?=\d)/, "") || "0";
  return `${whole}.${padded.slice(-2)}`;
}

interface CouponRedemptionExperienceProps extends SdkworkCouponRedemptionProps {
  active: boolean;
  className?: string;
  display: "dialog" | "inline";
  onClose?: () => void;
}

function CouponRedemptionExperience({
  active,
  className,
  copy: copyOverrides,
  display,
  initialCode = "",
  onClose,
  onCompleted,
  service: serviceProp,
}: CouponRedemptionExperienceProps) {
  const titleId = useId();
  const inputId = useId();
  const copy = useMemo(() => ({ ...DEFAULT_COPY, ...copyOverrides }), [copyOverrides]);
  const service = useMemo(
    () => serviceProp ?? createSdkworkCouponRedemptionService(),
    [serviceProp],
  );
  const [code, setCode] = useState(initialCode);
  const [error, setError] = useState<string | null>(null);
  const [isRedeeming, setIsRedeeming] = useState(false);
  const [result, setResult] = useState<SdkworkCouponRedemptionResult | null>(null);

  /**
   * Idempotency intent for the redemption the customer confirmed: the key is
   * derived once for the entered code and reused by every retry of that same
   * code. A retry after the coupon was consumed but the benefit failed
   * therefore replays the original redemption instead of consuming another
   * coupon; a different code (or a completed redemption) starts a new intent.
   */
  const intentRef = useRef<{ code: string; idempotencyKey: string } | null>(null);

  function idempotencyKeyFor(normalizedCode: string): string {
    if (intentRef.current?.code !== normalizedCode) {
      intentRef.current = {
        code: normalizedCode,
        idempotencyKey: createSdkworkIdempotencyParams().idempotencyKey,
      };
    }
    return intentRef.current.idempotencyKey;
  }

  useEffect(() => {
    if (!active) return;
    intentRef.current = null;
    setCode(initialCode);
    setError(null);
    setIsRedeeming(false);
    setResult(null);
  }, [active, initialCode]);

  async function redeem(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (isRedeeming) return;
    const normalizedCode = code.trim();
    if (!normalizedCode) {
      setError(copy.invalidCode);
      return;
    }
    setError(null);
    setIsRedeeming(true);
    try {
      const next = await service.redeem(normalizedCode, idempotencyKeyFor(normalizedCode));
      intentRef.current = null;
      setResult(next);
      await onCompleted?.(next);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : copy.invalidCode);
    } finally {
      setIsRedeeming(false);
    }
  }

  const content = (
    <>
      <ModalHeader className="sdkwork-coupon-redemption__header">
        <div className="sdkwork-coupon-redemption__heading">
          <TicketCheck aria-hidden="true" />
          <div>
            {display === "dialog" ? (
              <ModalTitle id={titleId}>{copy.title}</ModalTitle>
            ) : (
              <h2 id={titleId}>{copy.title}</h2>
            )}
            <p>{copy.description}</p>
          </div>
        </div>
        {display === "dialog" ? (
          <ModalClose
            aria-label={copy.close}
            className="sdkwork-coupon-redemption__close"
            onClick={onClose}
          >
            <X aria-hidden="true" />
          </ModalClose>
        ) : null}
      </ModalHeader>
      <ModalBody className="sdkwork-coupon-redemption__body">
        <form className="sdkwork-coupon-redemption__form" onSubmit={redeem}>
          <label htmlFor={inputId}>{copy.codeLabel}</label>
          <div className="sdkwork-coupon-redemption__input-row">
            <input
              autoComplete="off"
              disabled={isRedeeming}
              id={inputId}
              maxLength={128}
              onChange={(event) => {
                setCode(event.target.value);
                setError(null);
                setResult(null);
              }}
              placeholder={copy.codePlaceholder}
              spellCheck={false}
              value={code}
            />
            <Button disabled={isRedeeming || !code.trim()} type="submit">
              {isRedeeming ? (
                <LoaderCircle aria-hidden="true" className="sdkwork-coupon-redemption__spinner" />
              ) : (
                <Gift aria-hidden="true" />
              )}
              {isRedeeming ? copy.redeeming : copy.redeem}
            </Button>
          </div>
        </form>
        {error ? <StatusNotice tone="danger">{error}</StatusNotice> : null}
        {result ? <CouponRedemptionResult copy={copy} result={result} /> : null}
      </ModalBody>
    </>
  );

  if (display === "inline") {
    return (
      <section
        aria-labelledby={titleId}
        className={["sdkwork-coupon-redemption", "sdkwork-coupon-redemption--inline", className]
          .filter(Boolean)
          .join(" ")}
      >
        {content}
      </section>
    );
  }
  return (
    <Modal open={active} onOpenChange={(open) => { if (!open) onClose?.(); }}>
      <ModalContent
        aria-labelledby={titleId}
        className="sdkwork-coupon-redemption sdkwork-coupon-redemption--dialog"
        showCloseButton={false}
      >
        {content}
      </ModalContent>
    </Modal>
  );
}

function CouponRedemptionResult({
  copy,
  result,
}: {
  copy: SdkworkCouponRedemptionCopy;
  result: SdkworkCouponRedemptionResult;
}) {
  if (result.benefitKind === "token_bank_credit") {
    return (
      <div className="sdkwork-coupon-redemption__result" role="status">
        <CheckCircle2 aria-hidden="true" />
        <div>
          <strong>{copy.tokenBankCredited}</strong>
          <span>{result.grantAmount.toLocaleString()}</span>
        </div>
      </div>
    );
  }
  if (result.benefitKind === "points_credit") {
    return (
      <div className="sdkwork-coupon-redemption__result" role="status">
        <CheckCircle2 aria-hidden="true" />
        <div>
          <strong>{copy.pointsCredited}</strong>
          <span>{result.grantPoints.toLocaleString()}</span>
        </div>
      </div>
    );
  }
  if (result.benefitKind === "cash_credit") {
    return (
      <div className="sdkwork-coupon-redemption__result" role="status">
        <CheckCircle2 aria-hidden="true" />
        <div>
          <strong>{copy.cashCredited}</strong>
          <span>{minorUnitsToYuan(result.grantAmount)}</span>
        </div>
      </div>
    );
  }
  return (
    <div className="sdkwork-coupon-redemption__result sdkwork-coupon-redemption__result--subscription" role="status">
      <CalendarDays aria-hidden="true" />
      <div className="sdkwork-coupon-redemption__result-content">
        <strong>{copy.subscriptionActivated}</strong>
        <dl>
          <div><dt>{copy.dailyQuota}</dt><dd>{result.dailyQuota.toLocaleString()}</dd></div>
          <div><dt>{copy.totalQuota}</dt><dd>{result.totalQuota.toLocaleString()}</dd></div>
          <div><dt>{copy.expiresAt}</dt><dd>{result.expiresAt}</dd></div>
        </dl>
      </div>
    </div>
  );
}

export function SdkworkCouponRedemptionDialog(props: SdkworkCouponRedemptionDialogProps) {
  return (
    <CouponRedemptionExperience
      {...props}
      active={props.isOpen}
      display="dialog"
    />
  );
}

export function SdkworkCouponRedemptionInline(props: SdkworkCouponRedemptionInlineProps) {
  return <CouponRedemptionExperience {...props} active display="inline" />;
}
