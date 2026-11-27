use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    AddOrderTagsCommand, CancelManagementOrderCommand, CloseManagementOrderCommand,
    OrderCancellationListQuery, OrderCancellationPage, OrderCancellationView,
    OrderClassificationView, OrderManagementDetailQuery, OrderManagementEventListQuery,
    OrderManagementEventPage, OrderManagementEventView, OrderManagementFacetView,
    OrderManagementListPage, OrderManagementListQuery, OrderOwnerDetail, OrderOwnerItem,
    OrderOwnerSummary, OrderPartnerSnapshot, OrderScene, OrderSceneFacetView, OrderTagFacetView,
    ReplaceOrderClassificationCommand, ORDER_TAG_MAX_COUNT,
};
use sqlx::{PgPool, Row};

use crate::money_amount::commerce_money_stored;
use crate::order_limits::MAX_ORDER_LINE_ITEMS;
use crate::postgres_order::PostgresCommerceOrderStore;

/// Upper bound on tag buckets returned by the facet aggregation.
///
/// A tag facet is a suggestion surface, not an exhaustive dump: capping it
/// keeps the response bounded for tenants with thousands of distinct labels,
/// and the UI always offers free-text entry for labels outside the top N.
const TAG_FACET_BUCKET_LIMIT: i32 = 100;

/// Admin order list.
///
/// The payment intent/attempt sides are `LATERAL ... LIMIT 1` picks instead
/// of plain joins: an order retried its payment N times owns N attempts, and
/// a plain join would duplicate the order row N times (inflating both the
/// page and `COUNT(*) OVER()`). Organization matching uses the platform
/// sentinel (`'0'`, DATABASE_SPEC §6.3) instead of `IS NULL` fuzziness so a
/// NULL-org payment row can never decorate another organization's order.
/// Time-range predicates cast the TEXT instant column explicitly
/// (DATABASE_SPEC §8.1.1); the free-text `q` pattern is pre-escaped so user
/// input cannot inject `%`/`_` wildcards.
const LIST_MANAGEMENT_ORDERS: &str = r#"
SELECT
    o.id AS order_id,
    o.order_no AS order_sn,
    o.status,
    o.subject,
    TO_CHAR(o.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS created_at,
    TO_CHAR(o.paid_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS pay_time,
    TO_CHAR(o.expired_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS expire_time,
    COALESCE(
        (
            SELECT CAST(b.payable_amount AS TEXT)
            FROM commerce_order_amount_breakdown b
            WHERE b.tenant_id = o.tenant_id
              AND b.order_id = o.id
              AND b.allocation_type = 'order_total'
            LIMIT 1
        ),
        '0'
    ) AS total_amount,
    COALESCE(
        (
            SELECT CAST(b.discount_amount AS TEXT)
            FROM commerce_order_amount_breakdown b
            WHERE b.tenant_id = o.tenant_id
              AND b.order_id = o.id
              AND b.allocation_type = 'order_total'
            LIMIT 1
        ),
        '0'
    ) AS discount_amount,
    COALESCE(
        (
            SELECT CAST(SUM(oi.quantity) AS BIGINT)
            FROM commerce_order_item oi
            WHERE oi.tenant_id = o.tenant_id
              AND oi.order_id = o.id
        ),
        1
    ) AS quantity,
    COALESCE(
        NULLIF(pa.payment_method, ''),
        NULLIF(pi.payment_method, '')
    ) AS payment_method,
    COALESCE(NULLIF(o.currency_code, ''), 'CNY') AS currency_code,
    o.partner_id,
    o.partner_snapshot_json,
    o.scene_code,
    o.tags,
    o.remark,
    COUNT(*) OVER() AS total_count
FROM commerce_order o
LEFT JOIN LATERAL (
    SELECT pi.payment_method
    FROM commerce_payment_intent pi
    WHERE pi.tenant_id = o.tenant_id
      AND pi.order_id = o.id
      AND COALESCE(pi.organization_id, '0') IN (o.organization_id, '0')
    ORDER BY pi.created_at DESC, pi.id DESC
    LIMIT 1
) pi ON TRUE
LEFT JOIN LATERAL (
    SELECT pa.payment_method
    FROM commerce_payment_attempt pa
    WHERE pa.tenant_id = o.tenant_id
      AND pa.order_id = o.id
      AND COALESCE(pa.organization_id, '0') IN (o.organization_id, '0')
    ORDER BY pa.created_at DESC, pa.id DESC
    LIMIT 1
) pa ON TRUE
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id = '0' AND $2 IS NULL))
  AND ($3 IS NULL OR o.status = $3)
  AND (
        $4 IS NULL
        OR o.order_no ILIKE $4
        OR o.subject ILIKE $4
        OR o.id ILIKE $4
      )
  AND ($5 IS NULL OR o.created_at >= $5::timestamptz)
  AND ($6 IS NULL OR o.created_at <= $6::timestamptz)
  /* facets:scene */
  /* facets:tags */
ORDER BY o.created_at DESC, o.id DESC
LIMIT $7 OFFSET $8
"#;

/// Counts the same filter set as [`LIST_MANAGEMENT_ORDERS`] without the page
/// projection: when the requested page is past the end, `COUNT(*) OVER()`
/// never materializes a row and the page total would read as zero.
const COUNT_MANAGEMENT_ORDERS: &str = r#"
SELECT COUNT(*) AS total_count
FROM commerce_order o
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id = '0' AND $2 IS NULL))
  AND ($3 IS NULL OR o.status = $3)
  AND (
        $4 IS NULL
        OR o.order_no ILIKE $4
        OR o.subject ILIKE $4
        OR o.id ILIKE $4
      )
  AND ($5 IS NULL OR o.created_at >= $5::timestamptz)
  AND ($6 IS NULL OR o.created_at <= $6::timestamptz)
  /* facets:scene */
  /* facets:tags */
