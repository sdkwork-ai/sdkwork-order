import { type SdkworkPointsRechargePayment, type SdkworkPointsRechargeService } from "@sdkwork/order-service";
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
export declare function SdkworkPointsRechargeDialog({ isOpen, onClose, ...props }: SdkworkPointsRechargeDialogProps): import("react").JSX.Element;
export declare function SdkworkPointsRechargeInline({ className, ...props }: SdkworkPointsRechargeInlineProps): import("react").JSX.Element;
//# sourceMappingURL=points-recharge-dialog.d.ts.map