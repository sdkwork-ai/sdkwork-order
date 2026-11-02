//! Demand claim (抢单) domain: the provider's binding offer against a published
//! demand, its status machine, and the contract types shared by the repository
//! and the app router.
//!
//! The demand itself is owned by the company domain; a claim snapshots the
//! demand facts at submission time so later demand edits never rewrite a
//! provider's bid history. `specs/DEMAND_CLAIM_SPEC.md` is the product contract:
//! statuses, transitions, roles, and the delivery lifecycle.

use chrono::Utc;
use sdkwork_contract_service::CommerceServiceError;

use crate::validation::require_non_empty;

pub const DEMAND_CLAIM_STATUS_SUBMITTED: &str = "submitted";
pub const DEMAND_CLAIM_STATUS_SHORTLISTED: &str = "shortlisted";
pub const DEMAND_CLAIM_STATUS_ACCEPTED: &str = "accepted";
pub const DEMAND_CLAIM_STATUS_IN_DELIVERY: &str = "in_delivery";
pub const DEMAND_CLAIM_STATUS_DELIVERED: &str = "delivered";
pub const DEMAND_CLAIM_STATUS_COMPLETED: &str = "completed";
pub const DEMAND_CLAIM_STATUS_REJECTED: &str = "rejected";
pub const DEMAND_CLAIM_STATUS_WITHDRAWN: &str = "withdrawn";
pub const DEMAND_CLAIM_STATUS_CANCELLED: &str = "cancelled";

/// Longest proposal accepted, so one claim cannot store an unbounded document.
pub const DEMAND_CLAIM_PROPOSAL_MAX_LEN: usize = 10_000;
/// Upper bound of a single quote, in major currency units.
pub const DEMAND_CLAIM_QUOTE_MAX: &str = "1000000000";
/// Application forms a demand may commission, one code per SDKWork
/// application architecture standard. Kept in the claim domain because the
/// claim snapshots the demand's delivery forms.
pub const DEMAND_APPLICATION_FORMS: [&str; 10] = [
    "pc",
    "h5",
    "static-web",
    "miniprogram",
    "flutter",
    "uniapp",
    "android-native",
    "ios-native",
    "harmony-native",
    "unity",
];
/// A demand cannot commission an unbounded number of delivery forms.
pub const DEMAND_APPLICATION_FORMS_MAX: usize = 10;

/// Claim lifecycle states, in transition order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemandClaimStatus {
    Submitted,
    Shortlisted,
    Accepted,
    InDelivery,
    Delivered,
    Completed,
    Rejected,
    Withdrawn,
    Cancelled,
}

