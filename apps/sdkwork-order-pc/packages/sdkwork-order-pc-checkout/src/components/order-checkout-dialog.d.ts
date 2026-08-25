import "./order-checkout-dialog.css";
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
/**
 * Domain-neutral order checkout UI. Product features provide the item summary
 * and checkout driver; this component owns payment QR presentation only.
 */
export declare function SdkworkOrderCheckoutDialog({ copy, driver, isOpen, onClose, summary, }: SdkworkOrderCheckoutDialogProps): import("react").JSX.Element | null;
//# sourceMappingURL=order-checkout-dialog.d.ts.map