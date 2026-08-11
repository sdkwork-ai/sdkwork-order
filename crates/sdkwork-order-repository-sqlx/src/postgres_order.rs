use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_order_service::{
    stable_checkout_order_subject, stable_order_settlement_subject, CancelOwnerOrderCommand,
    CreateOwnerOrderCommand, CreateOwnerOrderOutcome, NoopOrderPartnerRelationPort,
    OrderOwnerDetail, OrderOwnerDetailQuery, OrderOwnerEventListQuery, OrderOwnerEventPage,
    OrderOwnerEventView, OrderOwnerItem, OrderOwnerListPage, OrderOwnerListQuery,
    OrderOwnerPaymentStatus, OrderOwnerStatistics, OrderOwnerSummary, OrderPartnerRelationPort,
    OrderPartnerSnapshot,
};
use sdkwork_payment_service::{parse_scene_codes_csv, PaymentMethodItem, PaymentMethodListQuery};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::money_amount::{
    commerce_money, commerce_money_stored, multiply_money_amount, normalize_money_amount,
};
use crate::order_limits::MAX_ORDER_LINE_ITEMS;
use crate::order_settlement_context::membership_purchase_snapshot;
use crate::read_model::{
    empty_rows_when_read_model_is_missing, none_when_read_model_is_missing,
    read_model_table_is_missing,
};

/// Platform orders persist the sentinel organization scope (`"0"`) so that
/// personal-login (no-org) sessions never write NULL into the NOT NULL
/// `organization_id` column (DATABASE_SPEC DB090, same convention as
/// recharge.rs / postgres_membership_order.rs).
const PLATFORM_ORGANIZATION_SCOPE_SENTINEL: &str = "0";

fn normalize_organization_scope(organization_id: Option<&str>) -> String {
    organization_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(PLATFORM_ORGANIZATION_SCOPE_SENTINEL)
        .to_owned()
}

const LIST_OWNER_ORDERS: &str = r#"
SELECT
    o.id AS order_id,
    o.order_no AS order_sn,
    o.status,
    o.payment_status,
    o.subject,
    o.created_at,
    o.paid_at AS pay_time,
    o.expired_at AS expire_time,
    COALESCE(
        (
            SELECT b.payable_amount
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
            SELECT b.discount_amount
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
            SELECT SUM(oi.quantity)
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
    COALESCE(
        (
            SELECT CAST(COALESCE(NULLIF(oi.sku_snapshot_json::jsonb ->> 'points', ''), '0') AS BIGINT)
            FROM commerce_order_item oi
            WHERE oi.tenant_id = o.tenant_id
              AND oi.order_id = o.id
            LIMIT 1
        ),
        CAST(COALESCE(NULLIF(pa.callback_payload::jsonb ->> 'points', ''), '0') AS BIGINT),
        0
    ) AS recharge_points,
    o.partner_id,
    o.partner_snapshot_json,
    o.partner_id,
    o.partner_snapshot_json,
    COUNT(*) OVER() AS total_count
FROM commerce_order o
LEFT JOIN commerce_payment_intent pi
    ON pi.tenant_id = o.tenant_id
   AND (pi.organization_id IS NULL OR o.organization_id IS NULL OR pi.organization_id = o.organization_id)
   AND pi.owner_user_id = o.owner_user_id
   AND pi.order_id = o.id
LEFT JOIN commerce_payment_attempt pa
    ON pa.tenant_id = o.tenant_id
   AND (pa.organization_id IS NULL OR o.organization_id IS NULL OR pa.organization_id = o.organization_id)
   AND pa.owner_user_id = o.owner_user_id
   AND pa.order_id = o.id
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id IS NULL AND $2 IS NULL) OR (o.organization_id = '0' AND $2 IS NULL))
  AND o.owner_user_id = CAST($3 AS TEXT)
  AND ($4 IS NULL OR o.status = $4)
  AND ($5 IS NULL OR o.subject = $5)
ORDER BY o.created_at DESC, o.id DESC
LIMIT $6 OFFSET $7
"#;

const LIST_OWNER_ORDER_EVENTS: &str = r#"
SELECT
    e.id AS event_id,
    e.order_id,
    e.event_type,
    e.from_status,
    e.to_status,
    e.actor_type,
    e.actor_id,
    e.message,
    e.created_at,
    COUNT(*) OVER() AS total_count
FROM commerce_order_event e
WHERE e.tenant_id = CAST($1 AS TEXT)
  AND e.order_id = CAST($2 AS TEXT)
  AND EXISTS (
        SELECT 1
        FROM commerce_order o
        WHERE o.tenant_id = e.tenant_id
          AND o.id = e.order_id
          AND ((o.organization_id = CAST($3 AS TEXT)) OR (o.organization_id IS NULL AND $3 IS NULL) OR (o.organization_id = '0' AND $3 IS NULL))
          AND o.owner_user_id = CAST($4 AS TEXT)
      )
ORDER BY e.created_at DESC, e.id DESC
LIMIT $5 OFFSET $6
"#;

const RETRIEVE_OWNER_ORDER: &str = r#"
SELECT
    o.id AS order_id,
    o.order_no AS order_sn,
    o.status,
    o.payment_status,
    o.subject,
    o.created_at,
    o.paid_at AS pay_time,
    o.expired_at AS expire_time,
    COALESCE(
        (
            SELECT b.payable_amount
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
            SELECT b.discount_amount
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
            SELECT SUM(oi.quantity)
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
    COALESCE(NULLIF(pa.out_trade_no, ''), NULLIF(o.order_no, '')) AS out_trade_no,
    CAST(pa.id AS TEXT) AS transaction_id
FROM commerce_order o
LEFT JOIN commerce_payment_intent pi
    ON pi.tenant_id = o.tenant_id
   AND (pi.organization_id IS NULL OR o.organization_id IS NULL OR pi.organization_id = o.organization_id)
   AND pi.owner_user_id = o.owner_user_id
   AND pi.order_id = o.id
LEFT JOIN commerce_payment_attempt pa
    ON pa.tenant_id = o.tenant_id
   AND (pa.organization_id IS NULL OR o.organization_id IS NULL OR pa.organization_id = o.organization_id)
   AND pa.owner_user_id = o.owner_user_id
   AND pa.order_id = o.id
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id IS NULL AND $2 IS NULL) OR (o.organization_id = '0' AND $2 IS NULL))
  AND o.owner_user_id = CAST($3 AS TEXT)
  AND o.id = CAST($4 AS TEXT)