impl DemandClaimStatus {
    pub const ALL: [Self; 9] = [
        Self::Submitted,
        Self::Shortlisted,
        Self::Accepted,
        Self::InDelivery,
        Self::Delivered,
        Self::Completed,
        Self::Rejected,
        Self::Withdrawn,
        Self::Cancelled,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => DEMAND_CLAIM_STATUS_SUBMITTED,
            Self::Shortlisted => DEMAND_CLAIM_STATUS_SHORTLISTED,
            Self::Accepted => DEMAND_CLAIM_STATUS_ACCEPTED,
            Self::InDelivery => DEMAND_CLAIM_STATUS_IN_DELIVERY,
            Self::Delivered => DEMAND_CLAIM_STATUS_DELIVERED,
            Self::Completed => DEMAND_CLAIM_STATUS_COMPLETED,
            Self::Rejected => DEMAND_CLAIM_STATUS_REJECTED,
            Self::Withdrawn => DEMAND_CLAIM_STATUS_WITHDRAWN,
            Self::Cancelled => DEMAND_CLAIM_STATUS_CANCELLED,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            DEMAND_CLAIM_STATUS_SUBMITTED => Some(Self::Submitted),
            DEMAND_CLAIM_STATUS_SHORTLISTED => Some(Self::Shortlisted),
            DEMAND_CLAIM_STATUS_ACCEPTED => Some(Self::Accepted),
            DEMAND_CLAIM_STATUS_IN_DELIVERY => Some(Self::InDelivery),
            DEMAND_CLAIM_STATUS_DELIVERED => Some(Self::Delivered),
            DEMAND_CLAIM_STATUS_COMPLETED => Some(Self::Completed),
            DEMAND_CLAIM_STATUS_REJECTED => Some(Self::Rejected),
            DEMAND_CLAIM_STATUS_WITHDRAWN => Some(Self::Withdrawn),
            DEMAND_CLAIM_STATUS_CANCELLED => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn group(self) -> DemandClaimStatusGroup {
        match self {
            Self::Submitted
            | Self::Shortlisted
            | Self::Accepted
            | Self::InDelivery
            | Self::Delivered => DemandClaimStatusGroup::InProgress,
            Self::Completed => DemandClaimStatusGroup::Completed,
            Self::Rejected | Self::Withdrawn | Self::Cancelled => DemandClaimStatusGroup::Closed,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Rejected | Self::Withdrawn | Self::Cancelled
        )
    }

    /// The milestone column an entry into this state stamps.
    pub fn milestone(self) -> Option<DemandClaimMilestone> {
        match self {
            Self::Submitted => Some(DemandClaimMilestone::Submitted),
            Self::Accepted => Some(DemandClaimMilestone::Accepted),
            Self::InDelivery => Some(DemandClaimMilestone::Started),
            Self::Delivered => Some(DemandClaimMilestone::Delivered),
            Self::Completed => Some(DemandClaimMilestone::Completed),
            Self::Rejected | Self::Withdrawn | Self::Cancelled => {
                Some(DemandClaimMilestone::Closed)
            }
            Self::Shortlisted => None,
        }
    }
}

/// Milestone timestamp columns written when a claim enters a state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemandClaimMilestone {
    Submitted,
    Accepted,
    Started,
    Delivered,
    Completed,
    Closed,
}

/// The three groups the My Claims page filters by.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemandClaimStatusGroup {
    InProgress,
    Completed,
    Closed,
}

impl DemandClaimStatusGroup {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Closed => "closed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "in_progress" => Some(Self::InProgress),
            "completed" => Some(Self::Completed),
            "closed" => Some(Self::Closed),
            _ => None,
        }
    }

    /// Statuses a group query selects.
    pub fn statuses(self) -> Vec<&'static str> {
        DemandClaimStatus::ALL
            .iter()
            .filter(|status| status.group() == self)
            .map(|status| status.as_str())
            .collect()
    }
}

/// Who acts on a claim. The actor is derived from the authenticated subject,
/// never from the request body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemandClaimActorType {
    Provider,
    Publisher,
    Operator,
    System,
}

impl DemandClaimActorType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::Publisher => "publisher",
            Self::Operator => "operator",
            Self::System => "system",
        }
    }
}

/// Which side of a claim a list query is scoped to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemandClaimViewerRole {
    Provider,
    Publisher,
    Operator,
}

impl DemandClaimViewerRole {
    pub fn parse(value: Option<&str>) -> Result<Self, CommerceServiceError> {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None | Some("provider") | Some("bidder") => Ok(Self::Provider),
            Some("publisher") => Ok(Self::Publisher),
            Some("operator") => Ok(Self::Operator),
            Some(other) => Err(CommerceServiceError::validation(format!(
                "unknown claim role: {other}"
            ))),
        }
    }
}

/// One claim action, with the states it may start from and the state it
/// produces, plus the actor allowed to perform it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemandClaimAction {
    Shortlist,
    Accept,
    Reject,
    Withdraw,
    Start,
    Deliver,
    Complete,
    Rework,
    Cancel,
}

