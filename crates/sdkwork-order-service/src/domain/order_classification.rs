//! Order classification facets: tags and commerce scene.
//!
//! Every order centre surface (PC admin, PC buyer, H5, Flutter, mini-program)
//! filters on the same two facets, so the vocabulary, canonicalization, and
//! limits live here once and the API/repository/UI layers only transport
//! normalized values.
//!
//! - **scene** is the commerce scenario an order was placed in. It is closed
//!   and enumerable, which is what makes it safe to build a filter dropdown
//!   from: [`OrderScene::ALL`] is the single source every surface renders.
//! - **tags** are an open, operator-owned label set. They are canonicalized
//!   ([`OrderTag::normalize`]) so `VIP`, `vip`, and ` vip ` are the same label,
//!   which keeps the tag facet and the GIN containment index coherent.

use sdkwork_contract_service::CommerceServiceError;

/// Maximum number of tags a single order may carry.
///
/// Bounded so the JSONB payload stays small enough for the containment index
/// and so a UI token editor has a hard ceiling to render against.
pub const ORDER_TAG_MAX_COUNT: usize = 20;

/// Maximum length of one canonical tag, in characters.
pub const ORDER_TAG_MAX_LENGTH: usize = 32;

/// Maximum number of tag values accepted in one filter request.
///
/// A request may ask for "any of these tags" (OR) or "all of these tags"
/// (AND); both sides are bounded here so a hand-crafted query string cannot
/// turn into an unbounded `IN` list.
pub const ORDER_TAG_FILTER_MAX_COUNT: usize = 20;

/// Maximum length of the operator remark stored beside the classification.
pub const ORDER_REMARK_MAX_LENGTH: usize = 500;

/// Commerce scenario an order was placed in.
///
/// The set is closed on purpose. An unknown scene on the wire is rejected
/// rather than silently stored, because a scene that no surface knows about
/// would be invisible to every filter dropdown and silently unretrievable.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum OrderScene {
    /// Web / desktop storefront checkout.
    Web,
    /// Mobile H5 checkout.
    H5,
    /// Native mobile application checkout (Flutter app).
    App,
    /// Mini-program checkout.
    MiniProgram,
    /// Cashier link opened before any order existed.
    Cashier,
    /// Subscription / membership purchase.
    Membership,
    /// Prepaid balance (token / points) recharge.
    Recharge,
    /// Platform-operated order (admin- or system-created).
    Platform,
    /// Order with no explicit scenario; the column default.
    #[default]
    Default,
}

impl OrderScene {
    /// Every scene, in the order surfaces should present them.
    pub const ALL: &'static [OrderScene] = &[
        OrderScene::Web,
        OrderScene::H5,
        OrderScene::App,
        OrderScene::MiniProgram,
        OrderScene::Cashier,
        OrderScene::Membership,
        OrderScene::Recharge,
        OrderScene::Platform,
        OrderScene::Default,
    ];

    /// Canonical wire/storage value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            OrderScene::Web => "web",
            OrderScene::H5 => "h5",
            OrderScene::App => "app",
            OrderScene::MiniProgram => "mini_program",
            OrderScene::Cashier => "cashier",
            OrderScene::Membership => "membership",
            OrderScene::Recharge => "recharge",
            OrderScene::Platform => "platform",
            OrderScene::Default => "default",
        }
    }

    /// Parses a wire/storage value, accepting the common aliases surfaces send
    /// (`miniProgram`, `miniapp`, `pc`) so a client cannot create a scene the
    /// filters can never show.
    pub fn parse(value: &str) -> Result<Self, CommerceServiceError> {
        let normalized = value.trim().to_ascii_lowercase();
        let canonical = normalized.replace(['-', ' '], "_");
        match canonical.as_str() {
            "web" | "pc" | "desktop" | "web_pc" => Ok(OrderScene::Web),
            "h5" | "wap" | "mobile_web" => Ok(OrderScene::H5),
            "app" | "native" | "flutter" | "android" | "ios" => Ok(OrderScene::App),
            "mini_program" | "miniprogram" | "miniapp" | "wechat" | "wechat_mini_program" => {
                Ok(OrderScene::MiniProgram)
            }
            "cashier" | "cashier_link" => Ok(OrderScene::Cashier),
            "membership" | "subscription" => Ok(OrderScene::Membership),
            "recharge" | "top_up" | "topup" => Ok(OrderScene::Recharge),
            "platform" | "admin" | "system" => Ok(OrderScene::Platform),
            "default" | "" => Ok(OrderScene::Default),
            _ => Err(CommerceServiceError::validation(format!(
                "unknown order scene: {value}"
            ))),
        }
    }
}

