import type { Location } from "react-router-dom";

import { hasSdkworkOrderH5IamSession, type SdkworkOrderH5SessionSnapshot } from "./sessionStore";

export type SdkworkOrderH5AuthGateDecision =
  | { kind: "product-route" }
  | { kind: "auth-route" }
  | { kind: "redirect"; replace: true; to: string };

const AUTH_BASE_PATH = "/auth";
const AUTH_LOGIN_PATH = "/auth/login";
const DEFAULT_HOME_PATH = "/orders";

export function hasSdkworkOrderH5AuthenticatedSession(
  snapshot: SdkworkOrderH5SessionSnapshot,
): boolean {
  return hasSdkworkOrderH5IamSession(snapshot);
}

export function buildSdkworkOrderH5AuthLoginRedirect(location: Pick<Location, "pathname" | "search" | "hash">): string {
  const returnPath = `${normalizePathname(location.pathname)}${location.search ?? ""}${location.hash ?? ""}`;
  return `${AUTH_LOGIN_PATH}?redirect=${encodeURIComponent(returnPath)}`;
}

export function sanitizeSdkworkOrderH5AuthRedirect(value: string | null | undefined): string {
  if (!value) {
    return DEFAULT_HOME_PATH;
  }

  let decoded = value;
  try {
    decoded = decodeURIComponent(value);
  } catch {
    return DEFAULT_HOME_PATH;
  }

  if (!decoded.startsWith("/") || decoded.startsWith("//")) {
    return DEFAULT_HOME_PATH;
  }

  const redirectUrl = new URL(decoded, "http://sdkwork-order.local");
  if (isAuthRoute(redirectUrl.pathname)) {
    return DEFAULT_HOME_PATH;
  }

  return `${redirectUrl.pathname}${redirectUrl.search}${redirectUrl.hash}`;
}

/**
 * Every order H5 route is session-gated (all-routes-auth decision, see
 * docs/decisions.md): the auth surface renders its own routes for anonymous
 * visitors, authenticated visitors get the product surface, and anonymous
 * hits on any product route redirect into the login flow with a sanitized
 * `redirect` return target.
 */
export function resolveSdkworkOrderH5AuthGateDecision({
  hasSession,
  homePath = DEFAULT_HOME_PATH,
  location,
}: {
  hasSession: boolean;
  homePath?: string;
  location: Pick<Location, "pathname" | "search" | "hash">;
}): SdkworkOrderH5AuthGateDecision {
  const pathname = normalizePathname(location.pathname);
  if (isAuthRoute(pathname)) {
    if (!hasSession) {
      return { kind: "auth-route" };
    }

    const redirect = new URLSearchParams((location.search ?? "").replace(/^\?/u, "")).get("redirect");
    return {
      kind: "redirect",
      replace: true,
      to: sanitizeSdkworkOrderH5AuthRedirect(redirect) || normalizePathname(homePath),
    };
  }

  if (!hasSession) {
    return {
      kind: "redirect",
      replace: true,
      to: buildSdkworkOrderH5AuthLoginRedirect(location),
    };
  }

  return { kind: "product-route" };
}

function isAuthRoute(pathname: string): boolean {
  return pathname === AUTH_BASE_PATH || pathname.startsWith(`${AUTH_BASE_PATH}/`);
}

function normalizePathname(pathname: string): string {
  const normalized = pathname.trim();
  if (!normalized) {
    return DEFAULT_HOME_PATH;
  }
  return normalized.startsWith("/") ? normalized : `/${normalized}`;
}