impl DemandClaimAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shortlist => "shortlist",
            Self::Accept => "accept",
            Self::Reject => "reject",
            Self::Withdraw => "withdraw",
            Self::Start => "start",
            Self::Deliver => "deliver",
            Self::Complete => "complete",
            Self::Rework => "rework",
            Self::Cancel => "cancel",
        }
    }

    pub fn actor(self) -> DemandClaimActorType {
        match self {
            Self::Shortlist
            | Self::Accept
            | Self::Reject
            | Self::Complete
            | Self::Rework
            | Self::Cancel => DemandClaimActorType::Publisher,
            Self::Withdraw | Self::Start | Self::Deliver => DemandClaimActorType::Provider,
        }
    }

    fn from_statuses(self) -> &'static [DemandClaimStatus] {
        match self {
            Self::Shortlist => &[DemandClaimStatus::Submitted],
            Self::Accept => &[DemandClaimStatus::Submitted, DemandClaimStatus::Shortlisted],
            Self::Reject => &[DemandClaimStatus::Submitted, DemandClaimStatus::Shortlisted],
            Self::Withdraw => &[DemandClaimStatus::Submitted, DemandClaimStatus::Shortlisted],
            Self::Start => &[DemandClaimStatus::Accepted],
            Self::Deliver => &[DemandClaimStatus::InDelivery],
            Self::Complete => &[DemandClaimStatus::Delivered],
            Self::Rework => &[DemandClaimStatus::Delivered],
            Self::Cancel => &[DemandClaimStatus::Accepted, DemandClaimStatus::InDelivery],
        }
    }

    fn to_status(self) -> DemandClaimStatus {
        match self {
            Self::Shortlist => DemandClaimStatus::Shortlisted,
            Self::Accept => DemandClaimStatus::Accepted,
            Self::Reject => DemandClaimStatus::Rejected,
            Self::Withdraw => DemandClaimStatus::Withdrawn,
            Self::Start => DemandClaimStatus::InDelivery,
            Self::Deliver => DemandClaimStatus::Delivered,
            Self::Complete => DemandClaimStatus::Completed,
            Self::Rework => DemandClaimStatus::InDelivery,
            Self::Cancel => DemandClaimStatus::Cancelled,
        }
    }

    /// Validate this action against the claim's current state.
    /// @returns the next status.
    pub fn validate(
        self,
        current: DemandClaimStatus,
    ) -> Result<DemandClaimStatus, CommerceServiceError> {
        if self.from_statuses().contains(&current) {
            Ok(self.to_status())
        } else {
            Err(CommerceServiceError::invalid_state(format!(
                "claim action {} is not allowed from status {}",
                self.as_str(),
                current.as_str()
            )))
        }
    }
}

/// Create-claim payload. Bidder identity is not part of it: the router takes it
/// from the authenticated subject.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDemandClaimCommand {
    pub demand_id: String,
    pub demand_title: String,
    #[serde(default)]
    pub demand_category: Option<String>,
    #[serde(default)]
    pub demand_type: Option<String>,
    #[serde(default)]
    /// @deprecated Single-form compatibility alias; prefer `app_forms`.
    pub app_form: Option<String>,
    #[serde(default)]
    /// Ordered application forms snapshotted from the demand.
    pub app_forms: Vec<String>,
    #[serde(default)]
    pub budget_text: Option<String>,
    #[serde(default)]
    pub budget_min: Option<String>,
    #[serde(default)]
    pub budget_max: Option<String>,
    #[serde(default)]
    pub bid_deadline: Option<String>,
    pub publisher_user_id: String,
    #[serde(default)]
    pub publisher_company_name: Option<String>,
    #[serde(default)]
    pub bidder_company_id: Option<String>,
    pub quote: String,
    #[serde(default)]
    pub quote_currency: Option<String>,
    #[serde(default)]
    pub delivery_days: Option<i32>,
    pub proposal: String,
    #[serde(default)]
    pub request_no: Option<String>,
}

/// Body of a claim action; the same shape is used by every transition so the
/// router keeps one extractor.
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemandClaimActionCommand {
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub delivery_note: Option<String>,
    #[serde(default)]
    pub attachment_urls: Vec<String>,
}

/// Server-validated create payload handed to the repository.
#[derive(Clone, Debug)]
pub struct DemandClaimInsert {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub claim_no: String,
    pub demand_id: String,
    pub demand_title: String,
    pub demand_category: Option<String>,
    pub demand_type: String,
    pub app_form: Option<String>,
    pub app_forms: Vec<String>,
    pub budget_text: Option<String>,
    pub budget_min: Option<String>,
    pub budget_max: Option<String>,
    pub bid_deadline: Option<String>,
    pub publisher_user_id: String,
    pub publisher_company_name: Option<String>,
    pub bidder_user_id: String,
    pub bidder_company_id: Option<String>,
    pub quote_amount_minor: String,
    pub quote_currency: String,
    pub delivery_days: Option<i32>,
    pub proposal: String,
    pub request_no: String,
    pub idempotency_key: String,
    pub now: String,
}

