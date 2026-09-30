//! Canonical store clock for every order-domain timestamp column.
//!
//! Every order-domain write goes through this module so the format has
//! exactly one definition: canonical RFC 3339 UTC with fixed millisecond
//! precision (for example `2026-10-01T12:34:56.789Z`), implemented on
//! `sdkwork-utils` datetime. All order-owned instant columns are native
//! TIMESTAMPTZ (migrations 0015/0017; DATABASE_SPEC §8.1): writes bind
//! canonical text through `CAST(... AS TIMESTAMPTZ)` and reads project back
//! through `TO_CHAR(col AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"')`,
//! so consumers always read the identical wire value. The
//! `commerce_inventory_reservation` table is excluded — it is owned by the
//! sdkwork-inventory module baseline and the order queries that touch it use
//! explicit casts.

/// Current instant in the canonical store format.
pub(crate) fn now_canonical() -> String {
    sdkwork_order_service::canonical_now_timestamp()
}

/// Canonical store format for an instant `seconds` in the future (a negative
/// value reaches into the past) — used to derive payment-window deadlines
/// such as `expired_at`.
#[allow(dead_code)]
pub(crate) fn canonical_after_seconds(seconds: i64) -> String {
    sdkwork_order_service::canonical_timestamp_after_seconds(seconds)
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
