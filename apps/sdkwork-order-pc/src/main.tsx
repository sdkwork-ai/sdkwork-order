import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import {
  bootstrapSdkworkOrderAppService,
  bootstrapSdkworkOrderBackendSdk,
  configureSdkworkOrderSessionTokenProvider,
} from "@sdkwork/order-service";
import "@sdkwork/ui-pc-react/styles.css";
import "./app.css";

import { OrderAppShell } from "@sdkwork/order-pc-shell";
import { resolveInitialThemeMode } from "./bootstrap/theme";

function readEnv(name: string): string | undefined {
  const value = (import.meta.env[name] as string | undefined)?.trim();
  return value && value.length > 0 ? value : undefined;
}

const orderApiBaseUrl =
  readEnv("VITE_SDKWORK_ORDER_API_ORIGIN")
  ?? readEnv("VITE_ORDER_API_ORIGIN")
  ?? "http://127.0.0.1:18093";

const accessToken =
  readEnv("VITE_SDKWORK_ACCESS_TOKEN")
  ?? readEnv("SDKWORK_ACCESS_TOKEN");

if (accessToken) {
  configureSdkworkOrderSessionTokenProvider(() => ({
    accessToken,
    authToken: readEnv("VITE_SDKWORK_AUTH_TOKEN") ?? readEnv("SDKWORK_AUTH_TOKEN"),
  }));
  bootstrapSdkworkOrderAppService({
    baseUrl: orderApiBaseUrl,
    accessToken,
  });
  bootstrapSdkworkOrderBackendSdk({
    baseUrl: orderApiBaseUrl,
    accessToken,
    authToken: readEnv("VITE_SDKWORK_AUTH_TOKEN") ?? readEnv("SDKWORK_AUTH_TOKEN"),
  });
}

const initialTheme = resolveInitialThemeMode();

const initialLocale = (() => {
  if (typeof document === "undefined") {
    return "zh-CN";
  }
  return document.documentElement.lang || "zh-CN";
})();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <OrderAppShell theme={initialTheme} locale={initialLocale} authConfigured={Boolean(accessToken)} />
  </StrictMode>,
);
