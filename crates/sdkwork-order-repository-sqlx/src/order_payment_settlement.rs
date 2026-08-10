use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    OrderPaymentSettlementAttempt, OwnerOrderPaymentConfirmationFuture,
    OwnerOrderPaymentStateOutcome, OwnerOrderPaymentStatePort,
};
use sqlx::{Postgres, Row, Transaction};

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
        "cancelled" | "canceled" => Some("cancelled"),
        "closed" => Some("closed"),
        "expired" => Some("expired"),
        _ => None,
    }
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
