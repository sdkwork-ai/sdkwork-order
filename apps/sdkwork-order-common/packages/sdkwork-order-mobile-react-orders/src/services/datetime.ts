/**
 * Locale-aware date-time formatting for order surfaces.
 *
 * Wire timestamps are ISO-8601 strings. Rendering them raw leaks machine text
 * into the UI (`2026-11-15T12:00:00.000Z`); every order screen should show a
 * short local date-time instead. Unparseable values pass through unchanged so a
 * malformed server value stays visible instead of becoming an empty cell.
 */
export function formatOrderDateTime(value: string | undefined | null, locale = "zh-CN"): string | undefined {
  const trimmed = value?.trim();
  if (!trimmed) {
    return undefined;
  }
  const date = new Date(trimmed);
  if (Number.isNaN(date.getTime())) {
    return trimmed;
  }
  return new Intl.DateTimeFormat(locale, {
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    month: "numeric",
  }).format(date);
}