"#;

/// Aggregates the scene facet for the filter bar.
///
/// Like the tag facet below it, this deliberately does **not** apply the scene
/// or tag predicate, so every bucket shows the count the operator would get by
/// adding that value to the current selection (faceted-search semantics). It
/// does honour `status` / `q` / `created_*`, keeping the numbers faithful to
/// the result set the operator is looking at.
const FACET_MANAGEMENT_ORDER_SCENES: &str = r#"
SELECT o.scene_code, COUNT(*) AS bucket_count
FROM commerce_order o
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id = '0' AND $2 IS NULL))
  AND ($3 IS NULL OR o.status = $3)
  AND (
        $4 IS NULL
        OR o.order_no ILIKE $4
        OR o.subject ILIKE $4
        OR o.id ILIKE $4
      )
  AND ($5 IS NULL OR o.created_at >= $5::timestamptz)
  AND ($6 IS NULL OR o.created_at <= $6::timestamptz)
GROUP BY o.scene_code
ORDER BY bucket_count DESC, o.scene_code ASC
"#;

/// Aggregates the tag facet for the filter bar (`jsonb_array_elements_text`
/// unnests each order's tag array so one order contributes one row per tag).
const FACET_MANAGEMENT_ORDER_TAGS: &str = r#"
SELECT tag.value AS tag, COUNT(*) AS bucket_count
FROM commerce_order o
CROSS JOIN LATERAL jsonb_array_elements_text(o.tags) AS tag(value)
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id = '0' AND $2 IS NULL))
  AND ($3 IS NULL OR o.status = $3)
  AND (
        $4 IS NULL
        OR o.order_no ILIKE $4
        OR o.subject ILIKE $4
        OR o.id ILIKE $4
      )
  AND ($5 IS NULL OR o.created_at >= $5::timestamptz)
  AND ($6 IS NULL OR o.created_at <= $6::timestamptz)
GROUP BY tag.value
ORDER BY bucket_count DESC, tag.value ASC
LIMIT $7
"#;

/// Substitutes the facet predicates into a management list/count statement.
///
/// The fragments come from [`crate::order_facets`] and contain only `$N`
/// placeholders, so this never interpolates user text.
fn render_management_sql(
    template: &str,
    scene_predicate: &str,
    tags_predicate: &str,
) -> String {
    template
        .replace("  /* facets:scene */\n", &facet_line(scene_predicate))
        .replace("  /* facets:tags */\n", &facet_line(tags_predicate))
}

fn facet_line(predicate: &str) -> String {
    let trimmed = predicate.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("  {trimmed}\n")
    }
}

/// Binds the facet parameters (scene, then every tag literal) in the exact
/// order their placeholders appear in the rendered statement.
///
/// Kept as a macro-free helper on [`FacetBinding`] because sqlx's `Query`
/// builder is not object-safe for a generic "bind these" function; the three
/// call sites all need the identical ordering rule, so the rule lives once.
struct FacetBinding<'a> {
    scene: Option<&'a str>,
    tag_literals: Vec<String>,
}

impl<'a> FacetBinding<'a> {
    fn for_query(query: &'a OrderManagementListQuery) -> Self {
        Self {
            scene: query.scene.map(|scene| scene.as_str()),
            tag_literals: crate::order_facets::json_tag_literals(&query.tags),
        }
    }

