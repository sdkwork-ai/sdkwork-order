import { type SdkworkCouponRedemptionResult, type SdkworkCouponRedemptionService } from "@sdkwork/order-service";
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
export declare function SdkworkCouponRedemptionDialog(props: SdkworkCouponRedemptionDialogProps): import("react").JSX.Element;
export declare function SdkworkCouponRedemptionInline(props: SdkworkCouponRedemptionInlineProps): import("react").JSX.Element;
//# sourceMappingURL=coupon-redemption.d.ts.map