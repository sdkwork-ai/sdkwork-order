use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    OrderPaymentSettlementAttempt, OwnerOrderPaymentConfirmationFuture,
    OwnerOrderPaymentStateOutcome, OwnerOrderPaymentStatePort,
};
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::order_lifecycle::{insert_order_event_postgres, OrderLifecycleAuditInput};
use crate::sql_store_error::map_sqlx_store_error;
use crate::PostgresCommerceOrderStore;

const LATE_PAYMENT_EVENT_TYPE: &str = "payment_succeeded_after_terminal";
const LATE_PAYMENT_REASON_CODE: &str = "late_payment";

impl PostgresCommerceOrderStore {
    pub async fn mark_owner_order_payment_succeeded(
        &self,
        attempt: &OrderPaymentSettlementAttempt,
        paid_at: &str,
    ) -> Result<OwnerOrderPaymentStateOutcome, CommerceServiceError> {
        let paid_at = required_paid_at(paid_at)?;
        let mut tx = self.pool().begin().await.map_err(|error| {
            map_sqlx_store_error(
                "failed to begin owner order payment-state transaction",
                error,
            )
        })?;

        let row = sqlx::query(
            r#"
            SELECT status, payment_status
            FROM commerce_order
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
              AND owner_user_id = CAST($3 AS TEXT)
              AND id = CAST($4 AS TEXT)
            LIMIT 1
            FOR UPDATE
            "#,
        )
        .bind(&attempt.tenant_id)
        .bind(attempt.organization_id.as_deref())
        .bind(&attempt.owner_user_id)
        .bind(&attempt.order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| map_sqlx_store_error("failed to load owner order payment state", error))?;

        let Some(row) = row else {
            return Err(CommerceServiceError::not_found(
                "owner order was not found for payment settlement",
            ));
        };
        let order_status = row.try_get::<String, _>("status").unwrap_or_default();
        let payment_status = row
            .try_get::<Option<String>, _>("payment_status")
            .ok()
            .flatten();
        let terminal_order_preserved = terminal_order_status(&order_status).is_some();
        let late_payment =
            terminal_order_preserved && !payment_status_is_success(payment_status.as_deref());
        // A second genuinely-different succeeded attempt on an already-paid
        // order (multi-attempt payment) is preserved by the terminal guard;
        // record it so ops can reconcile the duplicate collection.
        let duplicate_payment =
            terminal_order_preserved && payment_status_is_success(payment_status.as_deref());

        let update = sqlx::query(
            r#"
            UPDATE commerce_order
            SET status = CASE
                    WHEN LOWER(COALESCE(status, '')) IN (
                        'fulfilled', 'completed', 'finished', 'cancelled', 'canceled', 'closed', 'expired'
                    )
                        THEN status
                    ELSE 'paid'
                END,
                payment_status = 'success',
                paid_at = COALESCE(NULLIF(paid_at, ''), $1),
                updated_at = $1
            WHERE tenant_id = CAST($2 AS TEXT)
              AND ((organization_id = CAST($3 AS TEXT)) OR (organization_id IS NULL AND $3 IS NULL) OR (organization_id = '0' AND $3 IS NULL))
              AND owner_user_id = CAST($4 AS TEXT)
              AND id = CAST($5 AS TEXT)
            "#,
        )
        .bind(paid_at)
        .bind(&attempt.tenant_id)
        .bind(attempt.organization_id.as_deref())
        .bind(&attempt.owner_user_id)
        .bind(&attempt.order_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            map_sqlx_store_error("failed to mark owner order payment succeeded", error)
        })?;
        if update.rows_affected() != 1 {
            return Err(CommerceServiceError::storage(
                "owner order payment-state update did not affect exactly one row",
            ));
        }

        if late_payment {
            insert_late_payment_event_postgres(&mut tx, attempt, &order_status, paid_at).await?;
        }
        if duplicate_payment {
            insert_duplicate_payment_event_postgres(&mut tx, attempt, &order_status, paid_at)
                .await?;
        }

