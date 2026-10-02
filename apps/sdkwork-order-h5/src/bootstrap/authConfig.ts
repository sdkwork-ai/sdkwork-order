import type { CSSProperties } from "react";
import type { SdkworkAuthRuntimeConfig } from "@sdkwork/auth-pc-react";

export interface SdkworkOrderH5AuthAppearanceConfig {
  asidePanelClassName?: string;
  asidePanelStyle?: CSSProperties;
  bodyClassName?: string;
  contentContainerClassName?: string;
  pageClassName?: string;
  qrFrameClassName?: string;
  shellClassName?: string;
  slotProps?: {
    asidePanel?: { className?: string; style?: CSSProperties };
    background?: { className?: string };
    page?: { className?: string };
    shell?: { className?: string };
  };
  theme?: Record<string, string>;
}

export type SdkworkOrderH5AuthRuntimeConfig = SdkworkAuthRuntimeConfig;

const ORDER_VERIFICATION_POLICY = {
  emailCodeLoginEnabled: true,
  emailRegistrationVerificationRequired: false,
  phoneCodeLoginEnabled: true,
  phoneRegistrationVerificationRequired: false,
};

export function resolveSdkworkOrderH5AuthRuntimeConfig(): SdkworkOrderH5AuthRuntimeConfig {
  return {
    leftRailMode: "qr-only",
    loginMethods: ["password", "emailCode", "phoneCode"],
    oauthLoginEnabled: false,
    oauthProviders: [],
    qrLoginEnabled: true,
    recoveryMethods: ["email", "phone"],
    registerMethods: ["email", "phone"],
    verificationPolicy: ORDER_VERIFICATION_POLICY,
  };
}

export function resolveSdkworkOrderH5AuthAppearance(): SdkworkOrderH5AuthAppearanceConfig {
  return {
    asidePanelClassName: "sdkwork-order-h5-auth-aside-panel",
    asidePanelStyle: {
      backgroundColor: "#f8fafc",
      backgroundImage: "linear-gradient(180deg, #ffffff 0%, #f1f5f9 100%)",
      color: "#0f172a",
    },
    bodyClassName: "sdkwork-order-h5-auth-body",
    contentContainerClassName: "sdkwork-order-h5-auth-content",
    pageClassName: "sdkwork-order-h5-auth-page",
    qrFrameClassName: "sdkwork-order-h5-auth-qr-frame",
    shellClassName: "sdkwork-order-h5-auth-card-shell",
    slotProps: {
      asidePanel: {
        style: {
          backgroundColor: "#f8fafc",
          backgroundImage: "linear-gradient(180deg, #ffffff 0%, #f1f5f9 100%)",
          color: "#0f172a",
        },
      },
      background: {
        className: "sdkwork-order-h5-auth-background",
      },
      page: {
        className: "sdkwork-order-h5-auth-page",
      },
      shell: {
        className: "sdkwork-order-h5-auth-card-shell",
      },
    },
  };
}

export function resolveSdkworkOrderH5AuthLocale(defaultLocale: string): string {
  return defaultLocale;
}
