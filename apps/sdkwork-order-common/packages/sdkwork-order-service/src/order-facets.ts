/**
 * Order-centre tag and scene facets, shared by every client surface.
 *
 * The PC admin, PC buyer, H5, Flutter, and mini-program order centres all offer
 * the same two filters, so the canonical scene list, the display order, and the
 * tag canonicalization rule live here once. The server enforces the same rules
 * (`sdkwork-order` `OrderScene` / `OrderTag`); this module mirrors them so a
 * client can normalize before it submits and never send something the server
 * would reject.
 */

/**
 * Closed set of commerce scenes an order can be placed in.
 *
 * Values and members match the backend `OrderScene` enum exactly. An unknown
 * scene is rejected by the API rather than quietly stored, so this list is the
 * complete set of options any filter dropdown may offer.
 */
export const SDKWORK_ORDER_SCENES = [
  "web",
  "h5",
  "app",
  "mini_program",
  "cashier",
  "membership",
  "recharge",
  "platform",
  "default",
] as const;

export type SdkworkOrderScene = (typeof SDKWORK_ORDER_SCENES)[number];

/** Maximum tags one order may carry (mirrors `ORDER_TAG_MAX_COUNT`). */
export const SDKWORK_ORDER_TAG_MAX_COUNT = 20;

/** Maximum length of one canonical tag (mirrors `ORDER_TAG_MAX_LENGTH`). */
export const SDKWORK_ORDER_TAG_MAX_LENGTH = 32;

/** Maximum tag values accepted in one filter (mirrors the server bound). */
export const SDKWORK_ORDER_TAG_FILTER_MAX_COUNT = 20;

/**
 * How multiple selected tags combine.
 *
 * `any` is the default because a multi-select facet conventionally widens the
 * result set; `all` is opt-in for cohort narrowing.
 */
export type SdkworkOrderTagMatchMode = "any" | "all";

const SCENE_ALIASES: Record<string, SdkworkOrderScene> = {
  android: "app",
  app: "app",
  cashier: "cashier",
  cashier_link: "cashier",
  default: "default",
  desktop: "web",
  flutter: "app",
  h5: "h5",
  ios: "app",
  membership: "membership",
  mini_program: "mini_program",
  miniapp: "mini_program",
  miniprogram: "mini_program",
  mobile_web: "h5",
  native: "app",
  pc: "web",
  platform: "platform",
  recharge: "recharge",
  subscription: "membership",
  top_up: "recharge",
  topup: "recharge",
  wap: "h5",
  web: "web",
  wechat: "mini_program",
  wechat_mini_program: "mini_program",
};

/** Returns `true` when `value` is a canonical scene. */
export function isSdkworkOrderScene(value: string): value is SdkworkOrderScene {
  return (SDKWORK_ORDER_SCENES as readonly string[]).includes(value);
}

/**
 * Normalizes a scene value (canonical name or documented alias) into the
 * canonical scene, or `undefined` when it is not recognized.
 */
export function normalizeSdkworkOrderScene(value: string | undefined | null): SdkworkOrderScene | undefined {
  const normalized = String(value ?? "").trim().toLowerCase().replace(/[-\s]+/gu, "_");
  return SCENE_ALIASES[normalized];
}

/**
 * Canonicalizes one tag.
 *
 * Mirrors the server rule: lowercase, alphanumeric plus `-`/`_`/CJK, whitespace
 * and separators folded into a single `-`, leading/trailing separators removed.
 * Returns `undefined` for input that normalizes to nothing (so a token editor
 * can silently drop a stray comma) and for input too long to be a legal tag.
 */
export function normalizeSdkworkOrderTag(value: string | undefined | null): string | undefined {
  const raw = String(value ?? "").trim();
  let canonical = "";
  let pendingSeparator = false;
  for (const character of raw) {
    if (/[\p{L}\p{N}]/u.test(character)) {
      // A pending separator only materializes between two word characters, so
      // a trailing or leading separator is naturally dropped.
      if (pendingSeparator && canonical.length > 0) {
        canonical += "-";
      }
      canonical += character.toLowerCase();
      pendingSeparator = false;
      continue;
    }
    if (character === "_" || character === "-" || character === "." || /\s/u.test(character)) {
      pendingSeparator = true;
      continue;
    }
    // Characters the server rejects outright (punctuation, quotes, symbols).
    return undefined;
  }
  if (!canonical) return undefined;
  if ([...canonical].length > SDKWORK_ORDER_TAG_MAX_LENGTH) return undefined;
  return canonical;
}

/**
 * Canonicalizes and deduplicates a tag list, preserving input order.
 *
 * Accepts the shapes a filter input or a token editor produces: a single
 * comma/pipe-separated string, or an array of them (which is how a repeated
 * `tags=` query parameter and a pasted list both arrive).
 */
export function normalizeSdkworkOrderTags(
  value: string | readonly string[] | undefined | null,
): string[] {
  const parts = value === undefined || value === null
    ? []
    : typeof value === "string"
      ? value.split(/[,|]/u)
      : value.flatMap((item) => String(item ?? "").split(/[,|]/u));
  const tags: string[] = [];
  for (const part of parts) {
    const tag = normalizeSdkworkOrderTag(part);
    if (tag && !tags.includes(tag)) {
      tags.push(tag);
    }
    if (tags.length > SDKWORK_ORDER_TAG_FILTER_MAX_COUNT) {
      break;
    }
  }
  return tags.slice(0, SDKWORK_ORDER_TAG_FILTER_MAX_COUNT);
}

/** Facet values accepted by an order list request. */
export interface SdkworkOrderListFacets {
  /** Selected scene; `undefined` means "all scenes". */
  scene?: SdkworkOrderScene;
  /** Selected tags (canonical). Empty means "all tags". */
  tags?: readonly string[];
  /** How `tags` combine. Defaults to `any`. */
  tagMatch?: SdkworkOrderTagMatchMode;
}

/**
 * Converts in-memory facets into request parameters.
 *
 * Returns only the keys that are actually set so the caller can spread the
 * result into a generated SDK params object without overriding its defaults
 * with `undefined`.
 */
export function toSdkworkOrderFacetParams(
  facets: SdkworkOrderListFacets | undefined,
): { scene?: SdkworkOrderScene; tags?: string[]; tagMatch?: SdkworkOrderTagMatchMode } {
  if (!facets) return {};
  const tags = normalizeSdkworkOrderTags(facets.tags);
  const scene = facets.scene ? normalizeSdkworkOrderScene(facets.scene) : undefined;
  return {
    ...(scene ? { scene } : {}),
    ...(tags.length > 0 ? { tags } : {}),
    ...(tags.length > 0 ? { tagMatch: facets.tagMatch ?? "any" } : {}),
  };
}

/** Number of active facet filters, for a "filters applied" badge. */
export function countSdkworkOrderActiveFacets(facets: SdkworkOrderListFacets | undefined): number {
  if (!facets) return 0;
  const tags = normalizeSdkworkOrderTags(facets.tags);
  return Number(Boolean(facets.scene && facets.scene !== "default")) + Number(tags.length > 0);
}