/// The state changes a transition writes. `None` fields stay untouched.
#[derive(Clone, Debug)]
pub struct DemandClaimTransitionPatch {
    pub tenant_id: String,
    pub claim_id: String,
    pub demand_id: String,
    pub expected_status: String,
    pub next_status: String,
    pub milestone: Option<DemandClaimMilestone>,
    pub reject_reason: Option<String>,
    pub withdraw_reason: Option<String>,
    pub cancel_reason: Option<String>,
    pub delivery_note: Option<String>,
    pub delivery_attachment_urls: Option<Vec<String>>,
    pub now: String,
}

/// One immutable timeline entry.
#[derive(Clone, Debug)]
pub struct DemandClaimEventInsert {
    pub id: String,
    pub tenant_id: String,
    pub claim_id: String,
    pub event_no: String,
    pub event_type: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub message: Option<String>,
    pub payload_json: String,
    pub request_id: Option<String>,
    pub idempotency_key: String,
    pub now: String,
}

/// A claim as returned by the API.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemandClaimView {
    pub id: String,
    pub tenant_id: String,
    pub claim_no: String,
    pub demand_id: String,
    pub demand_title: String,
    pub demand_category: Option<String>,
    pub demand_type: String,
    pub app_form: Option<String>,
    pub app_forms: Vec<String>,
    pub budget_text: Option<String>,
    pub budget_min: Option<String>,
    pub budget_max: Option<String>,
    pub bid_deadline: Option<String>,
    pub publisher_user_id: String,
    pub publisher_company_name: Option<String>,
    pub bidder_user_id: String,
    pub bidder_company_id: Option<String>,
    pub quote: String,
    pub quote_currency: String,
    pub delivery_days: Option<i32>,
    pub proposal: String,
    pub status: String,
    pub status_group: String,
    pub reject_reason: Option<String>,
    pub withdraw_reason: Option<String>,
    pub cancel_reason: Option<String>,
    pub delivery_note: Option<String>,
    pub delivery_attachment_urls: Vec<String>,
    pub order_id: Option<String>,
    pub order_no: Option<String>,
    pub role: String,
    pub submitted_at: String,
    pub accepted_at: Option<String>,
    pub started_at: Option<String>,
    pub delivered_at: Option<String>,
    pub completed_at: Option<String>,
    pub closed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// One timeline entry as returned by the API.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemandClaimEventView {
    pub id: String,
    pub claim_id: String,
    pub event_type: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub message: Option<String>,
    pub payload: serde_json::Value,
    pub created_at: String,
}

/// Tab counters for My Claims.
#[derive(Clone, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemandClaimStatisticsView {
    pub in_progress: i64,
    pub completed: i64,
    pub closed: i64,
    pub total: i64,
}

/// List result handed to the route layer.
#[derive(Clone, Debug)]
pub struct DemandClaimListPage {
    pub items: Vec<DemandClaimView>,
    pub total_items: i64,
}

