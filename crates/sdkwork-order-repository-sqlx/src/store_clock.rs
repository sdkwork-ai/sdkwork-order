//! Canonical store clock for every order-domain timestamp column.
//!
//! DATABASE_SPEC §8.1.1: TEXT-stored logical instants are only compliant when
//! they use a single normalized UTC format whose lexical ordering matches
//! chronological ordering. Every order-domain write goes through this module
//! so the format has exactly one definition: canonical RFC 3339 UTC with
//! fixed millisecond precision (for example `2026-10-01T12:34:56.789Z`),
//! implemented on `sdkwork-utils` datetime. Predicates against these columns
//! therefore compare lexicographically consistently and cast explicitly
//! (`col::timestamptz`) when comparing against real instants.

use sdkwork_utils_rust::datetime;

/// Current instant in the canonical store format.
pub(crate) fn now_canonical() -> String {
    datetime::format_datetime(datetime::now(), None)
}

/// Canonical store format for an instant `seconds` in the future (a negative
/// value reaches into the past) — used to derive payment-window deadlines
/// such as `expired_at`.
pub(crate) fn canonical_after_seconds(seconds: i64) -> String {
    let instant = datetime::now() + chrono::Duration::seconds(seconds);
    datetime::format_datetime(instant, None)
}

#[cfg(test)]
mod tests {
    use super::{canonical_after_seconds, now_canonical};

    #[test]
    fn canonical_timestamps_are_lexicographically_ordered() {
        let now = now_canonical();
        let later = canonical_after_seconds(60);
        assert!(later > now, "{later} must sort after {now}");
    }

    #[test]
    fn canonical_format_has_fixed_precision_and_utc_suffix() {
        let value = now_canonical();
        assert!(value.ends_with('Z'), "{value} must end with the UTC suffix");
        assert_eq!(value.len(), 24, "{value} must be YYYY-MM-DDTHH:MM:SS.mmmZ");
        assert!(chrono::DateTime::parse_from_rfc3339(&value).is_ok());
    }
}