impl std::fmt::Display for OrderScene {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Which logical operator combines the requested tag filter values.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OrderTagMatchMode {
    /// The order must carry at least one of the requested tags.
    #[default]
    Any,
    /// The order must carry every requested tag.
    All,
}

impl OrderTagMatchMode {
    /// Parses `any` / `all` (case-insensitive); an absent value means [`Self::Any`].
    pub fn parse(value: &str) -> Result<Self, CommerceServiceError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "any" | "or" | "in" => Ok(Self::Any),
            "all" | "and" => Ok(Self::All),
            other => Err(CommerceServiceError::validation(format!(
                "unknown tag match mode: {other}"
            ))),
        }
    }
}

/// A normalized order tag.
///
/// Canonical form: lowercase alphanumeric plus `-`, `_`, and CJK characters,
/// with whitespace collapsed into `-`. The value is guaranteed non-empty and
/// at most [`ORDER_TAG_MAX_LENGTH`] characters once constructed.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct OrderTag(String);

impl OrderTag {
    /// Normalizes and validates one tag.
    ///
    /// Returns `Ok(None)` for input that normalizes to nothing (for example
    /// `"   "` or `"---"`) so callers can drop blank tokens from a comma
    /// separated list without treating them as an error.
    pub fn normalize(value: &str) -> Result<Option<Self>, CommerceServiceError> {
        let mut canonical = String::with_capacity(value.len());
        let mut last_was_separator = false;
        for character in value.trim().chars() {
            let mapped = match character {
                _ if character.is_alphanumeric() => {
                    for lowered in character.to_lowercase() {
                        canonical.push(lowered);
                    }
                    last_was_separator = false;
                    continue;
                }
                '_' | '-' | '.' => '-',
                _ if character.is_whitespace() => '-',
                _ => {
                    return Err(CommerceServiceError::validation(format!(
                        "order tag contains an unsupported character: {character:?}"
                    )));
                }
            };
            // Collapse runs of separators (`a  b` -> `a-b`) and never start
            // with one, so normalization is idempotent.
            if last_was_separator || canonical.is_empty() {
                continue;
            }
            canonical.push(mapped);
            last_was_separator = true;
        }
        while canonical.ends_with('-') {
            canonical.pop();
        }
        if canonical.is_empty() {
            return Ok(None);
        }
        if canonical.chars().count() > ORDER_TAG_MAX_LENGTH {
            return Err(CommerceServiceError::validation(format!(
                "order tag must be at most {ORDER_TAG_MAX_LENGTH} characters"
            )));
        }
        Ok(Some(Self(canonical)))
    }

    /// Parses an already-canonical tag (storage / wire round-trip).
    ///
    /// Unlike [`Self::normalize`], a value that would change under
    /// normalization is rejected: stored data must already be canonical.
    pub fn parse_strict(value: &str) -> Result<Self, CommerceServiceError> {
        let normalized = Self::normalize(value)?.ok_or_else(|| {
            CommerceServiceError::validation("order tag must not be empty")
        })?;
        if normalized.0 != value {
            return Err(CommerceServiceError::validation(format!(
                "order tag is not canonical: {value}"
            )));
        }
        Ok(normalized)
    }

    /// Canonical string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the tag, returning its canonical string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for OrderTag {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Result of parsing a multi-valued tag list from a filter or a request body.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OrderTagSet {
    tags: Vec<OrderTag>,
}