/// Validated list query for the My Claims page and the publisher inbox.
#[derive(Clone, Debug)]
pub struct DemandClaimListQuery {
    pub tenant_id: String,
    pub viewer_user_id: String,
    pub role: DemandClaimViewerRole,
    pub status: Option<DemandClaimStatus>,
    pub status_group: Option<DemandClaimStatusGroup>,
    pub demand_id: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

/// Validate and normalize a create command into the insert shape.
/// @param command - the client payload.
/// @param bidder_user_id - the authenticated provider.
/// @param request_no - transport request id used for traceability.
/// @returns the insert row the repository writes.
pub fn validate_demand_claim_command(
    command: CreateDemandClaimCommand,
    tenant_id: &str,
    organization_id: Option<&str>,
    bidder_user_id: &str,
    request_no: &str,
) -> Result<DemandClaimInsert, CommerceServiceError> {
    require_non_empty("tenantId", tenant_id)?;
    require_non_empty("demandId", &command.demand_id)?;
    require_non_empty("demandTitle", &command.demand_title)?;
    require_non_empty("publisherUserId", &command.publisher_user_id)?;
    require_non_empty("bidderUserId", bidder_user_id)?;
    require_non_empty("proposal", &command.proposal)?;
    if command.proposal.trim().chars().count() > DEMAND_CLAIM_PROPOSAL_MAX_LEN {
        return Err(CommerceServiceError::validation(format!(
            "proposal must be at most {DEMAND_CLAIM_PROPOSAL_MAX_LEN} characters"
        )));
    }
    if command.publisher_user_id.trim() == bidder_user_id.trim() {
        return Err(CommerceServiceError::validation(
            "a publisher cannot claim their own demand",
        ));
    }
    let quote_minor = quote_major_to_minor(&command.quote)?;
    // The demand type is a company-owned taxonomy snapshot. Order validates the
    // wire token rather than a closed list, so a new company demand family
    // (video, image, music, voice, ...) does not require an order release.
    let demand_type = match command.demand_type.as_deref().map(str::trim) {
        None | Some("") => "other".to_owned(),
        Some(value)
            if value.len() <= 32
                && value
                    .chars()
                    .next()
                    .is_some_and(|first| first.is_ascii_lowercase())
                && value.chars().all(|item| {
                    item.is_ascii_lowercase() || item.is_ascii_digit() || item == '-' || item == '_'
                }) =>
        {
            value.to_owned()
        }
        Some(other) => {
            return Err(CommerceServiceError::validation(format!(
                "unknown demandType: {other}"
            )))
        }
    };
    if let Some(days) = command.delivery_days {
        if days <= 0 {
            return Err(CommerceServiceError::validation(
                "deliveryDays must be a positive integer",
            ));
        }
    }
    let app_forms = normalize_claim_app_forms(&command.app_forms, command.app_form.as_deref())?;
    if demand_type == "development" && app_forms.is_empty() {
        return Err(CommerceServiceError::validation(
            "appForms is required for a development demand",
        ));
    }
    let app_form = app_forms.first().cloned();
    let bid_deadline = command
        .bid_deadline
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    if let Some(deadline) = bid_deadline.as_deref() {
        let parsed = chrono::DateTime::parse_from_rfc3339(deadline).map_err(|_| {
            CommerceServiceError::validation("bidDeadline must be an RFC 3339 instant")
        })?;
        if parsed.with_timezone(&Utc) < Utc::now() {
            return Err(CommerceServiceError::invalid_state(
                "the demand bid deadline has passed",
            ));
        }
    }
    let now = crate::clock::canonical_now_timestamp();
    let id = sdkwork_utils_rust::uuid();
    let claim_no = build_claim_no(&now, &id);
    let quote_currency = command
        .quote_currency
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("CNY")
        .to_owned();
    let resolved_request_no = {
        let value = request_no.trim();
        if value.is_empty() {
            build_request_no(&id)
        } else {
            value.to_owned()
        }
    };
    Ok(DemandClaimInsert {
        id,
        tenant_id: tenant_id.trim().to_owned(),
        organization_id: organization_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        claim_no,
        demand_id: command.demand_id.trim().to_owned(),
        demand_title: command.demand_title.trim().to_owned(),
        demand_category: normalize_optional(command.demand_category),
        demand_type,
        app_form,
        app_forms,
        budget_text: normalize_optional(command.budget_text),
        budget_min: normalize_optional(command.budget_min),
        budget_max: normalize_optional(command.budget_max),
        bid_deadline,
        publisher_user_id: command.publisher_user_id.trim().to_owned(),
        publisher_company_name: normalize_optional(command.publisher_company_name),
        bidder_user_id: bidder_user_id.trim().to_owned(),
        bidder_company_id: normalize_optional(command.bidder_company_id),
        quote_amount_minor: quote_minor,
        quote_currency,
        delivery_days: command.delivery_days,
        proposal: command.proposal.trim().to_owned(),
        request_no: resolved_request_no.clone(),
        idempotency_key: resolved_request_no,
        now,
    })
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
}

/// Normalize the application-form snapshot: legacy single form when the array
/// is absent, canonical architecture-standard order, deduplicated and bounded.
fn normalize_claim_app_forms(
    values: &[String],
    legacy: Option<&str>,
) -> Result<Vec<String>, CommerceServiceError> {
    let candidates: Vec<&str> = if values.is_empty() {
        legacy.into_iter().collect()
    } else {
        values.iter().map(String::as_str).collect()
    };
    let mut forms: Vec<String> = Vec::new();
    for raw in candidates {
        let value = raw.trim();
        if value.is_empty() {
            continue;
        }
        if !DEMAND_APPLICATION_FORMS.contains(&value) {
            return Err(CommerceServiceError::validation(format!(
                "unknown appForm: {value}"
            )));
        }
        if !forms.iter().any(|form| form == value) {
            forms.push(value.to_owned());
        }
    }
    if forms.len() > DEMAND_APPLICATION_FORMS_MAX {
        return Err(CommerceServiceError::validation(
            "at most 10 application forms can be selected",
        ));
    }
    forms.sort_by_key(|form| {
        DEMAND_APPLICATION_FORMS
            .iter()
            .position(|candidate| candidate == form)
            .unwrap_or(usize::MAX)
    });
    Ok(forms)
}

/// Validate a quote string: a positive decimal with at most two fraction
/// digits and an upper bound, returned in canonical `0.00` form.
pub fn normalize_quote(value: &str) -> Result<String, CommerceServiceError> {
    let text = value.trim();
    if text.is_empty() {
        return Err(CommerceServiceError::validation("quote is required"));
    }
    let (integer, fraction) = match text.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (text, None),
    };
    if integer.is_empty() || !integer.chars().all(|item| item.is_ascii_digit()) {
        return Err(CommerceServiceError::validation(
            "quote must be a positive amount",
        ));
    }
    if let Some(fraction) = fraction {
        if fraction.is_empty()
            || fraction.len() > 2
            || !fraction.chars().all(|item| item.is_ascii_digit())
        {
            return Err(CommerceServiceError::validation(
                "quote must use at most two decimal places",
            ));
        }
    }
    let normalized_integer = integer.trim_start_matches('0');
    let integer = if normalized_integer.is_empty() {
        "0"
    } else {
        normalized_integer
    };
    let fraction = fraction.unwrap_or("00");
    let padded_fraction = if fraction.len() == 1 {
        format!("{fraction}0")
    } else {
        fraction.to_owned()
    };
    let canonical = format!("{integer}.{padded_fraction}");
    if canonical == "0.00" {
        return Err(CommerceServiceError::validation(
            "quote must be greater than zero",
        ));
    }
    let max = DEMAND_CLAIM_QUOTE_MAX;
    if integer.len() > max.len() || (integer.len() == max.len() && integer > max) {
        return Err(CommerceServiceError::validation("quote is too large"));
    }
    Ok(canonical)
}