        tx.commit().await.map_err(|error| {
            map_sqlx_store_error(
                "failed to commit owner order payment-state transaction",
                error,
            )
        })?;
        Ok(OwnerOrderPaymentStateOutcome {
            order_status: if terminal_order_preserved {
                order_status
            } else {
                "paid".to_owned()
            },
            terminal_order_preserved,
        })
    }
}

impl PostgresCommerceOrderStore {
    /// Marks the owner order cancelled after a provider payment failure or
    /// closure webhook. Terminal orders (already paid, fulfilled, cancelled,
    /// closed, expired) are preserved so a late failure callback can never
    /// overwrite a confirmed success; the preserved outcome is reported to the
    /// caller for audit.
    pub async fn mark_owner_order_payment_failed(
        &self,
        attempt: &OrderPaymentSettlementAttempt,
        failure_status: &str,
    ) -> Result<OwnerOrderPaymentStateOutcome, CommerceServiceError> {
        let mut tx = self.pool().begin().await.map_err(|error| {
            map_sqlx_store_error(
                "failed to begin owner order payment-failure transaction",
                error,
            )
        })?;

        let row = sqlx::query(
            r#"
            SELECT status, payment_status
            FROM commerce_order
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
              AND owner_user_id = CAST($3 AS TEXT)
              AND id = CAST($4 AS TEXT)
            LIMIT 1
            FOR UPDATE
            "#,
        )
        .bind(&attempt.tenant_id)
        .bind(attempt.organization_id.as_deref())
        .bind(&attempt.owner_user_id)
        .bind(&attempt.order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| map_sqlx_store_error("failed to load owner order payment-failure state", error))?;

        let Some(row) = row else {
            return Err(CommerceServiceError::not_found(
                "owner order was not found for payment failure settlement",
            ));
        };
        let order_status = row.try_get::<String, _>("status").unwrap_or_default();
        let payment_status = row
            .try_get::<Option<String>, _>("payment_status")
            .ok()
            .flatten();
        // A failure notify must never overwrite an order that already has
        // money collected (paid/fulfilled/completed/finished) or any terminal
        // state — including cross-provider/cross-intent cases where the order
        // was paid through a different attempt. Preserve and ack idempotently
        // instead of erroring into a PSP retry loop.
        let terminal_order_preserved = terminal_order_status(&order_status).is_some()
            || payment_status_is_success(payment_status.as_deref());
        if terminal_order_preserved {
            return Ok(OwnerOrderPaymentStateOutcome {
                order_status,
                terminal_order_preserved: true,
            });
        }

        let update = sqlx::query(
            r#"
            UPDATE commerce_order
            SET status = 'cancelled',
                payment_status = $1,
                cancelled_at = COALESCE(NULLIF(cancelled_at, ''), $2),
                updated_at = $2
            WHERE tenant_id = CAST($3 AS TEXT)
              AND ((organization_id = CAST($4 AS TEXT)) OR (organization_id IS NULL AND $4 IS NULL) OR (organization_id = '0' AND $4 IS NULL))
              AND owner_user_id = CAST($5 AS TEXT)
              AND id = CAST($6 AS TEXT)
              AND LOWER(COALESCE(status, '')) NOT IN (
                  'fulfilled', 'completed', 'finished', 'cancelled', 'canceled', 'closed', 'expired'
              )
            "#,
        )
        .bind(failure_status)
        .bind(paid_at_now())
        .bind(&attempt.tenant_id)
        .bind(attempt.organization_id.as_deref())
        .bind(&attempt.owner_user_id)
        .bind(&attempt.order_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            map_sqlx_store_error("failed to mark owner order payment failed", error)
        })?;
        if update.rows_affected() != 1 {
            return Err(CommerceServiceError::storage(
                "owner order payment-failure update did not affect exactly one row",
            ));
        }

        tx.commit().await.map_err(|error| {
            map_sqlx_store_error(
                "failed to commit owner order payment-failure transaction",
                error,
            )
        })?;
        Ok(OwnerOrderPaymentStateOutcome {
            order_status: "cancelled".to_owned(),
            terminal_order_preserved: false,
        })
    }
}

