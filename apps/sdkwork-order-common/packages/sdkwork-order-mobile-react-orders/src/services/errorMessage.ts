/**
 * Localized API error surfacing for the order mobile UI.
 *
 * SDK failures carry an `error.problem` payload (problem+json) whose
 * `i18nKey` is the server-sanctioned localization hook (e.g.
 * `errors.result.40901`). Resolution order:
 *   1. `problem.i18nKey` through `t()` when the host ships a resource for it;
 *   2. the SDK error `code` family (network/timeout/auth/...) keys;
 *   3. the server `detail` / `error.message` as a raw fallback.
 * Unknown codes degrade gracefully instead of rendering the raw key.
 */

/**
 * Minimal `t` shape consumed by the helper. The react-i18next `TFunction`
 * is structurally compatible with this signature.
 */
export interface TranslateFunction {
  (key: string): string;
}

/** problem+json payload attached by the SDK as `error.problem`. */
export interface ApiProblemPayload {
  readonly i18nKey?: string;
  readonly detail?: string;
  readonly title?: string;
  readonly code?: string | number;
}

function toApiProblem(error: unknown): ApiProblemPayload | undefined {
  if (error && typeof error === "object") {
    const problem = (error as { problem?: unknown }).problem;
    if (problem && typeof problem === "object") {
      return problem as ApiProblemPayload;
    }
  }
  return undefined;
}

/** SDK error code families mapped onto `errors.*` resource keys. */
const SDK_ERROR_MESSAGE_KEYS: Readonly<Record<string, string>> = {
  NETWORK_ERROR: "errors.network",
  TIMEOUT: "errors.timeout",
  CANCELLED: "errors.cancelled",
  UNAUTHORIZED: "errors.unauthorized",
  TOKEN_EXPIRED: "errors.unauthorized",
  TOKEN_INVALID: "errors.unauthorized",
  FORBIDDEN: "errors.forbidden",
  NOT_FOUND: "errors.not_found",
  RATE_LIMIT: "errors.rate_limit",
  SERVER_ERROR: "errors.server",
  BAD_GATEWAY: "errors.server",
  SERVICE_UNAVAILABLE: "errors.server",
  GATEWAY_TIMEOUT: "errors.timeout",
};

function translateIfPresent(t: TranslateFunction, key: string): string | null {
  const translated = t(key);
  if (translated && translated !== key) {
    return translated;
  }
  return null;
}

/** Converts an API error into a localized, user-facing message. */
export function toUserErrorMessage(t: TranslateFunction, error: unknown): string {
  const problem = toApiProblem(error);
  if (problem?.i18nKey) {
    const localized = translateIfPresent(t, problem.i18nKey);
    if (localized) {
      return localized;
    }
  }
  const errorCode =
    error !== null && typeof error === "object"
      ? (error as { code?: unknown }).code
      : undefined;
  if (typeof errorCode === "string") {
    const fallbackKey = SDK_ERROR_MESSAGE_KEYS[errorCode];
    if (fallbackKey) {
      const localized = translateIfPresent(t, fallbackKey);
      if (localized) {
        return localized;
      }
    }
  }
  if (problem?.detail) {
    return problem.detail;
  }
  if (error instanceof Error && error.message) {
    return error.message;
  }
  return String(error);
}