LIMIT 1
"#;

const RETRIEVE_OWNER_ORDER_PAYMENT_STATUS: &str = r#"
SELECT o.status, o.payment_status
FROM commerce_order o
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id IS NULL AND $2 IS NULL) OR (o.organization_id = '0' AND $2 IS NULL))
  AND o.owner_user_id = CAST($3 AS TEXT)
  AND o.id = CAST($4 AS TEXT)
LIMIT 1
"#;

const RETRIEVE_OWNER_ORDER_FULFILLMENT_STATUS: &str = r#"
SELECT o.fulfillment_status
FROM commerce_order o
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id IS NULL AND $2 IS NULL) OR (o.organization_id = '0' AND $2 IS NULL))
  AND o.owner_user_id = CAST($3 AS TEXT)
  AND o.id = CAST($4 AS TEXT)
LIMIT 1
"#;

const LIST_ORDER_ITEMS: &str = r#"
SELECT
    id,
    title AS product_name,
    quantity,
    unit_price_amount,
    total_amount
FROM commerce_order_item
WHERE tenant_id = CAST($1 AS TEXT)
  AND order_id = CAST($2 AS TEXT)
ORDER BY created_at ASC, id ASC
LIMIT $3
"#;

const OWNER_ORDER_STATISTICS: &str = r#"
SELECT
    COUNT(*) AS total_orders,
    SUM(CASE WHEN LOWER(o.status) IN ('pending_payment', 'unpaid', 'wait_pay') THEN 1 ELSE 0 END) AS pending_payment,
    SUM(CASE WHEN LOWER(o.status) IN ('paid', 'fulfilled') THEN 1 ELSE 0 END) AS pending_shipment,
    SUM(CASE WHEN LOWER(o.status) IN ('shipped', 'delivered') THEN 1 ELSE 0 END) AS pending_receipt,
    SUM(CASE WHEN LOWER(o.status) IN ('completed', 'finished') THEN 1 ELSE 0 END) AS completed,
    CAST(
        COALESCE(
            SUM(
                CAST(
                    COALESCE(
                        (
                            SELECT b.payable_amount
                            FROM commerce_order_amount_breakdown b
                            WHERE b.tenant_id = o.tenant_id
                              AND b.order_id = o.id
                              AND b.allocation_type = 'order_total'
                            LIMIT 1
                        ),
                        '0'
                    ) AS BIGINT
                )
            ),
            0
        ) AS TEXT
    ) AS total_amount
FROM commerce_order o
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id IS NULL AND $2 IS NULL) OR (o.organization_id = '0' AND $2 IS NULL))
  AND o.owner_user_id = CAST($3 AS TEXT)
"#;

const LIST_PAYMENT_METHODS: &str = r#"
SELECT
    m.id,
    m.method_key,
    m.display_name,
    m.provider_code,
    m.sort_order,
    COALESCE((
        SELECT STRING_AGG(DISTINCT c.scene_code, ',')
        FROM commerce_payment_channel c
        WHERE c.tenant_id = m.tenant_id
          AND (
                c.organization_id IS NULL
                OR m.organization_id IS NULL
                OR c.organization_id = m.organization_id
              )
          AND (
                c.method_id = m.id
                OR (c.method_id IS NULL AND c.provider_code = m.provider_code)
              )
          AND c.status = 'active'
          AND c.deleted_at IS NULL
    ), 'web') AS scene_codes
FROM commerce_payment_method m
WHERE (
        (m.tenant_id = CAST($1 AS TEXT) AND m.organization_id = CAST($2 AS TEXT))
        OR (m.tenant_id = CAST($1 AS TEXT) AND m.organization_id = '0')
      )
  AND m.status = 'active'
  AND m.deleted_at IS NULL
ORDER BY COALESCE(m.sort_order, 0) ASC, m.id ASC
"#;

use crate::order_settlement_context::OrderPaymentSettlementContext;

#[derive(Debug, Clone)]
pub struct PostgresCommerceOrderStore {
    pool: PgPool,
    partner_relation_port: Arc<dyn OrderPartnerRelationPort>,
}

