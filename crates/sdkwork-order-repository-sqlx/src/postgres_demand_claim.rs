//! PostgreSQL repository for the demand-claim (抢单) lifecycle.
//!
//! The repository owns the transactional orchestration the app router calls:
//! idempotent claim creation, guarded status transitions with their timeline
//! event, and the My Claims list/statistics queries. Domain validation and the
//! transition table live in `sdkwork_order_service::domain::demand_claim`.

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    parse_status_or_default, quote_major_to_minor, quote_minor_to_major, DemandClaimActorType,
    DemandClaimEventInsert, DemandClaimEventView, DemandClaimInsert, DemandClaimListQuery,
    DemandClaimMilestone, DemandClaimStatisticsView, DemandClaimStatus, DemandClaimStatusGroup,
    DemandClaimTransitionPatch, DemandClaimView, DemandClaimViewerRole,
};
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::sql_store_error::map_sqlx_store_error;

const CLAIM_SELECT_WITH_TIMESTAMPS: &str = r#"
SELECT
    id,
    tenant_id,
    claim_no,
    demand_id,
    demand_title,
    demand_category,
    demand_type,
    app_form,
    app_forms,
    budget_text,
    budget_min,
    budget_max,
    publisher_user_id,
    publisher_company_name,
    bidder_user_id,
    bidder_company_id,
    CAST(quote_amount_minor AS TEXT) AS quote_amount_minor,
    quote_currency,
    delivery_days,
    proposal,
    status,
    reject_reason,
    withdraw_reason,
    cancel_reason,
    delivery_note,
    delivery_attachment_urls,
    order_id,
    order_no,
    TO_CHAR(bid_deadline AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS bid_deadline,
    TO_CHAR(submitted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS submitted_at,
    TO_CHAR(accepted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS accepted_at,
    TO_CHAR(started_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS started_at,
    TO_CHAR(delivered_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS delivered_at,
    TO_CHAR(completed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS completed_at,
    TO_CHAR(closed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS closed_at,
    TO_CHAR(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS created_at,
    TO_CHAR(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS updated_at
FROM commerce_demand_claim
"#;

/// Role filter shared by the list and statistics queries.
const VIEWER_PREDICATE: &str = r#"
    tenant_id = $1
    AND (
        ($2::text = 'operator')
        OR ($2::text = 'publisher' AND publisher_user_id = $3)
        OR ($2::text = 'provider' AND bidder_user_id = $3)
    )
"#;

/// PostgreSQL-backed demand-claim store.
pub struct PostgresDemandClaimRepository {
    pool: PgPool,
}

impl PostgresDemandClaimRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a claim idempotently: the same provider re-submitting the same
    /// demand with the same quote and proposal receives the existing row.
    pub async fn create_claim(
        &self,
        insert: &DemandClaimInsert,
    ) -> Result<DemandClaimView, CommerceServiceError> {
        if let Some(existing) = self
            .find_bidder_claim(&insert.tenant_id, &insert.demand_id, &insert.bidder_user_id)
            .await?
        {
            let same_quote = quote_major_to_minor(&existing.quote)
                .is_ok_and(|value| value == insert.quote_amount_minor.as_str());
            if same_quote && existing.proposal == insert.proposal {
                if matches!(existing.status.as_str(), "submitted" | "shortlisted") {
                    return Ok(existing);
                }
                return Err(CommerceServiceError::conflict(
                    "your previous claim for this demand is already closed",
                ));
            }
            return Err(CommerceServiceError::conflict(
                "you already submitted a claim for this demand",
            ));
        }

        // A demand that already has an awarded claim no longer accepts new
        // claims, even if the hall UI is stale or the request bypasses it.
        let awarded_count = sqlx::query(
            r#"
            SELECT COUNT(*) AS total_items
            FROM commerce_demand_claim
            WHERE tenant_id = $1
              AND demand_id = $2
              AND status IN ('accepted', 'in_delivery', 'delivered', 'completed')
            "#,
        )
        .bind(&insert.tenant_id)
        .bind(&insert.demand_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| map_sqlx_store_error("count awarded demand claims", error))?
        .try_get::<i64, _>("total_items")
        .unwrap_or_default();
        if awarded_count > 0 {
            return Err(CommerceServiceError::conflict(
                "this demand has already been awarded",
            ));
        }

        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_store_error("begin demand claim transaction", error))?;
        let result = sqlx::query(
            r#"
            INSERT INTO commerce_demand_claim (
                id, tenant_id, organization_id, claim_no, demand_id, demand_title,
                demand_category, demand_type, app_form, budget_text, budget_min, budget_max,
                bid_deadline, publisher_user_id, publisher_company_name, bidder_user_id,
                bidder_company_id, quote_amount_minor, quote_currency, delivery_days, proposal, status,
                request_no, idempotency_key, submitted_at, created_at, updated_at, app_forms
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13::timestamptz, $14,
                $15, $16, $17, CAST($18 AS NUMERIC), $19, $20, $21, 'submitted', $22, $23,
                $24::timestamptz, $24::timestamptz, $24::timestamptz, $25
            )
            "#,
        )
        .bind(&insert.id)
        .bind(&insert.tenant_id)
        .bind(insert.organization_id.as_deref().unwrap_or("0"))
        .bind(&insert.claim_no)
        .bind(&insert.demand_id)
        .bind(&insert.demand_title)
        .bind(insert.demand_category.as_deref())
        .bind(&insert.demand_type)
        .bind(insert.app_form.as_deref())
        .bind(insert.budget_text.as_deref())
        .bind(insert.budget_min.as_deref())
        .bind(insert.budget_max.as_deref())
        .bind(insert.bid_deadline.as_deref())
        .bind(&insert.publisher_user_id)
        .bind(insert.publisher_company_name.as_deref())
        .bind(&insert.bidder_user_id)
        .bind(insert.bidder_company_id.as_deref())
        .bind(&insert.quote_amount_minor)
        .bind(&insert.quote_currency)
        .bind(insert.delivery_days)
        .bind(&insert.proposal)
        .bind(&insert.request_no)
        .bind(&insert.idempotency_key)
        .bind(&insert.now)
        .bind(&insert.app_forms)
        .execute(&mut *transaction)
        .await;

        if let Err(error) = result {
            let _ = transaction.rollback().await;
            if let Some(existing) = self
                .find_bidder_claim(&insert.tenant_id, &insert.demand_id, &insert.bidder_user_id)
                .await?
            {
                let same_quote = quote_major_to_minor(&existing.quote)
                    .is_ok_and(|value| value == insert.quote_amount_minor.as_str());
                if same_quote && existing.proposal == insert.proposal {
                    if matches!(existing.status.as_str(), "submitted" | "shortlisted") {
                        return Ok(existing);
                    }
                    return Err(CommerceServiceError::conflict(
                        "your previous claim for this demand is already closed",
                    ));
                }
                return Err(CommerceServiceError::conflict(
                    "you already submitted a claim for this demand",
                ));
            }
            return Err(map_sqlx_store_error("insert demand claim", error));
        }

        let event = build_event(
            &insert.tenant_id,
            &insert.id,
            "claim.submitted",
            None,
            "submitted",
            DemandClaimActorType::Provider,
            Some(&insert.bidder_user_id),
            None,
            serde_json::json!({
                "quote": quote_minor_to_major(&insert.quote_amount_minor),
                "deliveryDays": insert.delivery_days,
                "demandId": insert.demand_id,
            }),
            Some(&insert.request_no),
            &insert.idempotency_key,
            &insert.now,
        );
        insert_event(&mut transaction, &event).await?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_store_error("commit demand claim", error))?;

        self.find_claim(&insert.tenant_id, &insert.id, &insert.bidder_user_id)
            .await?
            .ok_or_else(|| CommerceServiceError::storage("claim disappeared after insert"))
    }

    /// Load one claim visible to the viewer (provider, publisher, or operator).
    pub async fn find_claim(
        &self,
        tenant_id: &str,
        claim_id: &str,
        viewer_user_id: &str,
    ) -> Result<Option<DemandClaimView>, CommerceServiceError> {
        let sql = format!("{CLAIM_SELECT_WITH_TIMESTAMPS} WHERE tenant_id = $1 AND id = $2");
        let row = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(tenant_id)
            .bind(claim_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| map_sqlx_store_error("find demand claim", error))?;
        Ok(row.map(|row| map_claim_view(&row, viewer_user_id)))
    }

    async fn find_bidder_claim(
        &self,
        tenant_id: &str,
        demand_id: &str,
        bidder_user_id: &str,
    ) -> Result<Option<DemandClaimView>, CommerceServiceError> {
        let sql = format!(
            "{CLAIM_SELECT_WITH_TIMESTAMPS} WHERE tenant_id = $1 AND demand_id = $2 AND bidder_user_id = $3"
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(tenant_id)
            .bind(demand_id)
            .bind(bidder_user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| map_sqlx_store_error("find bidder demand claim", error))?;
        Ok(row.map(|row| map_claim_view(&row, bidder_user_id)))
    }

    /// List claims for one viewer, filtered by status, status group, or demand.
    pub async fn list_claims(
        &self,
        query: &DemandClaimListQuery,
    ) -> Result<(Vec<DemandClaimView>, i64), CommerceServiceError> {
        let role = role_binding(query.role);
        let statuses = status_binding(query.status, query.status_group);
        let page_size = query.page_size.clamp(1, 200);
        let page = query.page.max(1);
        let offset = (page - 1) * page_size;
        let list_sql = format!(
            r#"
            {CLAIM_SELECT_WITH_TIMESTAMPS}
            WHERE {VIEWER_PREDICATE}
              AND ($4::text IS NULL OR status = ANY(string_to_array($4, ',')))
              AND ($5::text IS NULL OR demand_id = $5)
            ORDER BY updated_at DESC, id DESC
            LIMIT $6 OFFSET $7
            "#
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(list_sql.as_str()))
            .bind(&query.tenant_id)
            .bind(role)
            .bind(&query.viewer_user_id)
            .bind(statuses.as_deref())
            .bind(query.demand_id.as_deref())
            .bind(page_size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| map_sqlx_store_error("list demand claims", error))?;
        let count_sql = format!(
            r#"
            SELECT COUNT(*) AS total_items
            FROM commerce_demand_claim
            WHERE {VIEWER_PREDICATE}
              AND ($4::text IS NULL OR status = ANY(string_to_array($4, ',')))
              AND ($5::text IS NULL OR demand_id = $5)
            "#
        );
        let total = sqlx::query(sqlx::AssertSqlSafe(count_sql.as_str()))
            .bind(&query.tenant_id)
            .bind(role)
            .bind(&query.viewer_user_id)
            .bind(statuses.as_deref())
            .bind(query.demand_id.as_deref())
            .fetch_one(&self.pool)
            .await
            .map_err(|error| map_sqlx_store_error("count demand claims", error))?
            .try_get::<i64, _>("total_items")
            .unwrap_or_default();
        Ok((
            rows.iter()
                .map(|row| map_claim_view(row, &query.viewer_user_id))
                .collect(),
            total,
        ))
    }

    /// Tab counters for the viewer's claims.
    pub async fn statistics(
        &self,
        tenant_id: &str,
        viewer_user_id: &str,
        role: DemandClaimViewerRole,
    ) -> Result<DemandClaimStatisticsView, CommerceServiceError> {
        let sql = format!(
            r#"
            SELECT status, COUNT(*) AS total_items
            FROM commerce_demand_claim
            WHERE {VIEWER_PREDICATE}
            GROUP BY status
            "#
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(tenant_id)
            .bind(role_binding(role))
            .bind(viewer_user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| map_sqlx_store_error("demand claim statistics", error))?;
        let mut statistics = DemandClaimStatisticsView::default();
        for row in &rows {
            let status = string_cell(row, "status");
            let count = row.try_get::<i64, _>("total_items").unwrap_or_default();
            match parse_status_or_default(&status).group() {
                DemandClaimStatusGroup::InProgress => statistics.in_progress += count,
                DemandClaimStatusGroup::Completed => statistics.completed += count,
                DemandClaimStatusGroup::Closed => statistics.closed += count,
            }
            statistics.total += count;
        }
        Ok(statistics)
    }

    /// The claim timeline, oldest first.
    pub async fn list_events(
        &self,
        tenant_id: &str,
        claim_id: &str,
        limit: i64,
    ) -> Result<Vec<DemandClaimEventView>, CommerceServiceError> {
        let rows = sqlx::query(
            r#"
            SELECT id, claim_id, event_type, from_status, to_status, actor_type, actor_id,
                   message, payload_json,
                   TO_CHAR(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS created_at
            FROM commerce_demand_claim_event
            WHERE tenant_id = $1 AND claim_id = $2
            ORDER BY created_at ASC, id ASC
            LIMIT $3
            "#,
        )
        .bind(tenant_id)
        .bind(claim_id)
        .bind(limit.clamp(1, 200))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_store_error("list demand claim events", error))?;
        Ok(rows.iter().map(map_event_view).collect())
    }

    /// Apply one guarded transition and append its timeline event atomically.
    /// Returns false when the claim is no longer in the expected state.
    pub async fn apply_transition(
        &self,
        patch: &DemandClaimTransitionPatch,
        actor_type: DemandClaimActorType,
        actor_id: &str,
        event_type: &str,
        message: Option<&str>,
        payload: serde_json::Value,
    ) -> Result<bool, CommerceServiceError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_store_error("begin claim transition", error))?;
        let milestone = patch.milestone.map(milestone_binding).unwrap_or("none");
        let attachments = patch
            .delivery_attachment_urls
            .as_ref()
            .map(|items| serde_json::to_string(items).unwrap_or_else(|_| "[]".to_owned()));
        let result = sqlx::query(
            r#"
            UPDATE commerce_demand_claim SET
                status = $3,
                reject_reason = COALESCE($4, reject_reason),
                withdraw_reason = COALESCE($5, withdraw_reason),
                cancel_reason = COALESCE($6, cancel_reason),
                delivery_note = COALESCE($7, delivery_note),
                delivery_attachment_urls = COALESCE($8, delivery_attachment_urls),
                submitted_at = CASE WHEN $9 = 'submitted' THEN $10::timestamptz ELSE submitted_at END,
                accepted_at = CASE WHEN $9 = 'accepted' THEN $10::timestamptz ELSE accepted_at END,
                started_at = CASE WHEN $9 = 'started' THEN $10::timestamptz ELSE started_at END,
                delivered_at = CASE WHEN $9 = 'delivered' THEN $10::timestamptz ELSE delivered_at END,
                completed_at = CASE WHEN $9 = 'completed' THEN $10::timestamptz ELSE completed_at END,
                closed_at = CASE WHEN $9 = 'closed' THEN $10::timestamptz ELSE closed_at END,
                updated_at = $10::timestamptz
            WHERE tenant_id = $1 AND id = $2 AND status = $11
            "#,
        )
        .bind(&patch.tenant_id)
        .bind(&patch.claim_id)
        .bind(&patch.next_status)
        .bind(patch.reject_reason.as_deref())
        .bind(patch.withdraw_reason.as_deref())
        .bind(patch.cancel_reason.as_deref())
        .bind(patch.delivery_note.as_deref())
        .bind(attachments.as_deref())
        .bind(milestone)
        .bind(&patch.now)
        .bind(&patch.expected_status)
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_store_error("update demand claim status", error))?;

        if result.rows_affected() == 0 {
            let _ = transaction.rollback().await;
            return Ok(false);
        }

        let event = build_event(
            &patch.tenant_id,
            &patch.claim_id,
            event_type,
            Some(&patch.expected_status),
            &patch.next_status,
            actor_type,
            Some(actor_id),
            message,
            payload,
            None,
            &format!("{}:{}", patch.claim_id, patch.next_status),
            &patch.now,
        );
        insert_event(&mut transaction, &event).await?;
        // Awarding one claim closes every still-open competing claim in the
        // same transaction, so losing providers see "未通过" instead of a
        // permanently pending claim that can no longer be awarded.
        if patch.milestone == Some(DemandClaimMilestone::Accepted) {
            close_competing_claims(&mut transaction, patch, actor_id).await?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_store_error("commit demand claim transition", error))?;
        Ok(true)
    }
}

fn role_binding(role: DemandClaimViewerRole) -> &'static str {
    match role {
        DemandClaimViewerRole::Provider => "provider",
        DemandClaimViewerRole::Publisher => "publisher",
        DemandClaimViewerRole::Operator => "operator",
    }
}

fn status_binding(
    status: Option<DemandClaimStatus>,
    group: Option<DemandClaimStatusGroup>,
) -> Option<String> {
    if let Some(status) = status {
        return Some(status.as_str().to_owned());
    }
    group.map(|group| group.statuses().join(","))
}

fn milestone_binding(milestone: DemandClaimMilestone) -> &'static str {
    match milestone {
        DemandClaimMilestone::Submitted => "submitted",
        DemandClaimMilestone::Accepted => "accepted",
        DemandClaimMilestone::Started => "started",
        DemandClaimMilestone::Delivered => "delivered",
        DemandClaimMilestone::Completed => "completed",
        DemandClaimMilestone::Closed => "closed",
    }
}

fn build_event(
    tenant_id: &str,
    claim_id: &str,
    event_type: &str,
    from_status: Option<&str>,
    to_status: &str,
    actor_type: DemandClaimActorType,
    actor_id: Option<&str>,
    message: Option<&str>,
    payload: serde_json::Value,
    request_id: Option<&str>,
    idempotency_key: &str,
    now: &str,
) -> DemandClaimEventInsert {
    let id = sdkwork_utils_rust::uuid();
    DemandClaimEventInsert {
        event_no: format!(
            "DCE{}",
            id.chars()
                .filter(|item| item.is_ascii_alphanumeric())
                .take(12)
                .collect::<String>()
                .to_ascii_uppercase()
        ),
        id,
        tenant_id: tenant_id.to_owned(),
        claim_id: claim_id.to_owned(),
        event_type: event_type.to_owned(),
        from_status: from_status.map(ToOwned::to_owned),
        to_status: to_status.to_owned(),
        actor_type: actor_type.as_str().to_owned(),
        actor_id: actor_id.map(ToOwned::to_owned),
        message: message.map(ToOwned::to_owned),
        payload_json: payload.to_string(),
        request_id: request_id.map(ToOwned::to_owned),
        idempotency_key: idempotency_key.to_owned(),
        now: now.to_owned(),
    }
}

async fn insert_event(
    transaction: &mut Transaction<'_, Postgres>,
    event: &DemandClaimEventInsert,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        INSERT INTO commerce_demand_claim_event (
            id, tenant_id, claim_id, event_no, event_type, from_status, to_status,
            actor_type, actor_id, message, payload_json, request_id, idempotency_key, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14::timestamptz)
        "#,
    )
    .bind(&event.id)
    .bind(&event.tenant_id)
    .bind(&event.claim_id)
    .bind(&event.event_no)
    .bind(&event.event_type)
    .bind(event.from_status.as_deref())
    .bind(&event.to_status)
    .bind(&event.actor_type)
    .bind(event.actor_id.as_deref())
    .bind(event.message.as_deref())
    .bind(&event.payload_json)
    .bind(event.request_id.as_deref())
    .bind(&event.idempotency_key)
    .bind(&event.now)
    .execute(&mut **transaction)
    .await
    .map_err(|error| map_sqlx_store_error("insert demand claim event", error))?;
    Ok(())
}

/// Reject every open claim on the same demand after one claim is awarded.
/// Runs inside the award transaction so no losing claim can drift back to an
/// awardable state.
async fn close_competing_claims(
    transaction: &mut Transaction<'_, Postgres>,
    patch: &DemandClaimTransitionPatch,
    actor_id: &str,
) -> Result<(), CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT id, status
        FROM commerce_demand_claim
        WHERE tenant_id = $1
          AND demand_id = $2
          AND id <> $3
          AND status IN ('submitted', 'shortlisted')
        FOR UPDATE
        "#,
    )
    .bind(&patch.tenant_id)
    .bind(&patch.demand_id)
    .bind(&patch.claim_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(|error| map_sqlx_store_error("list competing demand claims", error))?;

    for row in rows {
        let competing_id: String = row
            .try_get("id")
            .map_err(|error| map_sqlx_store_error("read competing demand claim id", error))?;
        let from_status: String = row
            .try_get("status")
            .map_err(|error| map_sqlx_store_error("read competing demand claim status", error))?;
        sqlx::query(
            r#"
            UPDATE commerce_demand_claim
            SET status = 'rejected',
                reject_reason = COALESCE(reject_reason, 'another claim was awarded'),
                closed_at = $4::timestamptz,
                updated_at = $4::timestamptz
            WHERE tenant_id = $1 AND id = $2 AND status = $3
            "#,
        )
        .bind(&patch.tenant_id)
        .bind(&competing_id)
        .bind(&from_status)
        .bind(&patch.now)
        .execute(&mut **transaction)
        .await
        .map_err(|error| map_sqlx_store_error("reject competing demand claim", error))?;

        let event = build_event(
            &patch.tenant_id,
            &competing_id,
            "claim.rejected",
            Some(&from_status),
            DemandClaimStatus::Rejected.as_str(),
            DemandClaimActorType::Publisher,
            Some(actor_id),
            Some("another claim was awarded"),
            serde_json::json!({
                "claimId": competing_id.clone(),
                "demandId": &patch.demand_id,
                "reason": "another claim was awarded"
            }),
            None,
            &format!("{competing_id}:competing-rejected"),
            &patch.now,
        );
        insert_event(transaction, &event).await?;
    }
    Ok(())
}

fn map_claim_view(row: &sqlx::postgres::PgRow, viewer_user_id: &str) -> DemandClaimView {
    let bidder_user_id = string_cell(row, "bidder_user_id");
    let publisher_user_id = string_cell(row, "publisher_user_id");
    let status = string_cell(row, "status");
    let status_group = parse_status_or_default(&status).group().as_str().to_owned();
    let role = if bidder_user_id == viewer_user_id {
        "provider"
    } else if publisher_user_id == viewer_user_id {
        "publisher"
    } else {
        "operator"
    };
    let app_form = optional_string_cell(row, "app_form");
    let mut app_forms = text_array_cell(row, "app_forms");
    if app_forms.is_empty() {
        if let Some(form) = app_form.clone() {
            app_forms.push(form);
        }
    }
    DemandClaimView {
        id: string_cell(row, "id"),
        tenant_id: string_cell(row, "tenant_id"),
        claim_no: string_cell(row, "claim_no"),
        demand_id: string_cell(row, "demand_id"),
        demand_title: string_cell(row, "demand_title"),
        demand_category: optional_string_cell(row, "demand_category"),
        demand_type: string_cell(row, "demand_type"),
        app_form,
        app_forms,
        budget_text: optional_string_cell(row, "budget_text"),
        budget_min: optional_string_cell(row, "budget_min"),
        budget_max: optional_string_cell(row, "budget_max"),
        bid_deadline: optional_string_cell(row, "bid_deadline"),
        publisher_user_id,
        publisher_company_name: optional_string_cell(row, "publisher_company_name"),
        bidder_user_id,
        bidder_company_id: optional_string_cell(row, "bidder_company_id"),
        quote: quote_minor_to_major(&string_cell(row, "quote_amount_minor")),
        quote_currency: string_cell(row, "quote_currency"),
        delivery_days: row
            .try_get::<Option<i32>, _>("delivery_days")
            .ok()
            .flatten(),
        proposal: string_cell(row, "proposal"),
        status,
        status_group,
        reject_reason: optional_string_cell(row, "reject_reason"),
        withdraw_reason: optional_string_cell(row, "withdraw_reason"),
        cancel_reason: optional_string_cell(row, "cancel_reason"),
        delivery_note: optional_string_cell(row, "delivery_note"),
        delivery_attachment_urls: parse_string_array(&string_cell(row, "delivery_attachment_urls")),
        order_id: optional_string_cell(row, "order_id"),
        order_no: optional_string_cell(row, "order_no"),
        role: role.to_owned(),
        submitted_at: string_cell(row, "submitted_at"),
        accepted_at: optional_string_cell(row, "accepted_at"),
        started_at: optional_string_cell(row, "started_at"),
        delivered_at: optional_string_cell(row, "delivered_at"),
        completed_at: optional_string_cell(row, "completed_at"),
        closed_at: optional_string_cell(row, "closed_at"),
        created_at: string_cell(row, "created_at"),
        updated_at: string_cell(row, "updated_at"),
    }
}

fn map_event_view(row: &sqlx::postgres::PgRow) -> DemandClaimEventView {
    DemandClaimEventView {
        id: string_cell(row, "id"),
        claim_id: string_cell(row, "claim_id"),
        event_type: string_cell(row, "event_type"),
        from_status: optional_string_cell(row, "from_status"),
        to_status: string_cell(row, "to_status"),
        actor_type: string_cell(row, "actor_type"),
        actor_id: optional_string_cell(row, "actor_id"),
        message: optional_string_cell(row, "message"),
        payload: serde_json::from_str(&string_cell(row, "payload_json"))
            .unwrap_or(serde_json::Value::Null),
        created_at: string_cell(row, "created_at"),
    }
}

fn parse_string_array(value: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(value).unwrap_or_default()
}

fn text_array_cell(row: &sqlx::postgres::PgRow, column: &str) -> Vec<String> {
    row.try_get::<Option<Vec<String>>, _>(column)
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{parse_string_array, role_binding, status_binding};
    use sdkwork_order_service::{DemandClaimStatus, DemandClaimStatusGroup, DemandClaimViewerRole};

    #[test]
    fn status_binding_expands_groups() {
        let statuses =
            status_binding(None, Some(DemandClaimStatusGroup::InProgress)).expect("group statuses");
        assert!(statuses.split(',').any(|item| item == "submitted"));
        assert!(statuses.split(',').any(|item| item == "delivered"));
        assert!(!statuses.split(',').any(|item| item == "completed"));
        assert_eq!(
            status_binding(Some(DemandClaimStatus::Completed), None),
            Some("completed".to_owned())
        );
    }

    #[test]
    fn role_binding_matches_the_wire_values() {
        assert_eq!(role_binding(DemandClaimViewerRole::Provider), "provider");
        assert_eq!(role_binding(DemandClaimViewerRole::Publisher), "publisher");
        assert_eq!(role_binding(DemandClaimViewerRole::Operator), "operator");
    }

    #[test]
    fn attachments_parse_into_strings() {
        assert_eq!(
            parse_string_array(r#"["a","b"]"#),
            vec!["a".to_owned(), "b".to_owned()]
        );
        assert!(parse_string_array("not-json").is_empty());
    }
}
