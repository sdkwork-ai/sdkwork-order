import { describe, expect, it } from "vitest";

import {
  buildSdkworkOrderH5AuthLoginRedirect,
  hasSdkworkOrderH5AuthenticatedSession,
  resolveSdkworkOrderH5AuthGateDecision,
  sanitizeSdkworkOrderH5AuthRedirect,
} from "../src/auth/authGateLogic";
import type { SdkworkOrderH5SessionSnapshot } from "../src/auth/sessionStore";

function locationOf(pathname: string, search = ""): { pathname: string; search: string; hash: string } {
  return { hash: "", pathname, search };
}

function snapshotWith(options: {
  accessToken?: string;
  authToken?: string;
  tenantId?: string;
}): SdkworkOrderH5SessionSnapshot {
  return {
    accessToken: options.accessToken,
    authToken: options.authToken,
    context: options.tenantId === undefined ? undefined : { tenantId: options.tenantId },
  };
}

describe("hasSdkworkOrderH5AuthenticatedSession", () => {
  it("requires authToken, accessToken and tenant context together", () => {
    expect(hasSdkworkOrderH5AuthenticatedSession({})).toBe(false);
    expect(hasSdkworkOrderH5AuthenticatedSession(snapshotWith({ accessToken: "a", authToken: "b" }))).toBe(false);
    expect(hasSdkworkOrderH5AuthenticatedSession(snapshotWith({ accessToken: "a", tenantId: "t" }))).toBe(false);
    expect(
      hasSdkworkOrderH5AuthenticatedSession(snapshotWith({ accessToken: "a", authToken: "b", tenantId: "t" })),
    ).toBe(true);
  });
});

describe("buildSdkworkOrderH5AuthLoginRedirect", () => {
  it("preserves pathname, search and hash in the redirect target", () => {
    expect(buildSdkworkOrderH5AuthLoginRedirect(locationOf("/orders/42", "?tab=paid#/x"))).toBe(
      "/auth/login?redirect=%2Forders%2F42%3Ftab%3Dpaid%23%2Fx",
    );
  });
});

describe("sanitizeSdkworkOrderH5AuthRedirect", () => {
  it("keeps same-origin absolute paths", () => {
    expect(sanitizeSdkworkOrderH5AuthRedirect("/orders/voucher")).toBe("/orders/voucher");
  });

  it("falls back to the home path for protocol-relative, foreign, non-path and auth targets", () => {
    const home = "/orders";
    expect(sanitizeSdkworkOrderH5AuthRedirect("//evil.example/a")).toBe(home);
    expect(sanitizeSdkworkOrderH5AuthRedirect("https://evil.example/a")).toBe(home);
    expect(sanitizeSdkworkOrderH5AuthRedirect("orders/voucher")).toBe(home);
    expect(sanitizeSdkworkOrderH5AuthRedirect("/auth/login")).toBe(home);
    expect(sanitizeSdkworkOrderH5AuthRedirect(null)).toBe(home);
    expect(sanitizeSdkworkOrderH5AuthRedirect("%zz")).toBe(home);
  });
});

describe("resolveSdkworkOrderH5AuthGateDecision", () => {
  const homePath = "/orders";

  it("renders the auth surface for anonymous visitors on auth routes", () => {
    expect(
      resolveSdkworkOrderH5AuthGateDecision({
        hasSession: false,
        homePath,
        location: locationOf("/auth/login"),
      }),
    ).toEqual({ kind: "auth-route" });
  });

  it("redirects authenticated visitors away from auth routes honoring a safe redirect", () => {
    expect(
      resolveSdkworkOrderH5AuthGateDecision({
        hasSession: true,
        homePath,
        location: { hash: "", pathname: "/auth/login", search: "?redirect=%2Forders%2Fvoucher" },
      }),
    ).toEqual({ kind: "redirect", replace: true, to: "/orders/voucher" });
  });

  it("redirects authenticated visitors on auth routes to home without a redirect param", () => {
    expect(
      resolveSdkworkOrderH5AuthGateDecision({
        hasSession: true,
        homePath,
        location: locationOf("/auth/login"),
      }),
    ).toEqual({ kind: "redirect", replace: true, to: homePath });
  });

  it("redirects anonymous visitors from any product route into the login flow", () => {
    const decision = resolveSdkworkOrderH5AuthGateDecision({
      hasSession: false,
      homePath,
      location: locationOf("/value/token-bank"),
    });
    expect(decision).toEqual({
      kind: "redirect",
      replace: true,
      to: "/auth/login?redirect=%2Fvalue%2Ftoken-bank",
    });
  });

  it("keeps authenticated visitors on product routes", () => {
    expect(
      resolveSdkworkOrderH5AuthGateDecision({
        hasSession: true,
        homePath,
        location: locationOf("/orders"),
      }),
    ).toEqual({ kind: "product-route" });
  });
});