impl PostgresCommerceOrderStore {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            partner_relation_port: Arc::new(NoopOrderPartnerRelationPort),
        }
    }

    /// Attaches the partner relation resolver used to snapshot the order's
    /// partner facts at creation time. Defaults to a no-op port.
    pub fn with_partner_relation_port(
        mut self,
        partner_relation_port: Arc<dyn OrderPartnerRelationPort>,
    ) -> Self {
        self.partner_relation_port = partner_relation_port;
        self
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn load_order_subject(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: &str,
    ) -> Result<Option<String>, CommerceServiceError> {
        Ok(self
            .load_order_payment_settlement_context(tenant_id, organization_id, order_id)
            .await?
            .map(|context| context.subject)
            .filter(|subject| !subject.trim().is_empty()))
    }

    pub async fn load_order_payment_settlement_context(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        order_id: &str,
    ) -> Result<Option<OrderPaymentSettlementContext>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT o.owner_user_id,
                   o.order_no,
                   o.subject,
                   o.membership_action,
                   (
                       SELECT oi.sku_snapshot_json
                       FROM commerce_order_item oi
                       WHERE oi.tenant_id = o.tenant_id
                         AND oi.order_id = o.id
                       ORDER BY oi.created_at ASC, oi.id ASC
                       LIMIT 1
                   ) AS sku_snapshot_json
            FROM commerce_order o
            WHERE o.tenant_id = CAST($1 AS TEXT)
              AND o.id = CAST($2 AS TEXT)
              AND ((o.organization_id = CAST($3 AS TEXT)) OR (o.organization_id IS NULL AND $3 IS NULL) OR (o.organization_id = '0' AND $3 IS NULL))
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(order_id)
        .bind(organization_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| store_error("failed to load order payment settlement context", error))?;

        let Some(row) = row else {
            return Ok(None);
        };
        let stored_subject = row.try_get::<Option<String>, _>("subject").ok().flatten();
        let snapshot = row
            .try_get::<Option<String>, _>("sku_snapshot_json")
            .ok()
            .flatten();
        let subject =
            stable_order_settlement_subject(stored_subject.as_deref(), snapshot.as_deref());
        let membership_action = row
            .try_get::<Option<String>, _>("membership_action")
            .ok()
            .flatten();
        let membership_purchase = membership_purchase_snapshot(
            &subject,
            &string_cell(&row, "order_no"),
            membership_action.as_deref(),
            snapshot.as_deref(),
        )?;
        Ok(Some(OrderPaymentSettlementContext {
            membership_purchase,
            owner_user_id: string_cell(&row, "owner_user_id"),
            subject,
        }))
    }

    pub async fn list_owner_orders(
        &self,
        query: OrderOwnerListQuery,
    ) -> Result<OrderOwnerListPage, CommerceServiceError> {
        let rows = sqlx::query(LIST_OWNER_ORDERS)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(&query.owner_user_id)
            .bind(query.status.as_deref())
            .bind(query.subject.as_deref())
            .bind(query.limit())
            .bind(query.offset())
            .fetch_all(&self.pool)
            .await
            .or_else(empty_rows_when_read_model_is_missing)
            .map_err(|error| store_error("failed to list owner orders", error))?;

        // COUNT(*) OVER() emits the same total on every row; read it from the
        // first row, or default to 0 when the page is empty.
        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);

        let items = rows
            .iter()
            .map(map_order_summary_row)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(OrderOwnerListPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    /// 列出订单事件（owner 域）。
    ///
    /// 通过 `EXISTS` 子查询校验订单归属，避免越权读取其他用户的事件。
    /// `COUNT(*) OVER()` 窗口函数在一次往返中给出无条件总数，用于
    /// `hasMore` / 总页数渲染，避免 N+1 或双查询模式。
    pub async fn list_owner_order_events(
        &self,
        query: OrderOwnerEventListQuery,
    ) -> Result<OrderOwnerEventPage, CommerceServiceError> {
        let rows = sqlx::query(LIST_OWNER_ORDER_EVENTS)
            .bind(&query.tenant_id)
            .bind(&query.order_id)
            .bind(query.organization_id.as_deref())
            .bind(&query.owner_user_id)
            .bind(query.limit())
            .bind(query.offset())
            .fetch_all(&self.pool)
            .await
            .or_else(empty_rows_when_read_model_is_missing)
            .map_err(|error| store_error("failed to list owner order events", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);

        let items = rows
            .iter()
            .map(map_owner_order_event_row)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(OrderOwnerEventPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn retrieve_owner_order(
        &self,
        query: OrderOwnerDetailQuery,
    ) -> Result<Option<OrderOwnerDetail>, CommerceServiceError> {
        let row = sqlx::query(RETRIEVE_OWNER_ORDER)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(&query.owner_user_id)
            .bind(&query.order_id)
            .fetch_optional(&self.pool)
            .await
            .or_else(none_when_read_model_is_missing)
            .map_err(|error| store_error("failed to retrieve owner order", error))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let summary = map_order_summary_row(&row)?;
        let items = load_order_items(&self.pool, &query.tenant_id, &query.order_id).await?;
        Ok(Some(OrderOwnerDetail {
            summary,
            payment_status: optional_string_cell(&row, "payment_status"),
            items,
            out_trade_no: optional_string_cell(&row, "out_trade_no"),
            transaction_id: optional_string_cell(&row, "transaction_id"),
        }))
    }

    pub async fn retrieve_owner_order_payment_status(
        &self,
        query: OrderOwnerDetailQuery,
    ) -> Result<Option<OrderOwnerPaymentStatus>, CommerceServiceError> {
        let row = sqlx::query(RETRIEVE_OWNER_ORDER_PAYMENT_STATUS)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(&query.owner_user_id)
            .bind(&query.order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| store_error("failed to retrieve owner order payment status", error))?;

        Ok(row.map(|row| OrderOwnerPaymentStatus {
            status: string_cell(&row, "status"),
            payment_status: optional_string_cell(&row, "payment_status"),
        }))
    }

    /// Returns the stored `fulfillment_status` for an owner order, or `None`
    /// when the order does not exist. Callers decide whether physical
    /// inventory release is required from the returned status.
    pub async fn retrieve_owner_order_fulfillment_status(
        &self,
        query: OrderOwnerDetailQuery,
    ) -> Result<Option<String>, CommerceServiceError> {
        let row = sqlx::query(RETRIEVE_OWNER_ORDER_FULFILLMENT_STATUS)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(&query.owner_user_id)
            .bind(&query.order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| {
                store_error("failed to retrieve owner order fulfillment status", error)
            })?;
        Ok(row.map(|row| optional_string_cell(&row, "fulfillment_status").unwrap_or_default()))
    }

    pub async fn retrieve_owner_order_statistics(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
    ) -> Result<OrderOwnerStatistics, CommerceServiceError> {
        match sqlx::query(OWNER_ORDER_STATISTICS)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(owner_user_id)
            .fetch_one(&self.pool)
            .await
        {
            Ok(row) => Ok(OrderOwnerStatistics {
                total_orders: row.try_get::<i64, _>("total_orders").unwrap_or(0),
                pending_payment: row.try_get::<i64, _>("pending_payment").unwrap_or(0),
                pending_shipment: row.try_get::<i64, _>("pending_shipment").unwrap_or(0),
                pending_receipt: row.try_get::<i64, _>("pending_receipt").unwrap_or(0),
                completed: row.try_get::<i64, _>("completed").unwrap_or(0),
                total_amount: {
                    // The aggregate is projected as text after integer-unit summation
                    // and re-validated through the stored-money reader.
                    let raw = row
                        .try_get::<String, _>("total_amount")
                        .unwrap_or_else(|_| "0".to_owned());
                    commerce_money_stored(&raw, "total_amount", "order statistics")?
                },
            }),
            Err(error) if read_model_table_is_missing(&error) => Ok(empty_order_statistics()),
            Err(error) => Err(store_error(
                "failed to retrieve owner order statistics",
                error,
            )),
        }
    }

    pub async fn list_payment_methods(
        &self,
        query: PaymentMethodListQuery,
    ) -> Result<Vec<PaymentMethodItem>, CommerceServiceError> {
        let rows = sqlx::query(LIST_PAYMENT_METHODS)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .or_else(empty_rows_when_read_model_is_missing)
            .map_err(|error| store_error("failed to list payment methods", error))?;

        Ok(rows
            .iter()
            .map(|row| PaymentMethodItem {
                id: string_cell(row, "id"),
                method_key: string_cell(row, "method_key"),
                display_name: string_cell(row, "display_name"),
                provider_code: string_cell(row, "provider_code"),
                scene_codes: parse_scene_codes_csv(&string_cell(row, "scene_codes")),
                sort_order: row.try_get::<i64, _>("sort_order").unwrap_or(0),
            })
            .collect())
    }
    pub async fn create_owner_order(
        &self,
        command: CreateOwnerOrderCommand,
    ) -> Result<CreateOwnerOrderOutcome, CommerceServiceError> {
        let order_id = format!("order-{}", command.checkout_session_id);
        // Resolve the order's partner relation before opening the write
        // transaction: the resolution port reads the partner domain on the
        // same federated commerce pool and must not run inside this
        // transaction (partner facts are optional; a missing binding never
        // blocks order creation).
        let partner_snapshot = match command.partner_snapshot.clone() {
            Some(snapshot) => Some(snapshot),
            None => {
                // The partner relation is an optional snapshot: a resolution
                // failure (partner domain down, binding store error) must
                // never block order creation. Degrade to no relation.
                match self
                    .partner_relation_port
                    .resolve_order_partner(
                        &command.tenant_id,
                        command.organization_id.as_deref(),
                        &command.owner_user_id,
                    )
                    .await
                {
                    Ok(snapshot) => snapshot,
                    Err(error) => {
                        tracing::warn!(
                            target: "sdkwork.order.partner",
                            error = ?error,
                            "partner relation resolution failed; creating order without partner snapshot"
                        );
                        None
                    }
                }
            }
        };
        let mut tx = self.pool.begin().await.map_err(|error| {
            store_error("failed to begin create owner order transaction", error)
        })?;

        let existing = sqlx::query(
            r#"
            SELECT
                o.id AS order_id,
                o.order_no AS order_sn,
                o.status,
                o.merchant_organization_id,
                o.expired_at,
                o.partner_id,
                o.partner_snapshot_json,
                COALESCE(
                    (
                        SELECT b.payable_amount
                        FROM commerce_order_amount_breakdown b
                        WHERE b.tenant_id = o.tenant_id
                          AND b.order_id = o.id
                          AND b.allocation_type = 'order_total'
                        LIMIT 1
                    ),
                    '0'
                ) AS total_amount
            FROM commerce_order o
            WHERE o.id = $1
              AND o.tenant_id = CAST($2 AS TEXT)
              AND ((o.organization_id = CAST($3 AS TEXT)) OR (o.organization_id IS NULL AND $4 IS NULL) OR (o.organization_id = '0' AND $4 IS NULL))
              AND o.owner_user_id = CAST($5 AS TEXT)
            FOR UPDATE
            "#,
        )
        .bind(&order_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to lock owner order for create", error))?;

        if let Some(row) = existing {
            let inventory_lines = load_order_inventory_lines_postgres(
                &mut tx,
                &command.tenant_id,
                &string_cell(&row, "order_id"),
            )
            .await?;
            tx.commit().await.map_err(|error| {
                store_error("failed to commit existing owner order lookup", error)
            })?;
            let total_amount = commerce_money_stored(
                &string_cell(&row, "total_amount"),
                "total_amount",
                &order_id,
            )?;
            return Ok(CreateOwnerOrderOutcome {
                inventory_lines,
                merchant_organization_id: optional_string_cell(&row, "merchant_organization_id"),
                order_id: string_cell(&row, "order_id"),
                order_sn: string_cell(&row, "order_sn"),
                status: string_cell(&row, "status"),
                total_amount,
                expires_at: optional_string_cell(&row, "expired_at"),
                partner_snapshot: optional_string_cell(&row, "partner_snapshot_json")
                    .and_then(|json| serde_json::from_str::<OrderPartnerSnapshot>(&json).ok())
                    .or_else(|| {
                        optional_string_cell(&row, "partner_id").map(|partner_id| {
                            OrderPartnerSnapshot {
                                partner_id,
                                name: String::new(),
                                level_no: String::new(),
                                status: String::new(),
                            }
                        })
                    }),
            });
        }

        let session = load_checkout_session_for_order(&mut tx, &command).await?;
        let lines = load_checkout_lines_for_order(&mut tx, &command).await?;
        if lines.is_empty() {
            return Err(CommerceServiceError::conflict(
                "checkout session has no selected lines",
            ));
        }
        let quote = load_checkout_quote_for_order(&mut tx, &command).await?;
        let now = current_command_timestamp();
        let order_sn = command.request_no.clone();
        let subject = checkout_order_subject(&lines);
        let currency_code = string_cell(&session, "currency_code");
        let payable_amount = normalize_money_amount(&string_cell(&quote, "payable_amount"))?;
        let original_amount = normalize_money_amount(&string_cell(&quote, "original_amount"))?;
        let discount_amount = normalize_money_amount(&string_cell(&quote, "discount_amount"))?;
        let total_amount = commerce_money(&payable_amount)?;
        let expires_at =
            optional_string_cell(&session, "expires_at").unwrap_or_else(|| now.clone());

        let partner_snapshot_json = partner_snapshot
            .as_ref()
            .map(|snapshot| {
                serde_json::to_string(snapshot)
                    .map_err(|error| store_error("failed to serialize partner snapshot", error))
            })
            .transpose()?;

        sqlx::query(
            r#"
            INSERT INTO commerce_order
                (id, tenant_id, organization_id, owner_user_id, order_no, status, payment_status,
                fulfillment_status, refund_status, subject, currency_code, merchant_organization_id,
                 shop_id, shipping_address_snapshot_json, shop_snapshot_json, request_no,
                 idempotency_key, partner_id, partner_snapshot_json, created_at, paid_at,
                 cancelled_at, expired_at, updated_at)
            VALUES
                ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, 'pending_inventory',
                 'pending', 'unfulfilled', 'none', $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                 $16, $17, NULL, NULL, $18, $19)
            "#,
        )
        .bind(&order_id)
        .bind(&command.tenant_id)
        .bind(normalize_organization_scope(command.organization_id.as_deref()))
        .bind(&command.owner_user_id)
        .bind(&order_sn)
        .bind(&subject)
        .bind(&currency_code)
        .bind(optional_string_cell(&session, "merchant_organization_id"))
        .bind(optional_string_cell(&session, "shop_id"))
        .bind(optional_string_cell(&session, "shipping_address_snapshot_json"))
        .bind(optional_string_cell(&session, "shop_snapshot_json"))
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(partner_snapshot.as_ref().map(|snapshot| &snapshot.partner_id))
        .bind(partner_snapshot_json.as_deref())
        .bind(&now)
        .bind(&expires_at)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert owner order", error))?;

        for line in &lines {
            let line_id = string_cell(line, "id");
            let item_id = format!("{order_id}-item-{line_id}");
            let quantity = line
                .try_get::<i64, _>("quantity")
                .map_err(|error| store_error("failed to decode checkout line quantity", error))?;
            let unit_price = string_cell(line, "price_amount_snapshot");
            let total_amount = multiply_money_amount(&unit_price, quantity)?;
            sqlx::query(
                r#"
                INSERT INTO commerce_order_item
                    (id, tenant_id, order_id, product_id, shop_id, sku_id, sku_snapshot_json,
                     title, quantity, unit_price_amount, discount_amount, tax_amount,
                     total_amount, fulfillment_status, refund_status, created_at)
                VALUES
                    ($1, CAST($2 AS TEXT), $3, $4, $5, $6, $7, $8, $9, $10, '0', '0', $11,
                     'unfulfilled', 'none', $12)
                "#,
            )
            .bind(&item_id)
            .bind(&command.tenant_id)
            .bind(&order_id)
            .bind(optional_string_cell(line, "product_id"))
            .bind(optional_string_cell(line, "shop_id"))
            .bind(string_cell(line, "sku_id"))
            .bind(string_cell(line, "sku_snapshot_json"))
            .bind(checkout_line_title(line))
            .bind(quantity)
            .bind(&unit_price)
            .bind(&total_amount)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|error| store_error("failed to insert owner order item", error))?;
        }

        sqlx::query(
            r#"
            INSERT INTO commerce_order_amount_breakdown
                (id, tenant_id, organization_id, order_id, order_item_id, allocation_type,
                 original_amount, discount_amount, payable_amount, currency_code, created_at)
            VALUES
                ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), $4, NULL, 'order_total', $5, $6, $7, $8, $9)
            "#,
        )
        .bind(format!("{order_id}-amount"))
        .bind(&command.tenant_id)
        .bind(normalize_organization_scope(command.organization_id.as_deref()))
        .bind(&order_id)
        .bind(&original_amount)
        .bind(&discount_amount)
        .bind(&payable_amount)
        .bind(&currency_code)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert owner order amount breakdown", error))?;

        sqlx::query(
            r#"
            UPDATE commerce_checkout_session
            SET status = 'submitted', submitted_at = $1, updated_at = $2
            WHERE id = $3
              AND tenant_id = CAST($4 AS TEXT)
              AND owner_user_id = CAST($5 AS TEXT)
            "#,
        )
        .bind(&now)
        .bind(&now)
        .bind(&command.checkout_session_id)
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to submit checkout session", error))?;

        tx.commit().await.map_err(|error| {
            store_error("failed to commit create owner order transaction", error)
        })?;

        Ok(CreateOwnerOrderOutcome {
            inventory_lines: physical_inventory_lines_postgres(&lines),
            merchant_organization_id: optional_string_cell(&session, "merchant_organization_id"),
            order_id,
            order_sn,
            status: "pending_inventory".to_owned(),
            total_amount,
            expires_at: optional_string_cell(&session, "expires_at"),
            partner_snapshot,
        })
    }

    pub async fn mark_owner_order_inventory_reserved(
        &self,
        tenant_id: &str,
        owner_user_id: &str,
        order_id: &str,
    ) -> Result<(), CommerceServiceError> {
        let result = sqlx::query(
            r#"
            UPDATE commerce_order
            SET status = 'pending_payment', fulfillment_status = 'inventory_reserved', updated_at = $1
            WHERE tenant_id = CAST($2 AS TEXT) AND owner_user_id = CAST($3 AS TEXT)
              AND id = CAST($4 AS TEXT) AND status IN ('pending_inventory', 'pending_payment')
            "#,
        )
        .bind(current_command_timestamp())
        .bind(tenant_id)
        .bind(owner_user_id)
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(|error| store_error("failed to activate inventory-reserved order", error))?;
        if result.rows_affected() == 0 {
            return Err(CommerceServiceError::invalid_state(
                "order is not waiting for inventory reservation",
            ));
        }
        Ok(())
    }

    pub async fn mark_owner_order_inventory_failed(
        &self,
        tenant_id: &str,
        owner_user_id: &str,
        order_id: &str,
    ) -> Result<(), CommerceServiceError> {
        sqlx::query(
            r#"
            UPDATE commerce_order
            SET status = 'inventory_failed', fulfillment_status = 'inventory_failed', updated_at = $1
            WHERE tenant_id = CAST($2 AS TEXT) AND owner_user_id = CAST($3 AS TEXT)
              AND id = CAST($4 AS TEXT) AND status = 'pending_inventory'
            "#,
        )
        .bind(current_command_timestamp())
        .bind(tenant_id)
        .bind(owner_user_id)
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(|error| store_error("failed to close inventory-failed order", error))?;
        Ok(())
    }

    pub async fn cancel_owner_order(
        &self,
        command: CancelOwnerOrderCommand,
    ) -> Result<(), CommerceServiceError> {
        use crate::order_lifecycle::{
            insert_order_cancellation_postgres, insert_order_event_postgres,
            order_cancel_idempotency_key, OrderLifecycleAuditInput,
        };

        let now = current_command_timestamp();
        let idempotency_key = order_cancel_idempotency_key(&command.order_id);
        let request_no = format!("cancel-{}", command.order_id);

        let mut tx = self.pool.begin().await.map_err(|error| {
            store_error("failed to begin cancel owner order transaction", error)
        })?;

        let from_status = sqlx::query_scalar::<_, String>(
            r#"
            SELECT status
            FROM commerce_order
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
              AND owner_user_id = CAST($3 AS TEXT)
              AND id = CAST($4 AS TEXT)
            "#,
        )
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to load order status before cancel", error))?;

        let Some(from_status) = from_status else {
            tx.rollback().await.map_err(|error| {
                store_error("failed to rollback cancel owner order transaction", error)
            })?;
            return Err(CommerceServiceError::not_found("order was not found"));
        };

        if from_status.eq_ignore_ascii_case("cancelled") {
            tx.rollback().await.map_err(|error| {
                store_error("failed to rollback cancel owner order transaction", error)
            })?;
            return Ok(());
        }

        let result = sqlx::query(
            r#"
            UPDATE commerce_order
            SET status = 'cancelled',
                payment_status = 'closed',
                cancelled_at = $1,
                updated_at = $2
            WHERE tenant_id = CAST($3 AS TEXT)
              AND ((organization_id = CAST($4 AS TEXT)) OR (organization_id IS NULL AND $4 IS NULL) OR (organization_id = '0' AND $4 IS NULL))
              AND owner_user_id = CAST($5 AS TEXT)
              AND id = CAST($6 AS TEXT)
              AND LOWER(COALESCE(status, '')) IN ('draft', 'pending', 'pending_payment', 'unpaid')
            "#,
        )
        .bind(&now)
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to cancel owner order", error))?;

        if result.rows_affected() == 0 {
            let current_status = sqlx::query_scalar::<_, String>(
                r#"
                SELECT status
                FROM commerce_order
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
                  AND owner_user_id = CAST($3 AS TEXT)
                  AND id = CAST($4 AS TEXT)
                "#,
            )
            .bind(&command.tenant_id)
            .bind(command.organization_id.as_deref())
            .bind(&command.owner_user_id)
            .bind(&command.order_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| store_error("failed to reload order status after cancel", error))?;

            tx.rollback().await.map_err(|error| {
                store_error("failed to rollback cancel owner order transaction", error)
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
            actor_type: "buyer",
            actor_id: Some(command.owner_user_id.clone()),
            reason_code: command
                .cancel_type
                .clone()
                .or_else(|| Some("user_cancel".to_owned())),
            reason_message: command.cancel_reason.clone(),
            request_no,
            idempotency_key,
            now: now.clone(),
        };
        insert_order_event_postgres(&mut tx, &audit).await?;
        insert_order_cancellation_postgres(&mut tx, &audit).await?;

        tx.commit().await.map_err(|error| {
            store_error("failed to commit cancel owner order transaction", error)
        })?;
        Ok(())
    }

    /// Buyer confirmation of physical receipt: advances the order from
    /// shipped/awaiting-shipment to `completed` (fulfillment `delivered`)
    /// with a lifecycle event. Idempotent — a completed order replays.
    pub async fn confirm_owner_order_receipt(
        &self,
        query: OrderOwnerDetailQuery,
        request_no: &str,
    ) -> Result<(), CommerceServiceError> {
        use crate::order_lifecycle::{insert_order_event_postgres, OrderLifecycleAuditInput};

        let now = current_command_timestamp();
        let idempotency_key = format!("order-receipt:{}", query.order_id);
        let mut tx = self.pool.begin().await.map_err(|error| {
            store_error(
                "failed to begin confirm owner order receipt transaction",
                error,
            )
        })?;
        let from_status = sqlx::query_scalar::<_, String>(
            r#"
            SELECT status
            FROM commerce_order
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3 IS NULL) OR (organization_id = '0' AND $3 IS NULL))
              AND owner_user_id = CAST($4 AS TEXT)
              AND id = CAST($5 AS TEXT)
            FOR UPDATE
            "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(&query.order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to load order before receipt confirmation", error))?;

        let Some(from_status) = from_status else {
            tx.rollback().await.map_err(|error| {
                store_error("failed to rollback receipt confirmation transaction", error)
            })?;
            return Err(CommerceServiceError::not_found("order was not found"));
        };
        if from_status.eq_ignore_ascii_case("completed") {
            tx.rollback().await.map_err(|error| {
                store_error("failed to rollback receipt confirmation replay", error)
            })?;
            return Ok(());
        }

        let result = sqlx::query(
            r#"
            UPDATE commerce_order
            SET status = 'completed',
                fulfillment_status = 'delivered',
                updated_at = $1
            WHERE tenant_id = CAST($2 AS TEXT)
              AND ((organization_id = CAST($3 AS TEXT)) OR (organization_id IS NULL AND $4 IS NULL) OR (organization_id = '0' AND $4 IS NULL))
              AND owner_user_id = CAST($5 AS TEXT)
              AND id = CAST($6 AS TEXT)
              AND LOWER(COALESCE(status, '')) IN ('paid', 'fulfilled', 'shipped')
              AND LOWER(COALESCE(fulfillment_status, '')) IN ('awaiting_shipment', 'shipped')
            "#,
        )
        .bind(&now)
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(&query.order_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to confirm owner order receipt", error))?;
        if result.rows_affected() == 0 {
            tx.rollback().await.map_err(|error| {
                store_error("failed to rollback receipt confirmation transaction", error)
            })?;
            return Err(CommerceServiceError::conflict(
                "order is not awaiting receipt confirmation",
            ));
        }

        let audit = OrderLifecycleAuditInput {
            tenant_id: query.tenant_id.clone(),
            organization_id: query.organization_id.clone(),
            order_id: query.order_id.clone(),
            event_type: "completed",
            from_status,
            to_status: "completed",
            actor_type: "buyer",
            actor_id: Some(query.owner_user_id.clone()),
            reason_code: Some("buyer_confirmed_receipt".to_owned()),
            reason_message: None,
            request_no: request_no.to_owned(),
            idempotency_key,
            now,
        };
        insert_order_event_postgres(&mut tx, &audit).await?;
        tx.commit().await.map_err(|error| {
            store_error("failed to commit owner order receipt confirmation", error)
        })?;
        Ok(())
    }
}