impl OrderTagSet {
    /// Normalizes, deduplicates, and bounds a tag list.
    ///
    /// `field_name` names the offending parameter in validation errors.
    pub fn parse_many<I, S>(
        values: I,
        field_name: &str,
        max_count: usize,
    ) -> Result<Self, CommerceServiceError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut tags: Vec<OrderTag> = Vec::new();
        for raw in values {
            let Some(tag) = OrderTag::normalize(raw.as_ref())? else {
                continue;
            };
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        if tags.len() > max_count {
            return Err(CommerceServiceError::validation(format!(
                "{field_name} accepts at most {max_count} values"
            )));
        }
        Ok(Self { tags })
    }

    /// Splits a comma/pipe separated parameter (`tags=vip` / `tags=vip,risk`)
    /// and additionally accepts the repeated-parameter form via `extra`.
    ///
    /// This is the industry-standard shape for a multi-select facet in a query
    /// string: both `tags=a,b` and `tags=a&tags=b` must mean the same thing.
    pub fn parse_csv_and_repeated<S>(
        csv: Option<&str>,
        repeated: &[S],
        field_name: &str,
    ) -> Result<Self, CommerceServiceError>
    where
        S: AsRef<str>,
    {
        let mut values: Vec<String> = Vec::new();
        if let Some(csv) = csv {
            for part in csv.split([',', '|']) {
                values.push(part.to_owned());
            }
        }
        for value in repeated {
            values.push(value.as_ref().to_owned());
        }
        Self::parse_many(values, field_name, ORDER_TAG_FILTER_MAX_COUNT)
    }

    /// The normalized tags, in request order.
    #[must_use]
    pub fn as_slice(&self) -> &[OrderTag] {
        &self.tags
    }

    /// `true` when no tag was requested (filter disabled).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    /// Number of requested tags.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tags.len()
    }

    /// Canonical string values, for transport and SQL binding.
    #[must_use]
    pub fn to_values(&self) -> Vec<String> {
        self.tags.iter().map(|tag| tag.0.clone()).collect()
    }
}

/// Validates and trims the operator remark.
pub fn normalize_order_remark(
    value: Option<&str>,
) -> Result<Option<String>, CommerceServiceError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > ORDER_REMARK_MAX_LENGTH {
        return Err(CommerceServiceError::validation(format!(
            "order remark must be at most {ORDER_REMARK_MAX_LENGTH} characters"
        )));
    }
    Ok(Some(trimmed.to_owned()))
}

