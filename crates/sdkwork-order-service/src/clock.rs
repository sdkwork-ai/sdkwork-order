//! Canonical order-domain clock.
//!
//! Single authority for the order-domain timestamp wire/store format:
//! canonical RFC 3339 UTC with fixed millisecond precision (for example
//! `2026-10-01T12:34:56.789Z`), implemented on `sdkwork-utils` datetime.
//! DATABASE_SPEC §8.1.1 requires TEXT-stored instants to use exactly one
//! normalized UTC format whose lexical ordering matches chronological
//! ordering — every order-domain producer of stored or submitted timestamps
//! (routes, repository, hosts) must go through this module.

use sdkwork_utils_rust::datetime;

/// Current instant in the canonical order-domain format.
pub fn canonical_now_timestamp() -> String {
    datetime::format_datetime(datetime::now(), None)
}

/// Canonical order-domain format for a given instant — the decode-side
/// counterpart of [`canonical_now_timestamp`]: native TIMESTAMPTZ reads
/// re-encode through this so the wire/store value stays the exact canonical
/// RFC 3339 UTC string it was before the column went native.
pub fn canonical_now_timestamp_from(instant: chrono::DateTime<chrono::Utc>) -> String {
    datetime::format_datetime(instant, None)
}

/// Canonical order-domain format for an instant `seconds` in the future
/// (a negative value reaches into the past) — used to derive payment-window
/// deadlines such as `expired_at` / checkout `expires_at`.
pub fn canonical_timestamp_after_seconds(seconds: i64) -> String {
    let instant = datetime::now() + chrono::Duration::seconds(seconds);
    datetime::format_datetime(instant, None)
}

#[cfg(test)]
mod tests {
    use super::{canonical_now_timestamp, canonical_timestamp_after_seconds};

    #[test]
    fn canonical_format_is_fixed_precision_rfc3339_utc() {
        let value = canonical_now_timestamp();
        assert_eq!(value.len(), 24);
        assert!(value.ends_with('Z'));
        assert!(chrono::DateTime::parse_from_rfc3339(&value).is_ok());
    }

    #[test]
    fn deadlines_sort_after_now_lexicographically() {
        let now = canonical_now_timestamp();
        let later = canonical_timestamp_after_seconds(1_800);
        assert!(later > now);
    }
}