    /// Base index for the first tag placeholder, given six fixed parameters.
    fn first_tag_parameter(&self) -> usize {
        6 + 1 + usize::from(self.scene.is_some())
    }
}

/// Escapes LIKE wildcards in user-provided free text so `q` cannot widen the
/// search into a full scan (`%`/`_`/`\` are the LIKE metacharacters).
fn escape_ilike_pattern(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    for character in value.chars() {
        if matches!(character, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

impl PostgresCommerceOrderStore {
    pub async fn list_management_orders(
        &self,
        query: OrderManagementListQuery,
    ) -> Result<OrderManagementListPage, CommerceServiceError> {
        let search = query
            .q
            .as_deref()
            .map(|value| format!("%{}%", escape_ilike_pattern(value.trim())));
        let binding = FacetBinding::for_query(&query);
        let scene_predicate = crate::order_facets::order_scene_predicate(query.scene, 7);
        let tags_predicate = crate::order_facets::order_tags_predicate(
            &query.tags,
            query.tag_match,
            binding.first_tag_parameter(),
        );
        let list_sql = render_management_sql(
            LIST_MANAGEMENT_ORDERS,
            scene_predicate.as_deref().unwrap_or_default(),
            tags_predicate.as_deref().unwrap_or_default(),
        );
        // Page placeholders follow the facets, so their positions depend on how
        // many facet values were bound. The template fixes them at $7/$8.
        let facet_count = usize::from(binding.scene.is_some()) + binding.tag_literals.len();
        let list_sql = list_sql.replace(
            "LIMIT $7 OFFSET $8",
            &format!("LIMIT ${} OFFSET ${}", 7 + facet_count, 8 + facet_count),
        );

        let mut list_statement = sqlx::query(sqlx::AssertSqlSafe(list_sql.as_str()))
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.status.as_deref())
            .bind(search.as_deref())
            .bind(query.created_from.as_deref())
            .bind(query.created_to.as_deref());
        if let Some(scene) = binding.scene {
            list_statement = list_statement.bind(scene);
        }
        for literal in &binding.tag_literals {
            list_statement = list_statement.bind(literal);
        }
        let rows = list_statement
            .bind(query.limit())
            .bind(query.offset())
            .fetch_all(self.pool())
            .await
            .map_err(|error| store_error("failed to list management orders", error))?;

        let total = if rows.is_empty() {
            let count_sql = render_management_sql(
                COUNT_MANAGEMENT_ORDERS,
                scene_predicate.as_deref().unwrap_or_default(),
                tags_predicate.as_deref().unwrap_or_default(),
            );
            let mut count_statement = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(count_sql.as_str()))
                .bind(&query.tenant_id)
                .bind(query.organization_id.as_deref())
                .bind(query.status.as_deref())
                .bind(search.as_deref())
                .bind(query.created_from.as_deref())
                .bind(query.created_to.as_deref());
            if let Some(scene) = binding.scene {
                count_statement = count_statement.bind(scene);
            }
            for literal in &binding.tag_literals {
                count_statement = count_statement.bind(literal);
            }
            count_statement
                .fetch_one(self.pool())
                .await
                .map_err(|error| store_error("failed to count management orders", error))?
        } else {
            rows.first()
                .and_then(|row| row.try_get::<i64, _>("total_count").ok())
                .unwrap_or(0)
        };
        let items = rows
            .iter()
            .map(map_management_summary_row)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(OrderManagementListPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    /// Aggregates the scene and tag facets for a management list scope.
    ///
    /// The tag/scene predicates on `query` are intentionally ignored (see
    /// [`FACET_MANAGEMENT_ORDER_SCENES`]); only `status`, `q`, and the created
    /// range narrow the buckets.
    pub async fn management_order_facets(
        &self,
        query: &OrderManagementListQuery,
    ) -> Result<OrderManagementFacetView, CommerceServiceError> {
        let search = query
            .q
            .as_deref()
            .map(|value| format!("%{}%", escape_ilike_pattern(value.trim())));

        let scene_rows = sqlx::query(FACET_MANAGEMENT_ORDER_SCENES)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.status.as_deref())
            .bind(search.as_deref())
            .bind(query.created_from.as_deref())
            .bind(query.created_to.as_deref())
            .fetch_all(self.pool())
            .await
            .map_err(|error| store_error("failed to aggregate order scene facets", error))?;

        let mut scenes: Vec<OrderSceneFacetView> = Vec::with_capacity(scene_rows.len());
        for row in &scene_rows {
            // Unknown stored scenes collapse into `Default` so the bucket list
            // stays within the closed enum the UI renders.
            let scene = optional_string_cell(row, "scene_code")
                .as_deref()
                .and_then(|value| OrderScene::parse(value).ok())
                .unwrap_or_default();
            let count = row.try_get::<i64, _>("bucket_count").unwrap_or(0);
            match scenes.iter_mut().find(|bucket| bucket.scene == scene) {
                Some(bucket) => bucket.count += count,
                None => scenes.push(OrderSceneFacetView { scene, count }),
            }
        }
        let total_orders = scenes.iter().map(|bucket| bucket.count).sum();

        let tag_rows = sqlx::query(FACET_MANAGEMENT_ORDER_TAGS)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.status.as_deref())
            .bind(search.as_deref())
            .bind(query.created_from.as_deref())
            .bind(query.created_to.as_deref())
            .bind(TAG_FACET_BUCKET_LIMIT)
            .fetch_all(self.pool())
            .await
            .map_err(|error| store_error("failed to aggregate order tag facets", error))?;

        let tags = tag_rows
            .iter()
            .filter_map(|row| {
                let tag = string_cell(row, "tag");
                if tag.is_empty() {
                    return None;
                }
                Some(OrderTagFacetView {
                    tag,
                    count: row.try_get::<i64, _>("bucket_count").unwrap_or(0),
                })
            })
            .collect();

        Ok(OrderManagementFacetView {
            scenes,
            tags,
            total_orders,
        })
    }

    /// Adds tags to one order, returning the resulting classification.
    pub async fn add_management_order_tags(
        &self,
        command: AddOrderTagsCommand,
    ) -> Result<OrderClassificationView, CommerceServiceError> {
        let added: serde_json::Value = serde_json::Value::Array(
            command
                .tags
                .iter()
                .map(|tag| serde_json::Value::String(tag.as_str().to_owned()))
                .collect(),
        );
        let row = sqlx::query(
            r#"
            UPDATE commerce_order
            SET tags = (
                    SELECT COALESCE(jsonb_agg(DISTINCT value ORDER BY value), '[]'::jsonb)
                    FROM jsonb_array_elements_text(o.tags || CAST($3 AS jsonb)) AS value
                ),
                remark = COALESCE(CAST($4 AS TEXT), remark),
                updated_at = CAST($5 AS TIMESTAMPTZ)
            FROM commerce_order o
            WHERE commerce_order.tenant_id = CAST($1 AS TEXT)
              AND ((commerce_order.organization_id = CAST($2 AS TEXT))
                   OR (commerce_order.organization_id = '0' AND $2 IS NULL))
              AND commerce_order.id = CAST($6 AS TEXT)
              AND o.tenant_id = commerce_order.tenant_id
              AND o.id = commerce_order.id
              AND jsonb_array_length(commerce_order.tags || CAST($3 AS jsonb))
                  <= CAST($7 AS INTEGER)
            RETURNING
                commerce_order.id AS order_id,
                commerce_order.scene_code,
                commerce_order.tags,
                commerce_order.remark,
                TO_CHAR(commerce_order.updated_at AT TIME ZONE 'UTC',
                        'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS updated_at
            "#,
        )
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(serde_json::to_string(&added).unwrap_or_else(|_| "[]".to_owned()))
        .bind(command.remark.as_deref())
        .bind(crate::store_clock::now_canonical())
        .bind(&command.order_id)
        .bind(ORDER_TAG_MAX_COUNT as i32)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to add management order tags", error))?;

        map_classification_row(row, &command.order_id)
    }

    /// Replaces the whole tag set and (optionally) the scene of one order.
    ///
    /// This is the idempotent write path: submitting the same classification
    /// twice leaves the same row. Tag *removal* is expressed by submitting the
    /// resulting set here rather than through a separate delete command, so
    /// there is exactly one way the classification can change.
    pub async fn replace_management_order_classification(
        &self,
        command: ReplaceOrderClassificationCommand,
    ) -> Result<OrderClassificationView, CommerceServiceError> {
        let tags: Vec<String> = command
            .tags
            .iter()
            .map(|tag| tag.as_str().to_owned())
            .collect();
        let row = sqlx::query(
            r#"
            UPDATE commerce_order
            SET tags = CAST($3 AS jsonb),
                scene_code = COALESCE(CAST($4 AS TEXT), scene_code),
                remark = COALESCE(CAST($5 AS TEXT), remark),
                updated_at = CAST($6 AS TIMESTAMPTZ)
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT))
                   OR (organization_id = '0' AND $2 IS NULL))
              AND id = CAST($7 AS TEXT)
            RETURNING
                id AS order_id,
                scene_code,
                tags,
                remark,
                TO_CHAR(updated_at AT TIME ZONE 'UTC',
                        'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS updated_at
            "#,
        )
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(
            serde_json::to_string(&tags)
                .map_err(|error| store_error("failed to encode order tags", error))?,
        )
        .bind(command.scene.map(|scene| scene.as_str()))
        .bind(command.remark.as_deref())
        .bind(crate::store_clock::now_canonical())
        .bind(&command.order_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to replace order classification", error))?;

        map_classification_row(row, &command.order_id)
    }

    /// Refund safety bounds for an order: payment state, payable amount, and
    /// the cumulative amount of in-flight refunds. The over-refund check is
    /// evaluated in SQL so amount arithmetic never happens in application code.
    pub async fn retrieve_management_order(
        &self,
        query: OrderManagementDetailQuery,
    ) -> Result<Option<OrderOwnerDetail>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT
                o.id AS order_id,
                o.order_no AS order_sn,
                o.status,
                o.payment_status,
                o.subject,
                TO_CHAR(o.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS created_at,
                TO_CHAR(o.paid_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS pay_time,
                TO_CHAR(o.expired_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS expire_time,
                COALESCE(
                    (
                        SELECT CAST(b.payable_amount AS TEXT)
                        FROM commerce_order_amount_breakdown b
                        WHERE b.tenant_id = o.tenant_id
                          AND b.order_id = o.id
                          AND b.allocation_type = 'order_total'
                        LIMIT 1
                    ),
                    '0'
                ) AS total_amount,
                COALESCE(
                    (
                        SELECT CAST(b.discount_amount AS TEXT)
                        FROM commerce_order_amount_breakdown b
                        WHERE b.tenant_id = o.tenant_id
                          AND b.order_id = o.id
                          AND b.allocation_type = 'order_total'
                        LIMIT 1
                    ),
                    '0'
                ) AS discount_amount,
                COALESCE(
                    (
                        SELECT CAST(SUM(oi.quantity) AS BIGINT)
                        FROM commerce_order_item oi
                        WHERE oi.tenant_id = o.tenant_id
                          AND oi.order_id = o.id
                    ),
                    1
                ) AS quantity,
                COALESCE(
                    NULLIF(pa.payment_method, ''),
                    NULLIF(pi.payment_method, '')
                ) AS payment_method,
                COALESCE(NULLIF(o.currency_code, ''), 'CNY') AS currency_code,
                o.partner_id,
                o.partner_snapshot_json,
                o.scene_code,
                o.tags,
                o.remark,
                COALESCE(NULLIF(pa.out_trade_no, ''), NULLIF(o.order_no, '')) AS out_trade_no,
                CAST(pa.id AS TEXT) AS transaction_id
            FROM commerce_order o
            LEFT JOIN commerce_payment_intent pi
                ON pi.tenant_id = o.tenant_id
               AND (pi.organization_id = o.organization_id)
               AND pi.owner_user_id = o.owner_user_id
               AND pi.order_id = o.id
            LEFT JOIN commerce_payment_attempt pa
                ON pa.tenant_id = o.tenant_id
               AND (pa.organization_id = o.organization_id)
               AND pa.owner_user_id = o.owner_user_id
               AND pa.order_id = o.id
            WHERE o.tenant_id = CAST($1 AS TEXT)
              AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id = '0' AND $2 IS NULL))
              AND o.id = CAST($3 AS TEXT)
            "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(&query.order_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to retrieve management order", error))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let summary = map_management_summary_row(&row)?;
        let items =
            load_management_order_items(self.pool(), &query.tenant_id, &query.order_id).await?;
        Ok(Some(OrderOwnerDetail {
            summary,
            payment_status: optional_string_cell(&row, "payment_status"),
            items,
            out_trade_no: optional_string_cell(&row, "out_trade_no"),
            transaction_id: optional_string_cell(&row, "transaction_id"),
        }))
    }

    pub async fn resolve_management_order_owner_user_id(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: &str,
    ) -> Result<Option<String>, CommerceServiceError> {
        sqlx::query_scalar(
            r#"
            SELECT owner_user_id
            FROM commerce_order
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id = '0' AND $2 IS NULL))
              AND id = CAST($3 AS TEXT)
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(order_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to resolve management order owner", error))
    }

    pub async fn cancel_management_order(
        &self,
        command: CancelManagementOrderCommand,
    ) -> Result<(), CommerceServiceError> {
        use crate::order_lifecycle::{
            insert_order_cancellation_postgres, insert_order_event_postgres,
            order_cancel_idempotency_key, OrderLifecycleAuditInput,
        };

        let now = current_command_timestamp();
        let idempotency_key = order_cancel_idempotency_key(&command.order_id);
        let request_no = format!("admin-cancel-{}", command.order_id);

        let mut tx = self.pool().begin().await.map_err(|error| {
            store_error("failed to begin cancel management order transaction", error)
        })?;

        let from_status = sqlx::query_scalar::<_, String>(
            r#"
            SELECT status
            FROM commerce_order
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id = '0' AND $2 IS NULL))
              AND id = CAST($3 AS TEXT)
            "#,
        )
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to load order status before cancel", error))?;

        let Some(from_status) = from_status else {
            tx.rollback().await.map_err(|error| {
                store_error(
                    "failed to rollback cancel management order transaction",
                    error,
                )
            })?;
            return Err(CommerceServiceError::not_found("order was not found"));
        };

        if from_status.eq_ignore_ascii_case("cancelled") {
            tx.rollback().await.map_err(|error| {
                store_error(
                    "failed to rollback cancel management order transaction",
                    error,
                )
            })?;
            return Ok(());
        }

        let result = sqlx::query(
            r#"
            UPDATE commerce_order
            SET status = 'cancelled',
                payment_status = 'closed',
                cancelled_at = CAST($1 AS TIMESTAMPTZ),
                updated_at = CAST($2 AS TIMESTAMPTZ)
            WHERE tenant_id = CAST($3 AS TEXT)
              AND ((organization_id = CAST($4 AS TEXT)) OR (organization_id = '0' AND $4 IS NULL))
              AND id = CAST($5 AS TEXT)
              AND LOWER(COALESCE(status, '')) IN ('draft', 'pending', 'pending_payment', 'unpaid')
            "#,
        )
        .bind(&now)
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.order_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to cancel management order", error))?;

        if result.rows_affected() == 0 {
            let current_status = sqlx::query_scalar::<_, String>(
                r#"
                SELECT status
                FROM commerce_order
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id = '0' AND $2 IS NULL))
                  AND id = CAST($3 AS TEXT)
                "#,
            )
            .bind(&command.tenant_id)
            .bind(command.organization_id.as_deref())
            .bind(&command.order_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| store_error("failed to reload order status after cancel", error))?;

            tx.rollback().await.map_err(|error| {
                store_error(
                    "failed to rollback cancel management order transaction",
                    error,
                )
            })?;

            if current_status
                .as_deref()
                .is_some_and(|status| status.eq_ignore_ascii_case("cancelled"))
            {
                return Ok(());
            }
            return Err(CommerceServiceError::conflict(
                "order is not cancellable or was not found",
            ));
        }

        let audit = OrderLifecycleAuditInput {
            tenant_id: command.tenant_id.clone(),
            organization_id: command.organization_id.clone(),
            order_id: command.order_id.clone(),
            event_type: "cancelled",
            from_status,
            to_status: "cancelled",
            actor_type: "admin",
            actor_id: None,
            reason_code: command
                .cancel_type
                .clone()
                .or_else(|| Some("admin_cancel".to_owned())),
            reason_message: command.cancel_reason.clone(),
            request_no,
            idempotency_key,
            now: now.clone(),
        };
        insert_order_event_postgres(&mut tx, &audit).await?;
        insert_order_cancellation_postgres(&mut tx, &audit).await?;

        tx.commit().await.map_err(|error| {
            store_error(
                "failed to commit cancel management order transaction",
                error,
            )
        })?;
        Ok(())
    }

    pub async fn close_management_order(
        &self,
        command: CloseManagementOrderCommand,
    ) -> Result<(), CommerceServiceError> {
        use crate::order_lifecycle::{
            insert_order_event_postgres, order_close_idempotency_key, OrderLifecycleAuditInput,
        };

        let now = current_command_timestamp();
        let idempotency_key = order_close_idempotency_key(&command.order_id);
        let request_no = format!("admin-close-{}", command.order_id);

        let mut tx = self.pool().begin().await.map_err(|error| {
            store_error("failed to begin close management order transaction", error)
        })?;

        let from_status = sqlx::query_scalar::<_, String>(
            r#"
            SELECT status
            FROM commerce_order
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id = '0' AND $2 IS NULL))
              AND id = CAST($3 AS TEXT)
            "#,
        )
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to load order status before close", error))?;

        let Some(from_status) = from_status else {
            tx.rollback().await.map_err(|error| {
                store_error(
                    "failed to rollback close management order transaction",
                    error,
                )
            })?;
            return Err(CommerceServiceError::not_found("order was not found"));
        };

        if from_status.eq_ignore_ascii_case("closed") {
            tx.rollback().await.map_err(|error| {
                store_error(
                    "failed to rollback close management order transaction",
                    error,
                )
            })?;
            return Ok(());
        }

        let result = sqlx::query(
            r#"
            UPDATE commerce_order
            SET status = 'closed',
                updated_at = CAST($1 AS TIMESTAMPTZ)
            WHERE tenant_id = CAST($2 AS TEXT)
              AND ((organization_id = CAST($3 AS TEXT)) OR (organization_id = '0' AND $3 IS NULL))
              AND id = CAST($4 AS TEXT)
              AND LOWER(COALESCE(status, '')) NOT IN ('cancelled', 'closed')
            "#,
        )
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.order_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to close management order", error))?;

        if result.rows_affected() == 0 {
            let current_status = sqlx::query_scalar::<_, String>(
                r#"
                SELECT status
                FROM commerce_order
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id = '0' AND $2 IS NULL))
                  AND id = CAST($3 AS TEXT)
                "#,
            )
            .bind(&command.tenant_id)
            .bind(command.organization_id.as_deref())
            .bind(&command.order_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| store_error("failed to reload order status after close", error))?;

            tx.rollback().await.map_err(|error| {
                store_error(
                    "failed to rollback close management order transaction",
                    error,
                )
            })?;

            if current_status
                .as_deref()
                .is_some_and(|status| status.eq_ignore_ascii_case("closed"))
            {
                return Ok(());
            }
            return Err(CommerceServiceError::conflict(
                "order is not closable or was not found",
            ));
        }

        let audit = OrderLifecycleAuditInput {
            tenant_id: command.tenant_id.clone(),
            organization_id: command.organization_id.clone(),
            order_id: command.order_id.clone(),
            event_type: "closed",
            from_status,
            to_status: "closed",
            actor_type: "admin",
            actor_id: None,
            reason_code: command
                .close_type
                .clone()
                .or_else(|| Some("admin_close".to_owned())),
            reason_message: command.close_reason.clone(),
            request_no,
            idempotency_key,
            now: now.clone(),
        };
        insert_order_event_postgres(&mut tx, &audit).await?;

        tx.commit().await.map_err(|error| {
            store_error("failed to commit close management order transaction", error)
        })?;
        Ok(())
    }

    pub async fn list_management_order_events(
        &self,
        query: OrderManagementEventListQuery,
    ) -> Result<OrderManagementEventPage, CommerceServiceError> {
        let rows = sqlx::query(
            r#"
            SELECT id, event_type, from_status, to_status, actor_type, actor_id, message, created_at,
                   COUNT(*) OVER() AS total_count
            FROM commerce_order_event
            WHERE tenant_id = CAST($1 AS TEXT)
              AND order_id = CAST($2 AS TEXT)
            ORDER BY created_at DESC, id DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(&query.tenant_id)
        .bind(&query.order_id)
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to list management order events", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
        let items = rows
            .iter()
            .map(|row| OrderManagementEventView {
                id: string_cell(row, "id"),
                event_type: string_cell(row, "event_type"),
                from_status: optional_string_cell(row, "from_status"),
                to_status: string_cell(row, "to_status"),
                actor_type: string_cell(row, "actor_type"),
                actor_id: optional_string_cell(row, "actor_id"),
                message: optional_string_cell(row, "message"),
                created_at: string_cell(row, "created_at"),
            })
            .collect();

        Ok(OrderManagementEventPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn list_order_cancellations(
        &self,
        query: OrderCancellationListQuery,
    ) -> Result<OrderCancellationPage, CommerceServiceError> {
        let rows = sqlx::query(
            r#"
            SELECT id, order_id, status, reason_code, reason_message, created_at,
                   COUNT(*) OVER() AS total_count
            FROM commerce_order_cancellation
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ($2 IS NULL OR status = $2)
            ORDER BY created_at DESC, id DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(&query.tenant_id)
        .bind(query.status.as_deref())
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to list order cancellations", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
        let items = rows
            .iter()
            .map(|row| OrderCancellationView {
                id: string_cell(row, "id"),
                order_id: string_cell(row, "order_id"),
                status: string_cell(row, "status"),
                reason_code: string_cell(row, "reason_code"),
                reason_message: optional_string_cell(row, "reason_message"),
                created_at: string_cell(row, "created_at"),
            })
            .collect();

        Ok(OrderCancellationPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }
}

async fn load_management_order_items(
    pool: &PgPool,
    tenant_id: &str,
    order_id: &str,
) -> Result<Vec<OrderOwnerItem>, CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT id, title AS product_name, quantity, CAST(unit_price_amount AS TEXT) AS unit_price_amount, CAST(total_amount AS TEXT) AS total_amount
        FROM commerce_order_item
        WHERE tenant_id = CAST($1 AS TEXT)
          AND order_id = CAST($2 AS TEXT)
        ORDER BY created_at ASC, id ASC
        LIMIT $3
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind(MAX_ORDER_LINE_ITEMS)
    .fetch_all(pool)
    .await
    .map_err(|error| store_error("failed to list management order items", error))?;

    rows.iter()
        .map(|row| {
            Ok(OrderOwnerItem {
                id: string_cell(row, "id"),
                product_name: string_cell(row, "product_name"),
                quantity: row.try_get::<i64, _>("quantity").unwrap_or(1),
                unit_price: commerce_money_stored(
                    &string_cell(row, "unit_price_amount"),
                    "unit_price_amount",
                    order_id,
                )?,
                total_amount: commerce_money_stored(
                    &string_cell(row, "total_amount"),
                    "total_amount",
                    order_id,
                )?,
            })
        })
        .collect()
}

fn map_management_summary_row(
    row: &sqlx::postgres::PgRow,
) -> Result<OrderOwnerSummary, CommerceServiceError> {
    let order_id = string_cell(row, "order_id");
    // Stored money columns may carry legacy major-unit decimals; never let a
    // single legacy row fail the whole management order list.
    let total_amount =
        commerce_money_stored(&string_cell(row, "total_amount"), "total_amount", &order_id)?;
    let discount_amount = commerce_money_stored(
        &string_cell(row, "discount_amount"),
        "discount_amount",
        &order_id,
    )?;
    Ok(OrderOwnerSummary {
        order_id,
        order_sn: string_cell(row, "order_sn"),
        status: string_cell(row, "status"),
        subject: string_cell(row, "subject"),
        total_amount,
        paid_amount: None,
        discount_amount: Some(discount_amount),
        currency_code: string_cell(row, "currency_code"),
        quantity: row.try_get::<i64, _>("quantity").unwrap_or(1),
        created_at: string_cell(row, "created_at"),
        pay_time: optional_string_cell(row, "pay_time"),
        expire_time: optional_string_cell(row, "expire_time"),
        payment_method: optional_string_cell(row, "payment_method"),
        points: None,
        partner: optional_string_cell(row, "partner_snapshot_json")
            .and_then(|json| serde_json::from_str::<OrderPartnerSnapshot>(&json).ok())
            .or_else(|| {
                optional_string_cell(row, "partner_id").map(|partner_id| OrderPartnerSnapshot {
                    partner_id,
                    name: String::new(),
                    level_no: String::new(),
                    status: String::new(),
                })
            }),
        scene: optional_string_cell(row, "scene_code")
            .as_deref()
            .and_then(|value| OrderScene::parse(value).ok())
            .unwrap_or_default(),
        tags: json_string_array_cell(row, "tags"),
        remark: optional_string_cell(row, "remark"),
    })
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    row.try_get::<String, _>(column).unwrap_or_default()
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column)
        .ok()
        .flatten()
        .filter(|value| !value.is_empty())
}

fn store_error(message: &str, error: impl std::fmt::Display) -> CommerceServiceError {
    crate::sql_store_error::map_sql_store_error(message, error)
}

/// Maps a `RETURNING` classification row, reporting a missing order as
/// `not_found` so the admin API can answer 404 rather than 500.
fn map_classification_row(
    row: Option<sqlx::postgres::PgRow>,
    order_id: &str,
) -> Result<OrderClassificationView, CommerceServiceError> {
    let Some(row) = row else {
        return Err(CommerceServiceError::not_found(format!(
            "order {order_id} was not found"
        )));
    };
    let scene = optional_string_cell(&row, "scene_code")
        .as_deref()
        .and_then(|value| OrderScene::parse(value).ok())
        .unwrap_or_default();
    Ok(OrderClassificationView {
        order_id: string_cell(&row, "order_id"),
        scene,
        tags: json_string_array_cell(&row, "tags"),
        remark: optional_string_cell(&row, "remark"),
        updated_at: string_cell(&row, "updated_at"),
    })
}

/// Reads a JSONB text array into `Vec<String>`, dropping non-string elements.
fn json_string_array_cell(row: &sqlx::postgres::PgRow, column: &str) -> Vec<String> {
    let raw: Option<serde_json::Value> = row.try_get(column).ok().flatten();
    match raw {
        Some(serde_json::Value::Array(values)) => values
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

fn current_command_timestamp() -> String {
    crate::store_clock::now_canonical()
}