/// Convert a validated major-unit quote into the order domain's canonical
/// minor-unit storage form (`"96000.00"` → `"9600000"`).
pub fn quote_major_to_minor(value: &str) -> Result<String, CommerceServiceError> {
    let canonical = normalize_quote(value)?;
    let (integer, fraction) = canonical
        .split_once('.')
        .expect("normalized quote always has two fraction digits");
    let digits = format!("{integer}{fraction}");
    let trimmed = digits.trim_start_matches('0');
    Ok(if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    })
}

/// Project stored minor units back to the major-unit wire form
/// (`"9600000"` → `"96000.00"`), so every SDK keeps one money convention.
pub fn quote_minor_to_major(value: &str) -> String {
    let digits = value.trim().trim_start_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    if digits.len() <= 2 {
        let padded = format!("{}{}", "0".repeat(3 - digits.len()), digits);
        let (integer, fraction) = padded.split_at(1);
        return format!("{integer}.{fraction}");
    }
    let (integer, fraction) = digits.split_at(digits.len() - 2);
    format!("{integer}.{fraction}")
}

fn build_claim_no(now: &str, id: &str) -> String {
    let stamp = now
        .chars()
        .filter(|item| item.is_ascii_digit())
        .take(17)
        .collect::<String>();
    let suffix = id
        .chars()
        .filter(|item| item.is_ascii_alphanumeric())
        .take(6)
        .collect::<String>();
    format!("DC{stamp}{}", suffix.to_ascii_uppercase())
}

