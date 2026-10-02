import { readRuntimeEnv, resolveBaseUrlWithAlignProtocol } from "@sdkwork/sdk-common";

/** Single shared API base-url key resolved through `@sdkwork/sdk-common`. */
const SDKWORK_API_BASE_URL_ENV_KEY = "SDKWORK_API_BASE_URL";

/**
 * Resolve the shared SDK API base url through `@sdkwork/sdk-common`.
 *
 * `SDKWORK_API_BASE_URL` replaces the per-app
 * `VITE_SDKWORK_<APP>_*_API_BASE_URL` keys. The resolver reads the configured
 * candidates and picks the API host matching the current page environment +
 * brand, preferring the current protocol. The generated SDK clients append
 * their own `/app/v3/api` / `/backend/v3/api` prefix, so the returned value is
 * a bare origin (path preservation stays off).
 *
 * Returns `undefined` when the shared key is not configured so callers fall
 * back to their config-derived base url instead of deriving a host from the
 * current page location.
 */
export function resolveSharedSdkApiBaseUrl(): string | undefined {
  if (!readRuntimeEnv(SDKWORK_API_BASE_URL_ENV_KEY)) {
    return undefined;
  }

  return resolveBaseUrlWithAlignProtocol({ envKey: SDKWORK_API_BASE_URL_ENV_KEY }).url || undefined;
}
