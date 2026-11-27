import { Button } from "@sdkwork/ui-pc-react";
import {
  SDKWORK_ORDER_SCENES,
  type SdkworkOrderScene,
} from "@sdkwork/order-service";
import type { SdkworkOrderFacetSelection } from "../order-controller";

/**
 * Buyer-side scene and tag facets for the order centre.
 *
 * Mirrors the admin surface so a buyer and an operator filtering "H5 orders
 * tagged vip" see the same rows. The scene row is a chip group (few, closed
 * values); tags are free text with active-tag chips, because a buyer's tag
 * vocabulary is tenant-specific and cannot be enumerated client-side.
 */
export interface SdkworkOrderFacetFilterProps {
  /** Scene labels keyed by canonical scene; the component falls back to the raw value. */
  sceneLabels?: Partial<Record<SdkworkOrderScene, string>>;
  facets: SdkworkOrderFacetSelection;
  labels: {
    anyScene: string;
    /** Localized label for a canonical tag value. */
    tagLabel?: (tag: string) => string;
    tagsEmpty: string;
    tagsPlaceholder: string;
    tagMatchAll: string;
    tagMatchAny: string;
    tagsTitle: string;
  };
  onChange(facets: Partial<SdkworkOrderFacetSelection>): void;
  /** Accepted tag input; normalized by the caller with the shared rule. */
  tagDraft: string;
  onTagDraftChange(value: string): void;
  onCommitTag(): void;
  onRemoveTag(tag: string): void;
  disabled?: boolean;
}

export function SdkworkOrderFacetFilter({
  disabled = false,
  facets,
  labels,
  onChange,
  onCommitTag,
  onRemoveTag,
  onTagDraftChange,
  sceneLabels,
  tagDraft,
}: SdkworkOrderFacetFilterProps) {
  return (
    <div className="space-y-3">
      <div
        aria-label={labels.anyScene}
        className="flex flex-wrap items-center gap-2"
        role="group"
      >
        <Button
          className="rounded-full px-3"
          disabled={disabled}
          onClick={() => onChange({ scene: undefined })}
          size="sm"
          type="button"
          variant={facets.scene ? "ghost" : "secondary"}
        >
          {labels.anyScene}
        </Button>
        {SDKWORK_ORDER_SCENES.map((scene) => (
          <Button
            aria-pressed={facets.scene === scene}
            className="rounded-full px-3"
            disabled={disabled}
            key={scene}
            onClick={() => onChange({ scene })}
            size="sm"
            type="button"
            variant={facets.scene === scene ? "secondary" : "ghost"}
          >
            {sceneLabels?.[scene] ?? scene}
          </Button>
        ))}
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <span className="text-xs font-medium text-[var(--sdk-color-text-secondary)]">
          {labels.tagsTitle}
        </span>
        {facets.tags.length === 0 ? (
          <span className="text-xs text-[var(--sdk-color-text-muted)]">{labels.tagsEmpty}</span>
        ) : (
          facets.tags.map((tag) => (
            <span
              className="inline-flex items-center gap-1 rounded-full border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-muted)] py-0.5 pl-2.5 pr-1 text-xs font-medium text-[var(--sdk-color-text-secondary)]"
              key={tag}
            >
              {labels.tagLabel?.(tag) ?? tag}
              <button
                aria-label={`${labels.tagsTitle}: ${tag}`}
                className="inline-flex h-4 w-4 items-center justify-center rounded-full text-[var(--sdk-color-text-muted)] transition-colors hover:bg-[var(--sdk-color-surface-panel)]"
                disabled={disabled}
                type="button"
                onClick={() => onRemoveTag(tag)}
              >
                ×
              </button>
            </span>
          ))
        )}
        <input
          aria-label={labels.tagsPlaceholder}
          className="h-8 w-48 rounded-[var(--sdk-radius-field)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-2.5 text-sm text-[var(--sdk-color-text-primary)] outline-none transition-colors focus-visible:border-[var(--sdk-color-brand-primary)]"
          disabled={disabled}
          placeholder={labels.tagsPlaceholder}
          value={tagDraft}
          onChange={(event) => onTagDraftChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === ",") {
              event.preventDefault();
              onCommitTag();
            }
          }}
        />
        {facets.tags.length > 1 ? (
          <Button
            className="rounded-full px-3"
            disabled={disabled}
            onClick={() => onChange({ tagMatch: facets.tagMatch === "all" ? "any" : "all" })}
            size="sm"
            type="button"
            variant="ghost"
          >
            {facets.tagMatch === "all" ? labels.tagMatchAll : labels.tagMatchAny}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