async fn load_order_items(
    pool: &PgPool,
    tenant_id: &str,
    order_id: &str,
) -> Result<Vec<OrderOwnerItem>, CommerceServiceError> {
    let rows = sqlx::query(LIST_ORDER_ITEMS)
        .bind(tenant_id)
        .bind(order_id)
        .bind(MAX_ORDER_LINE_ITEMS)
        .fetch_all(pool)
        .await
        .or_else(empty_rows_when_read_model_is_missing)
        .map_err(|error| store_error("failed to list order items", error))?;

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

fn map_owner_order_event_row(
    row: &sqlx::postgres::PgRow,
) -> Result<OrderOwnerEventView, CommerceServiceError> {
    Ok(OrderOwnerEventView {
        event_id: string_cell(row, "event_id"),
        order_id: string_cell(row, "order_id"),
        event_type: string_cell(row, "event_type"),
        from_status: optional_string_cell(row, "from_status"),
        to_status: string_cell(row, "to_status"),
        actor_type: optional_string_cell(row, "actor_type"),
        actor_id: optional_string_cell(row, "actor_id"),
        message: optional_string_cell(row, "message"),
        created_at: string_cell(row, "created_at"),
    })
}

fn map_order_summary_row(
    row: &sqlx::postgres::PgRow,
) -> Result<OrderOwnerSummary, CommerceServiceError> {
    let order_id = string_cell(row, "order_id");
    // Stored money columns may carry legacy major-unit decimals (`0.00`); a
    // single legacy row must not fail the whole owner order list.
    let total_amount =
        commerce_money_stored(&string_cell(row, "total_amount"), "total_amount", &order_id)?;
    let discount_amount = commerce_money_stored(
        &string_cell(row, "discount_amount"),
        "discount_amount",
        &order_id,
    )?;
    let status = string_cell(row, "status");
    let payment_status = optional_string_cell(row, "payment_status");
    let paid_amount = if status.eq_ignore_ascii_case("paid")
        || status.eq_ignore_ascii_case("completed")
        || status.eq_ignore_ascii_case("fulfilled")
        || payment_status.is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "success" | "succeeded" | "paid"
            )
        }) {
        Some(total_amount.clone())
    } else {
        None
    };

    Ok(OrderOwnerSummary {
        order_id: string_cell(row, "order_id"),
        order_sn: string_cell(row, "order_sn"),
        status,
        subject: string_cell(row, "subject"),
        total_amount,
        paid_amount,
        discount_amount: Some(discount_amount),
        currency_code: string_cell(row, "currency_code"),
        quantity: row.try_get::<i64, _>("quantity").unwrap_or(1),
        created_at: string_cell(row, "created_at"),
        pay_time: optional_string_cell(row, "pay_time"),
        expire_time: optional_string_cell(row, "expire_time"),
        payment_method: optional_string_cell(row, "payment_method"),
        points: positive_i64_cell(row, "recharge_points"),
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
    })
}