fn build_request_no(id: &str) -> String {
    format!(
        "DCREQ{}",
        id.chars()
            .filter(|item| item.is_ascii_alphanumeric())
            .take(12)
            .collect::<String>()
    )
}

/// Map a stored status string onto the domain enum, defaulting unknown values
/// to `submitted` so a legacy row cannot crash a list query.
pub fn parse_status_or_default(value: &str) -> DemandClaimStatus {
    DemandClaimStatus::parse(value).unwrap_or(DemandClaimStatus::Submitted)
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_claim_app_forms, normalize_quote, quote_major_to_minor, quote_minor_to_major,
        DemandClaimAction, DemandClaimStatus, DemandClaimStatusGroup,
    };

    #[test]
    fn claim_app_forms_normalize_to_canonical_order_and_accept_legacy_single() {
        assert_eq!(
            normalize_claim_app_forms(&["h5".to_owned(), "pc".to_owned(), "h5".to_owned()], None)
                .expect("forms"),
            vec!["pc".to_owned(), "h5".to_owned()]
        );
        assert_eq!(
            normalize_claim_app_forms(&[], Some("miniprogram")).expect("legacy"),
            vec!["miniprogram".to_owned()]
        );
        assert!(normalize_claim_app_forms(&["native".to_owned()], None).is_err());
    }

    #[test]
    fn quote_normalization_accepts_decimals_and_rejects_zero() {
        assert_eq!(normalize_quote("50000").expect("integer"), "50000.00");
        assert_eq!(normalize_quote(" 12.5 ").expect("one decimal"), "12.50");
        assert_eq!(normalize_quote("0.01").expect("cent"), "0.01");
        assert!(normalize_quote("0").is_err());
        assert!(normalize_quote("-1").is_err());
        assert!(normalize_quote("1.234").is_err());
        assert!(normalize_quote("abc").is_err());
    }

    #[test]
    fn quote_units_round_trip_between_major_and_minor() {
        assert_eq!(quote_major_to_minor("96000.00").expect("minor"), "9600000");
        assert_eq!(quote_minor_to_major("9600000"), "96000.00");
        assert_eq!(quote_minor_to_major("5"), "0.05");
        assert_eq!(quote_minor_to_major("0"), "0.00");
        assert!(quote_major_to_minor("0.00").is_err());
    }

    #[test]
    fn transition_table_is_role_and_state_aware() {
        assert_eq!(
            DemandClaimAction::Accept
                .validate(DemandClaimStatus::Shortlisted)
                .expect("shortlisted can be accepted"),
            DemandClaimStatus::Accepted
        );
        assert!(DemandClaimAction::Accept
            .validate(DemandClaimStatus::Delivered)
            .is_err());
        assert!(DemandClaimAction::Deliver
            .validate(DemandClaimStatus::Accepted)
            .is_err());
        assert_eq!(
            DemandClaimAction::Rework
                .validate(DemandClaimStatus::Delivered)
                .expect("delivered can be reworked"),
            DemandClaimStatus::InDelivery
        );
    }

    #[test]
    fn status_groups_match_the_my_claims_tabs() {
        assert_eq!(
            DemandClaimStatus::Delivered.group(),
            DemandClaimStatusGroup::InProgress
        );
        assert_eq!(
            DemandClaimStatus::Completed.group(),
            DemandClaimStatusGroup::Completed
        );
        assert_eq!(
            DemandClaimStatus::Withdrawn.group(),
            DemandClaimStatusGroup::Closed
        );
        assert_eq!(
            DemandClaimStatusGroup::InProgress.statuses(),
            vec![
                "submitted",
                "shortlisted",
                "accepted",
                "in_delivery",
                "delivered"
            ]
        );
    }
}