impl sdkwork_order_service::RefundNotifyStatePort for PostgresCommerceOrderStore {
    fn mark_owner_order_refund_status<'a>(
        &'a self,
        attempt: &'a sdkwork_order_service::OrderPaymentSettlementAttempt,
        refund_status: &'a str,
    ) -> sdkwork_order_service::OwnerOrderRefundStateFuture<'a> {
        let pool = self.pool().clone();
        let tenant_id = attempt.tenant_id.clone();
        let organization_id = attempt.organization_id.clone();
        let owner_user_id = attempt.owner_user_id.clone();
        let order_id = attempt.order_id.clone();
        let refund_status = refund_status.to_owned();
        Box::pin(async move {
            mark_owner_order_refund_status_postgres(
                &pool,
                &tenant_id,
                organization_id.as_deref(),
                &owner_user_id,
                &order_id,
                &refund_status,
            )
            .await
        })
    }
}

impl OwnerOrderPaymentStatePort for PostgresCommerceOrderStore {
    fn mark_owner_order_payment_succeeded<'a>(
        &'a self,
        attempt: &'a OrderPaymentSettlementAttempt,
        paid_at: &'a str,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, OwnerOrderPaymentStateOutcome> {
        Box::pin(async move {
            PostgresCommerceOrderStore::mark_owner_order_payment_succeeded(self, attempt, paid_at)
                .await
        })
    }
}

/// Advances `commerce_order.refund_status` from a refund notification. The
/// state machine is terminal-safe: `refunded` is final and is never
/// overwritten by a late `refund_failed`/`refunding` notification; replays of
/// the same target state are idempotent.
async fn mark_owner_order_refund_status_postgres(
    pool: &PgPool,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    order_id: &str,
    refund_status: &str,
) -> Result<sdkwork_order_service::OwnerOrderRefundStateOutcome, CommerceServiceError> {
    let mut tx = pool.begin().await.map_err(|error| {
        map_sqlx_store_error(
            "failed to begin owner order refund-state transaction",
            error,
        )
    })?;
    let row = sqlx::query(
        r#"
        SELECT refund_status
        FROM commerce_order
        WHERE tenant_id = CAST($1 AS TEXT)
          AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
          AND owner_user_id = CAST($3 AS TEXT)
          AND id = CAST($4 AS TEXT)
        LIMIT 1
        FOR UPDATE
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(owner_user_id)
    .bind(order_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|error| map_sqlx_store_error("failed to load owner order refund state", error))?;
    let Some(row) = row else {
        return Err(CommerceServiceError::not_found(
            "owner order was not found for refund state settlement",
        ));
    };
    let current_status = row
        .try_get::<Option<String>, _>("refund_status")
        .ok()
        .flatten()
        .unwrap_or_else(|| "none".to_owned());
    let terminal_preserved = current_status == "refunded" && refund_status != "refunded";
    if terminal_preserved || current_status == refund_status {
        return Ok(sdkwork_order_service::OwnerOrderRefundStateOutcome {
            refund_status: current_status,
            terminal_preserved,
        });
    }

    sqlx::query(
        r#"
        UPDATE commerce_order
        SET refund_status = $1,
            updated_at = $2
        WHERE tenant_id = CAST($3 AS TEXT)
          AND ((organization_id = CAST($4 AS TEXT)) OR (organization_id IS NULL AND $4 IS NULL) OR (organization_id = '0' AND $4 IS NULL))
          AND owner_user_id = CAST($5 AS TEXT)
          AND id = CAST($6 AS TEXT)
        "#,
    )
    .bind(refund_status)
    .bind(paid_at_now())
    .bind(tenant_id)
    .bind(organization_id)
    .bind(owner_user_id)
    .bind(order_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| {
        map_sqlx_store_error("failed to mark owner order refund status", error)
    })?;
    tx.commit().await.map_err(|error| {
        map_sqlx_store_error(
            "failed to commit owner order refund-state transaction",
            error,
        )
    })?;
    Ok(sdkwork_order_service::OwnerOrderRefundStateOutcome {
        refund_status: refund_status.to_owned(),
        terminal_preserved: false,
    })
}

fn paid_at_now() -> String {
    // Unix-seconds text matching `current_command_timestamp` used across the
    // order store; order timestamp columns are TEXT and the settlement
    // queries compare them lexicographically-consistently.
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    format!("{seconds}")
}

fn required_paid_at(paid_at: &str) -> Result<&str, CommerceServiceError> {
    let paid_at = paid_at.trim();
    if paid_at.is_empty() {
        return Err(CommerceServiceError::validation("paid_at is required"));
    }
    Ok(paid_at)
}

fn payment_status_is_success(payment_status: Option<&str>) -> bool {
    payment_status.is_some_and(|status| {
        matches!(
            status.trim().to_ascii_lowercase().as_str(),
            "success" | "succeeded" | "paid"
        )
    })
}

fn terminal_order_status(order_status: &str) -> Option<&'static str> {
    match order_status.trim().to_ascii_lowercase().as_str() {
        "paid" => Some("paid"),
        "fulfilled" | "completed" | "finished" => Some("fulfilled"),
        "cancelled" | "canceled" => Some("cancelled"),
        "closed" => Some("closed"),
        "expired" => Some("expired"),
        _ => None,
    }
}