fn positive_i64_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<i64> {
    row.try_get::<i64, _>(column)
        .ok()
        .filter(|value| *value > 0)
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}

fn store_error(message: &str, error: impl std::fmt::Display) -> CommerceServiceError {
    crate::sql_store_error::map_sql_store_error(message, error)
}

async fn load_checkout_session_for_order(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateOwnerOrderCommand,
) -> Result<sqlx::postgres::PgRow, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT currency_code, expires_at, status, shop_id, merchant_organization_id,
               shop_snapshot_json, shipping_address_snapshot_json
        FROM commerce_checkout_session
        WHERE id = $1
          AND tenant_id = CAST($2 AS TEXT)
          AND ((organization_id = CAST($3 AS TEXT)) OR (organization_id IS NULL AND $4 IS NULL) OR (organization_id = '0' AND $4 IS NULL))
          AND owner_user_id = CAST($5 AS TEXT)
          AND LOWER(COALESCE(status, '')) IN ('active', 'quoted', 'open')
        "#,
    )
    .bind(&command.checkout_session_id)
    .bind(&command.tenant_id)
    .bind(command.organization_id.as_deref())
    .bind(command.organization_id.as_deref())
    .bind(&command.owner_user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load checkout session", error))?
    .ok_or_else(|| CommerceServiceError::conflict("checkout session is not orderable"))?;
    let expires_at = optional_string_cell(&row, "expires_at");
    let now_seconds = current_command_timestamp()
        .trim()
        .parse::<i64>()
        .unwrap_or(0);
    if expires_at
        .as_deref()
        .is_some_and(|value| value.trim().parse::<i64>().unwrap_or(i64::MAX) <= now_seconds)
    {
        return Err(CommerceServiceError::invalid_state(
            "checkout session has expired; create a new checkout session",
        ));
    }
    Ok(row)
}