/// Parses a wire scene filter value; empty/absent means "no scene filter".
pub fn parse_optional_scene(
    value: Option<&str>,
) -> Result<Option<OrderScene>, CommerceServiceError> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some(raw) if raw.eq_ignore_ascii_case("all") || raw == "*" => Ok(None),
        Some(raw) => OrderScene::parse(raw).map(Some),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_normalization_is_idempotent_and_case_folded() {
        for (input, expected) in [
            ("VIP", "vip"),
            ("  vip  ", "vip"),
            ("summer sale", "summer-sale"),
            ("Risk__Review", "risk-review"),
            ("a--b", "a-b"),
            ("-lead", "lead"),
            ("trail-", "trail"),
            ("高价值", "高价值"),
            ("v2.0", "v2-0"),
        ] {
            let tag = OrderTag::normalize(input)
                .unwrap_or_else(|error| panic!("{input:?} rejected: {}", error.message()))
                .unwrap_or_else(|| panic!("{input:?} normalized to nothing"));
            assert_eq!(tag.as_str(), expected, "input {input:?}");
            let again = OrderTag::normalize(tag.as_str()).unwrap().unwrap();
            assert_eq!(again, tag, "normalization must be idempotent for {input:?}");
        }
    }

    #[test]
    fn blank_tags_normalize_to_none() {
        for input in ["", "   ", "---", "___", "  -  "] {
            assert!(
                OrderTag::normalize(input).unwrap().is_none(),
                "{input:?} should be dropped"
            );
        }
    }

    #[test]
    fn tag_rejects_unsupported_characters_and_length() {
        assert!(OrderTag::normalize("vip!").is_err());
        assert!(OrderTag::normalize("a/b").is_err());
        let too_long = "x".repeat(ORDER_TAG_MAX_LENGTH + 1);
        assert!(OrderTag::normalize(&too_long).is_err());
        let at_limit = "x".repeat(ORDER_TAG_MAX_LENGTH);
        assert!(OrderTag::normalize(&at_limit).is_ok());
    }

    #[test]
    fn strict_parse_rejects_non_canonical_storage() {
        assert!(OrderTag::parse_strict("vip").is_ok());
        assert!(OrderTag::parse_strict("VIP").is_err());
        assert!(OrderTag::parse_strict(" summer sale ").is_err());
    }

    #[test]
    fn tag_set_dedupes_and_bounds() {
        let set = OrderTagSet::parse_many(["VIP", "vip", " ", "risk"], "tags", 4).unwrap();
        assert_eq!(set.to_values(), vec!["vip".to_owned(), "risk".to_owned()]);
        assert!(OrderTagSet::parse_many(["a", "b", "c", "d", "e"], "tags", 4).is_err());
    }

    #[test]
    fn tag_set_accepts_csv_and_repeated_parameters() {
        let set =
            OrderTagSet::parse_csv_and_repeated(Some("vip, risk"), &["campaign"], "tags").unwrap();
        assert_eq!(
            set.to_values(),
            vec!["vip".to_owned(), "risk".to_owned(), "campaign".to_owned()]
        );
        let none = OrderTagSet::parse_csv_and_repeated::<&str>(None, &[], "tags").unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn scene_parses_aliases_and_rejects_unknown() {
        assert_eq!(OrderScene::parse("WEB").unwrap(), OrderScene::Web);
        assert_eq!(OrderScene::parse("pc").unwrap(), OrderScene::Web);
        assert_eq!(
            OrderScene::parse("miniProgram").unwrap(),
            OrderScene::MiniProgram
        );
        assert_eq!(
            OrderScene::parse("mini-program").unwrap(),
            OrderScene::MiniProgram
        );
        assert_eq!(OrderScene::parse("h5").unwrap(), OrderScene::H5);
        assert_eq!(OrderScene::parse("flutter").unwrap(), OrderScene::App);
        assert!(OrderScene::parse("telepathy").is_err());
    }

    #[test]
    fn scene_all_is_round_trippable() {
        for scene in OrderScene::ALL {
            assert_eq!(OrderScene::parse(scene.as_str()).unwrap(), *scene);
        }
        assert_eq!(OrderScene::default().as_str(), "default");
    }

    #[test]
    fn optional_scene_treats_all_as_no_filter() {
        assert!(parse_optional_scene(None).unwrap().is_none());
        assert!(parse_optional_scene(Some("  ")).unwrap().is_none());
        assert!(parse_optional_scene(Some("all")).unwrap().is_none());
        assert!(parse_optional_scene(Some("*")).unwrap().is_none());
        assert_eq!(
            parse_optional_scene(Some("h5")).unwrap(),
            Some(OrderScene::H5)
        );
    }

    #[test]
    fn tag_match_mode_parses() {
        assert_eq!(OrderTagMatchMode::parse("").unwrap(), OrderTagMatchMode::Any);
        assert_eq!(
            OrderTagMatchMode::parse("AND").unwrap(),
            OrderTagMatchMode::All
        );
        assert!(OrderTagMatchMode::parse("xor").is_err());
    }

    #[test]
    fn remark_is_trimmed_and_bounded() {
        assert_eq!(normalize_order_remark(None).unwrap(), None);
        assert_eq!(normalize_order_remark(Some("  ")).unwrap(), None);
        assert_eq!(
            normalize_order_remark(Some("  fraud review ")).unwrap(),
            Some("fraud review".to_owned())
        );
        let too_long = "x".repeat(ORDER_REMARK_MAX_LENGTH + 1);
        assert!(normalize_order_remark(Some(&too_long)).is_err());
    }
}