const DUPLICATE_PAYMENT_EVENT_TYPE: &str = "duplicate_payment_success";
const DUPLICATE_PAYMENT_REASON_CODE: &str = "duplicate_payment";

async fn insert_duplicate_payment_event_postgres(
    tx: &mut Transaction<'_, Postgres>,
    attempt: &OrderPaymentSettlementAttempt,
    order_status: &str,
    paid_at: &str,
) -> Result<(), CommerceServiceError> {
    let audit = OrderLifecycleAuditInput {
        tenant_id: attempt.tenant_id.clone(),
        organization_id: attempt.organization_id.clone(),
        order_id: attempt.order_id.clone(),
        event_type: DUPLICATE_PAYMENT_EVENT_TYPE,
        from_status: order_status.to_owned(),
        to_status: "paid",
        actor_type: "payment",
        actor_id: None,
        reason_code: Some(DUPLICATE_PAYMENT_REASON_CODE.to_owned()),
        reason_message: Some(
            "another payment attempt succeeded on an already-paid order; duplicate collection needs reconciliation"
                .to_owned(),
        ),
        request_no: format!("duplicate-payment-{}", attempt.order_id),
        idempotency_key: format!("order-duplicate-payment:{}", attempt.order_id),
        now: paid_at.to_owned(),
    };
    insert_order_event_postgres(tx, &audit).await
}

async fn insert_late_payment_event_postgres(
    tx: &mut Transaction<'_, Postgres>,
    attempt: &OrderPaymentSettlementAttempt,
    order_status: &str,
    paid_at: &str,
) -> Result<(), CommerceServiceError> {
    let audit = late_payment_audit_input(attempt, order_status, paid_at)?;
    insert_order_event_postgres(tx, &audit).await
}

fn late_payment_audit_input(
    attempt: &OrderPaymentSettlementAttempt,
    order_status: &str,
    paid_at: &str,
) -> Result<OrderLifecycleAuditInput, CommerceServiceError> {
    let to_status = terminal_order_status(order_status).ok_or_else(|| {
        CommerceServiceError::invalid_state(
            "late-payment audit requires a cancelled, closed, or expired order",
        )
    })?;
    Ok(OrderLifecycleAuditInput {
        tenant_id: attempt.tenant_id.clone(),
        organization_id: attempt.organization_id.clone(),
        order_id: attempt.order_id.clone(),
        event_type: LATE_PAYMENT_EVENT_TYPE,
        from_status: order_status.to_owned(),
        to_status,
        actor_type: "payment",
        actor_id: None,
        reason_code: Some(LATE_PAYMENT_REASON_CODE.to_owned()),
        reason_message: Some(
            "payment succeeded after terminal order state; order status was preserved".to_owned(),
        ),
        request_no: format!("late-payment-{}", attempt.order_id),
        idempotency_key: format!("order-late-payment:{}", attempt.order_id),
        now: paid_at.to_owned(),
    })
}
