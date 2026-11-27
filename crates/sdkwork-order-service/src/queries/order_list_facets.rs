//! Query-string parsing for the order-centre tag and scene facets.
//!
//! The same facet shapes appear on the buyer order list (app API), the admin
//! order list, and the admin facet endpoint. Parsing them in one place keeps
//! the accepted wire forms identical across surfaces, which is what lets the
//! PC, H5, Flutter, and mini-program clients share a vocabulary.
//!
//! Accepted forms, all equivalent:
//!
//! - `tags=vip,risk`
//! - `tags=vip|risk`
//! - `tags=vip&tags=risk`
//! - any combination of the above
//!
//! Scene accepts the canonical value plus the documented aliases
//! (`pc`, `wap`, `miniprogram`, …); `all` and `*` mean "no filter".

use sdkwork_contract_service::CommerceServiceError;

use crate::{parse_optional_scene, OrderScene, OrderTagMatchMode, OrderTagSet};

/// Facets parsed from one order list request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderListFacets {
    pub scene: Option<OrderScene>,
    pub tags: OrderTagSet,
    pub tag_match: OrderTagMatchMode,
}

impl OrderListFacets {
    /// Parses the facet portion of a query string.
    ///
    /// `csv` carries the (possibly comma-separated) `tags` parameter value and
    /// `repeated` the extra values produced by repeated `tags=` keys.
    pub fn parse(
        scene: Option<&str>,
        csv: Option<&str>,
        repeated: &[String],
        tag_match: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        Ok(Self {
            scene: parse_optional_scene(scene)?,
            tags: OrderTagSet::parse_csv_and_repeated(csv, repeated, "tags")?,
            tag_match: OrderTagMatchMode::parse(tag_match.unwrap_or_default())?,
        })
    }

    /// Facets with every filter disabled.
    #[must_use]
    pub fn none() -> Self {
        Self {
            scene: None,
            tags: OrderTagSet::default(),
            tag_match: OrderTagMatchMode::default(),
        }
    }
}

/// Extracts every value of repeated `key` parameters from a raw query string.
///
/// The first occurrence is returned by the `serde`-decoded single-value field,
/// so this returns only the *additional* occurrences; callers therefore pass
/// the decoded `Option<String>` as the CSV value and this list as the extras.
///
/// Only percent-decoding is applied (`+` becomes a space); the tag
/// canonicalizer then rejects anything that is not a legal tag character.
#[must_use]
pub fn collect_repeated_query_values(raw_query: Option<&str>, key: &str) -> Vec<String> {
    let Some(raw_query) = raw_query else {
        return Vec::new();
    };
    let mut collected = Vec::new();
    let mut seen = 0_usize;
    for pair in raw_query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        if percent_decode(name) != key {
            continue;
        }
        seen += 1;
        // The first occurrence is already represented by the serde-decoded
        // scalar field; only hand back the rest.
        if seen > 1 {
            collected.push(percent_decode(value));
        }
    }
    collected
}

/// Decodes `%XX` escapes and `+` as a space, leaving malformed escapes intact.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                match (hex_value(bytes[index + 1]), hex_value(bytes[index + 2])) {
                    (Some(high), Some(low)) => {
                        out.push(high << 4 | low);
                        index += 3;
                    }
                    _ => {
                        out.push(b'%');
                        index += 1;
                    }
                }
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_values_skip_the_first_occurrence() {
        let repeated = collect_repeated_query_values(Some("tags=vip&tags=risk&page=1"), "tags");
        assert_eq!(repeated, vec!["risk".to_owned()]);
    }

    #[test]
    fn repeated_values_percent_decode() {
        let repeated =
            collect_repeated_query_values(Some("tags=a&tags=summer%20sale&tags=%E9%AB%98"), "tags");
        assert_eq!(repeated, vec!["summer sale".to_owned(), "高".to_owned()]);
    }

    #[test]
    fn missing_or_other_keys_yield_nothing() {
        assert!(collect_repeated_query_values(None, "tags").is_empty());
        assert!(collect_repeated_query_values(Some("page=2"), "tags").is_empty());
        assert!(collect_repeated_query_values(Some("tags=vip"), "other").is_empty());
    }

    #[test]
    fn facets_combine_csv_and_repeated_forms() {
        let facets = OrderListFacets::parse(
            Some("pc"),
            Some("vip, risk"),
            &["summer sale".to_owned()],
            Some("all"),
        )
        .unwrap();
        assert_eq!(facets.scene, Some(OrderScene::Web));
        assert_eq!(
            facets.tags.to_values(),
            vec!["vip", "risk", "summer-sale"]
        );
        assert_eq!(facets.tag_match, OrderTagMatchMode::All);
    }

    #[test]
    fn facets_default_to_disabled() {
        let facets = OrderListFacets::parse(None, None, &[], None).unwrap();
        assert_eq!(facets, OrderListFacets::none());
    }

    #[test]
    fn scene_all_disables_the_filter() {
        let facets = OrderListFacets::parse(Some("all"), None, &[], None).unwrap();
        assert!(facets.scene.is_none());
    }

    #[test]
    fn invalid_input_is_rejected() {
        assert!(OrderListFacets::parse(Some("telepathy"), None, &[], None).is_err());
        assert!(OrderListFacets::parse(None, Some("bad!tag"), &[], None).is_err());
        assert!(OrderListFacets::parse(None, None, &[], Some("xor")).is_err());
    }

    #[test]
    fn too_many_tags_are_rejected() {
        let many: Vec<String> = (0..25).map(|index| format!("tag{index}")).collect();
        assert!(OrderListFacets::parse(None, None, &many, None).is_err());
    }
}
