use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    CancelManagementOrderCommand, CloseManagementOrderCommand, OrderCancellationListQuery,
    OrderCancellationPage, OrderCancellationView, OrderManagementDetailQuery,
    OrderManagementEventListQuery, OrderManagementEventPage, OrderManagementEventView,
    OrderManagementListPage, OrderManagementListQuery, OrderOwnerDetail, OrderOwnerItem,
    OrderOwnerSummary, OrderPartnerSnapshot,
};
use sqlx::{PgPool, Row};

use crate::money_amount::commerce_money_stored;
use crate::order_limits::MAX_ORDER_LINE_ITEMS;
use crate::postgres_order::PostgresCommerceOrderStore;

const LIST_MANAGEMENT_ORDERS: &str = r#"
SELECT
    o.id AS order_id,
    o.order_no AS order_sn,
    o.status,
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
  AND ($3 IS NULL OR o.status = $3)
  AND (
        $4 IS NULL
        OR o.order_no ILIKE $4
        OR o.subject ILIKE $4
        OR CAST(o.id AS TEXT) ILIKE $4
      )
ORDER BY o.created_at DESC, o.id DESC
LIMIT $5 OFFSET $6
"#;

impl PostgresCommerceOrderStore {
    pub async fn list_management_orders(
        &self,
        query: OrderManagementListQuery,
    ) -> Result<OrderManagementListPage, CommerceServiceError> {
        let search = query.q.as_deref().map(|value| format!("%{value}%"));
        let rows = sqlx::query(LIST_MANAGEMENT_ORDERS)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.status.as_deref())
            .bind(search.as_deref())
            .bind(query.limit())
            .bind(query.offset())
            .fetch_all(self.pool())
            .await
            .map_err(|error| store_error("failed to list management orders", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
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
                o.partner_id,
                o.partner_snapshot_json,
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
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
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
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
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
                cancelled_at = $1,
                updated_at = $2
            WHERE tenant_id = CAST($3 AS TEXT)
              AND ((organization_id = CAST($4 AS TEXT)) OR (organization_id IS NULL AND $4 IS NULL) OR (organization_id = '0' AND $4 IS NULL))
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
                  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
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
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
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
                updated_at = $1
            WHERE tenant_id = CAST($2 AS TEXT)
              AND ((organization_id = CAST($3 AS TEXT)) OR (organization_id IS NULL AND $3 IS NULL) OR (organization_id = '0' AND $3 IS NULL))
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
                  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
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
        SELECT id, product_name, quantity, unit_price_amount, total_amount
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

fn current_command_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_owned())
}