async fn load_checkout_lines_for_order(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateOwnerOrderCommand,
) -> Result<Vec<sqlx::postgres::PgRow>, CommerceServiceError> {
    sqlx::query(
        r#"
        SELECT id, product_id, shop_id, sku_id, sku_snapshot_json, quantity, price_amount_snapshot,
               fulfillment_type
        FROM commerce_checkout_line
        WHERE tenant_id = CAST($1 AS TEXT)
          AND checkout_session_id = $2
          AND selected = 1
        ORDER BY created_at ASC, id ASC
        "#,
    )
    .bind(&command.tenant_id)
    .bind(&command.checkout_session_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load checkout lines", error))
}

async fn load_checkout_quote_for_order(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateOwnerOrderCommand,
) -> Result<sqlx::postgres::PgRow, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT original_amount, discount_amount, payable_amount, expires_at
        FROM commerce_checkout_quote
        WHERE tenant_id = CAST($1 AS TEXT)
          AND checkout_session_id = $2
          AND LOWER(COALESCE(quote_status, '')) IN ('active', 'quoted', 'ready')
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(&command.tenant_id)
    .bind(&command.checkout_session_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load checkout quote", error))?
    .ok_or_else(|| CommerceServiceError::conflict("checkout quote was not found"))?;
    let expires_at = optional_string_cell(&row, "expires_at");
    let now_seconds = current_command_timestamp()
        .trim()
        .parse::<i64>()
        .unwrap_or(0);
    if expires_at
        .as_deref()
        .is_some_and(|value| value.trim().parse::<i64>().unwrap_or(i64::MAX) <= now_seconds)
    {
        return Err(CommerceServiceError::invalid_state(
            "checkout quote has expired; create a new checkout session",
        ));
    }
    Ok(row)
}

fn checkout_order_subject(lines: &[sqlx::postgres::PgRow]) -> String {
    lines.first().map_or_else(
        || "product".to_owned(),
        |line| {
            let fulfillment_type = optional_string_cell(line, "fulfillment_type");
            let snapshot = optional_string_cell(line, "sku_snapshot_json");
            stable_checkout_order_subject(fulfillment_type.as_deref(), snapshot.as_deref())
        },
    )
}

fn physical_inventory_lines_postgres(
    lines: &[sqlx::postgres::PgRow],
) -> Vec<sdkwork_order_service::PhysicalInventoryLine> {
    lines
        .iter()
        .filter_map(|line| {
            let fulfillment_type = string_cell(line, "fulfillment_type");
            if !matches!(
                fulfillment_type.trim().to_ascii_lowercase().as_str(),
                "physical" | "physical_shipment"
            ) {
                return None;
            }
            Some(sdkwork_order_service::PhysicalInventoryLine {
                sku_id: string_cell(line, "sku_id"),
                shop_id: string_cell(line, "shop_id"),
                quantity: line.try_get::<i64, _>("quantity").unwrap_or(0),
            })
        })
        .collect()
}

async fn load_order_inventory_lines_postgres(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    order_id: &str,
) -> Result<Vec<sdkwork_order_service::PhysicalInventoryLine>, CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT sku_id, shop_id, quantity
        FROM commerce_order_item
        WHERE tenant_id = CAST($1 AS TEXT) AND order_id = CAST($2 AS TEXT)
        ORDER BY created_at ASC, id ASC
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load order inventory lines", error))?;

    Ok(rows
        .iter()
        .map(|row| sdkwork_order_service::PhysicalInventoryLine {
            sku_id: string_cell(row, "sku_id"),
            shop_id: string_cell(row, "shop_id"),
            quantity: row.try_get::<i64, _>("quantity").unwrap_or(0),
        })
        .collect())
}

fn checkout_line_title(row: &sqlx::postgres::PgRow) -> String {
    let snapshot = string_cell(row, "sku_snapshot_json");
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&snapshot) {
        if let Some(title) = value.get("title").and_then(serde_json::Value::as_str) {
            if !title.trim().is_empty() {
                return title.trim().to_owned();
            }
        }
    }
    string_cell(row, "sku_id")
}

fn current_command_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    format!("{seconds}")
}

fn empty_order_statistics() -> OrderOwnerStatistics {
    OrderOwnerStatistics {
        total_orders: 0,
        pending_payment: 0,
        pending_shipment: 0,
        pending_receipt: 0,
        completed: 0,
        total_amount: CommerceMoney::new("0").expect("zero money should be valid"),
    }
}

#[cfg(test)]
mod tests {
    use super::empty_order_statistics;

    #[test]
    fn empty_order_statistics_uses_smallest_unit_zero() {
        let statistics = empty_order_statistics();

        assert_eq!(statistics.total_amount.as_str(), "0");
    }
}
