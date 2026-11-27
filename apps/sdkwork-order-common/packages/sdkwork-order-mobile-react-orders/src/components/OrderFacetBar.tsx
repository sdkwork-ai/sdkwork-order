import { useMemo, useState } from "react";
import { Filter, X } from "lucide-react";

import type { OrderListFacets } from "../services/OrderService";

/**
 * Canonical commerce scenes, mirroring the backend `OrderScene` enum and the
 * `SDKWORK_ORDER_SCENES` constant every other order-centre surface renders.
 * Declared locally so the mobile package keeps its own dependency surface.
 */
const ORDER_SCENES = [
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

export interface OrderFacetBarProps {
  facets: OrderListFacets;
  onChange(facets: OrderListFacets): void;
  /** Localized scene label; falls back to the canonical value. */
  sceneLabel?(scene: string): string;
  /** Localized labels; hosts pass i18next-bound values. */
  labels: {
    anyScene: string;
    tagFilter: string;
    tagMatchAll: string;
    tagMatchAny: string;
    tagsPlaceholder: string;
  };
}

/**
 * Collapsible scene/tag facet bar for the mobile order centre.
 *
 * Collapsed by default because the mobile viewport is precious and most buyers
 * never filter; the trigger only reveals the panel on demand. Results still
 * reload through the same server-side facet path as the PC surfaces, so the
 * counts and rows always agree across devices.
 */
export function OrderFacetBar({ facets, labels, onChange, sceneLabel }: OrderFacetBarProps) {
  const [open, setOpen] = useState(false);
  const [tagDraft, setTagDraft] = useState("");

  const activeCount = useMemo(
    () => Number(Boolean(facets.scene)) + Number((facets.tags?.length ?? 0) > 0),
    [facets.scene, facets.tags],
  );

  const commitTag = () => {
    const tag = tagDraft.trim().toLowerCase().replace(/[\s_.-]+/gu, "-").replace(/^-+|-+$/gu, "");
    setTagDraft("");
    if (!tag) return;
    const current = facets.tags ?? [];
    if (current.includes(tag)) return;
    onChange({ ...facets, tags: [...current, tag] });
  };

  return (
    <div className="border-b border-border-color bg-panel-bg">
      <button
        type="button"
        aria-expanded={open}
        className="flex w-full items-center justify-between px-3 py-2 text-[13px] text-text-sub active:opacity-70"
        onClick={() => setOpen((value) => !value)}
      >
        <span className="flex items-center gap-1.5">
          <Filter className="h-3.5 w-3.5" />
          {labels.tagFilter}
          {activeCount > 0 ? (
            <span className="rounded-full bg-active-bg px-1.5 text-[11px] text-text-main">{activeCount}</span>
          ) : null}
        </span>
        <span className="text-[11px]">{open ? "收起" : "展开"}</span>
      </button>

      {open ? (
        <div className="space-y-2 px-3 pb-3">
          <div className="flex flex-wrap gap-1.5">
            <button
              type="button"
              aria-pressed={!facets.scene}
              className={
                facets.scene
                  ? "rounded-full border border-border-color px-2.5 py-1 text-[12px] text-text-sub"
                  : "rounded-full border border-brand-primary bg-active-bg px-2.5 py-1 text-[12px] text-text-main"
              }
              onClick={() => onChange({ ...facets, scene: undefined })}
            >
              {labels.anyScene}
            </button>
            {ORDER_SCENES.map((scene) => (
              <button
                key={scene}
                type="button"
                aria-pressed={facets.scene === scene}
                className={
                  facets.scene === scene
                    ? "rounded-full border border-brand-primary bg-active-bg px-2.5 py-1 text-[12px] text-text-main"
                    : "rounded-full border border-border-color px-2.5 py-1 text-[12px] text-text-sub"
                }
                onClick={() => onChange({ ...facets, scene })}
              >
                {sceneLabel?.(scene) ?? scene}
              </button>
            ))}
          </div>

          {(facets.tags ?? []).length > 0 ? (
            <div className="flex flex-wrap items-center gap-1.5">
              {(facets.tags ?? []).map((tag) => (
                <span
                  key={tag}
                  className="inline-flex items-center gap-1 rounded-full border border-border-color px-2.5 py-1 text-[12px] text-text-sub"
                >
                  {tag}
                  <button
                    type="button"
                    aria-label={`${labels.tagFilter}: ${tag}`}
                    onClick={() => onChange({
                      ...facets,
                      tags: (facets.tags ?? []).filter((value) => value !== tag),
                    })}
                  >
                    <X className="h-3 w-3" />
                  </button>
                </span>
              ))}
              {(facets.tags ?? []).length > 1 ? (
                <button
                  type="button"
                  className="rounded-full border border-border-color px-2.5 py-1 text-[12px] text-text-sub"
                  onClick={() => onChange({
                    ...facets,
                    tagMatch: facets.tagMatch === "all" ? "any" : "all",
                  })}
                >
                  {facets.tagMatch === "all" ? labels.tagMatchAll : labels.tagMatchAny}
                </button>
              ) : null}
            </div>
          ) : null}

          <input
            aria-label={labels.tagsPlaceholder}
            className="h-9 w-full rounded-[var(--sdk-radius-field)] border border-border-color bg-panel-bg px-2.5 text-[13px] text-text-main outline-none"
            placeholder={labels.tagsPlaceholder}
            value={tagDraft}
            onChange={(event) => setTagDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === ",") {
                event.preventDefault();
                commitTag();
              }
            }}
          />
        </div>
      ) : null}
    </div>
  );
}
