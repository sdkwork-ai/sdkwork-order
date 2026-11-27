//! Shared SQL fragments for the order-centre tag and scene facets.
//!
//! The filters live in one place because three read paths (buyer order list,
//! admin order list, admin facet aggregates) must compose the *same* predicate
//! text with the *same* parameter numbering rules. Duplicating them is how a
//! facet drifts between the list and the filter bar that describes it.
//!
//! ## Parameter safety
//!
//! Every user value is bound, never interpolated. The only text built from
//! input is the number of `$N` placeholders, which is derived from a validated
//! [`OrderTagSet`] whose length is bounded by `ORDER_TAG_FILTER_MAX_COUNT`.
//!
//! Callers wrap the rendered statement in `sqlx::AssertSqlSafe`; that assertion
//! is sound for this module because no interpolation path exists — the
//! fragments are assembled from string literals (column names are hardcoded,
//! never derived from input) plus integer placeholder indices.

use sdkwork_order_service::{OrderScene, OrderTagMatchMode, OrderTagSet};

/// Renders the tag predicate for a list query.
///
/// Returns `None` when no tag filter was requested, so the caller can emit the
/// column expression unchanged.
///
/// `parameter_base` is the 1-based number the first tag placeholder takes
/// (`$N`), so the fragment can be appended to a statement that already uses
/// lower numbers. Bind the values with [`json_tag_literals`] in the same order.
///
/// Both modes use `@>`, which the `jsonb_path_ops` GIN index on
/// `commerce_order.tags` serves directly; the two modes differ only in how many
/// probes they emit.
#[must_use]
pub(crate) fn order_tags_predicate(
    tags: &OrderTagSet,
    mode: OrderTagMatchMode,
    parameter_base: usize,
) -> Option<String> {
    if tags.is_empty() {
        return None;
    }
    let placeholders: Vec<String> = (0..tags.len())
        .map(|offset| format!("${}", parameter_base + offset))
        .collect();
    Some(match mode {
        // AND semantics: one containment test against the whole requested set,
        // which is a single GIN probe.
        OrderTagMatchMode::All => format!(
            " AND o.tags @> jsonb_build_array({})",
            placeholders.join(", ")
        ),
        // OR semantics: one containment probe per tag, ORed together so the
        // planner can bitmap-combine them on the GIN index.
        OrderTagMatchMode::Any => format!(
            " AND ({})",
            placeholders
                .iter()
                .map(|placeholder| format!("o.tags @> jsonb_build_array({placeholder})"))
                .collect::<Vec<_>>()
                .join(" OR ")
        ),
    })
}

/// Renders the scene predicate for a list query (`None` when unfiltered).
#[must_use]
pub(crate) fn order_scene_predicate(
    scene: Option<OrderScene>,
    parameter_index: usize,
) -> Option<String> {
    scene.map(|_| format!(" AND o.scene_code = CAST(${parameter_index} AS TEXT)"))
}

/// Binds the tag values for [`order_tags_predicate`] in placeholder order.
///
/// Each value is wrapped as a JSON string literal (`"vip"`) so a single bound
/// text can be cast to `jsonb` and compared against the stored array.
#[must_use]
pub(crate) fn json_tag_literals(tags: &OrderTagSet) -> Vec<String> {
    tags.as_slice()
        .iter()
        .map(|tag| {
            serde_json::to_string(tag.as_str())
                .unwrap_or_else(|_| String::from("\"\""))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag_set(values: &[&str]) -> OrderTagSet {
        OrderTagSet::parse_many(values.iter().copied(), "tags", 20).unwrap()
    }

    #[test]
    fn empty_tag_set_emits_no_predicate() {
        assert!(order_tags_predicate(&tag_set(&[]), OrderTagMatchMode::Any, 9).is_none());
        assert!(order_scene_predicate(None, 9).is_none());
    }

    #[test]
    fn any_mode_ors_each_tag_with_numbered_placeholders() {
        let predicate =
            order_tags_predicate(&tag_set(&["vip", "risk"]), OrderTagMatchMode::Any, 9).unwrap();
        assert!(predicate.starts_with(" AND ("));
        assert!(predicate.contains("$9"));
        assert!(predicate.contains("$10"));
        assert_eq!(predicate.matches(" OR ").count(), 1);
    }

    #[test]
    fn all_mode_single_containment_probe() {
        let predicate =
            order_tags_predicate(&tag_set(&["vip", "risk"]), OrderTagMatchMode::All, 9).unwrap();
        assert!(predicate.contains("@> jsonb_build_array("));
        assert!(!predicate.contains(" OR "));
        assert!(predicate.contains("$9"));
        assert!(predicate.contains("$10"));
    }

    #[test]
    fn scene_predicate_uses_the_documented_column() {
        assert_eq!(
            order_scene_predicate(Some(OrderScene::H5), 8).unwrap(),
            " AND o.scene_code = CAST($8 AS TEXT)"
        );
    }

    #[test]
    fn tag_literals_are_json_quoted_so_injection_cannot_escape_a_string() {
        let literals = json_tag_literals(&tag_set(&["vip"]));
        assert_eq!(literals, vec!["\"vip\"".to_owned()]);
        // Canonicalization rejects quotes before they ever reach SQL.
        assert!(OrderTagSet::parse_many(["vip\" OR 1=1"], "tags", 20).is_err());
    }
}
