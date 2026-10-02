import { lazy, type ReactNode, useEffect, useMemo, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { SdkworkIamAuthRoutes } from "@sdkwork/auth-pc-react";

import {
  resolveSdkworkOrderH5AuthAppearance,
  resolveSdkworkOrderH5AuthLocale,
  resolveSdkworkOrderH5AuthRuntimeConfig,
} from "./bootstrap/authConfig";
import { SDKWORK_ORDER_H5_HOME_PATH } from "./bootstrap/routes";
import type { SdkworkOrderH5Runtime } from "./bootstrap/runtime";
import {
  hasSdkworkOrderH5AuthenticatedSession,
  resolveSdkworkOrderH5AuthGateDecision,
} from "@sdkwork/order-h5-shell";

export interface AuthGateProps {
  children: ReactNode;
  runtime: SdkworkOrderH5Runtime;
}

export function AuthGate({ children, runtime }: AuthGateProps) {
  const location = useLocation();
  const navigate = useNavigate();
  const [snapshot, setSnapshot] = useState(() => runtime.session.getSnapshot());

  useEffect(() => runtime.session.subscribe(setSnapshot), [runtime.session]);

  const decision = useMemo(
    () =>
      resolveSdkworkOrderH5AuthGateDecision({
        hasSession: hasSdkworkOrderH5AuthenticatedSession(snapshot),
        homePath: SDKWORK_ORDER_H5_HOME_PATH,
        location,
      }),
    [location, snapshot],
  );

  useEffect(() => {
    if (decision.kind !== "redirect") {
      return;
    }
    navigate(decision.to, { replace: true });
  }, [decision, navigate]);

  if (decision.kind === "redirect") {
    return null;
  }

  if (decision.kind === "auth-route") {
    const authProps = {
      appearance: resolveSdkworkOrderH5AuthAppearance(),
      basePath: "/auth",
      getRuntime: () => runtime.iamRuntime,
      homePath: SDKWORK_ORDER_H5_HOME_PATH,
      locale: resolveSdkworkOrderH5AuthLocale(runtime.config.i18n.defaultLocale),
      runtimeConfig: resolveSdkworkOrderH5AuthRuntimeConfig(),
      viewportMode: "flow" as const,
    };

    return <SdkworkIamAuthRoutes {...(authProps as unknown as Parameters<typeof SdkworkIamAuthRoutes>[0])} />;
  }

  return <>{children}</>;
}
