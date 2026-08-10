use sdkwork_contract_service::CommerceServiceError;
use sqlx::{Postgres, Transaction};

/// `organization_id` column (DATABASE_SPEC DB090, same convention as
/// postgres_order.rs / postgres_recharge.rs).
const PLATFORM_ORGANIZATION_SCOPE_SENTINEL: &str = "0";

fn normalize_organization_scope(organization_id: Option<&str>) -> String {
    organization_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(PLATFORM_ORGANIZATION_SCOPE_SENTINEL)
        .to_owned()
}

#[derive(Clone, Debug)]
pub struct OrderLifecycleAuditInput {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub order_id: String,
    pub event_type: &'static str,
    pub from_status: String,
    pub to_status: &'static str,
    pub actor_type: &'static str,
    pub actor_id: Option<String>,
    pub reason_code: Option<String>,
    pub reason_message: Option<String>,
    pub request_no: String,
    pub idempotency_key: String,
    pub now: String,
}

pub fn order_cancel_idempotency_key(order_id: &str) -> String {
    format!("order-cancel:{order_id}")
}

pub fn order_close_idempotency_key(order_id: &str) -> String {
    format!("order-close:{order_id}")
}

pub fn stable_order_storage_id(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|part| {
            part.chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                        character
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("-")
}

pub async fn insert_order_event_postgres(
    tx: &mut Transaction<'_, Postgres>,
    input: &OrderLifecycleAuditInput,
) -> Result<(), CommerceServiceError> {
    let event_id = stable_order_storage_id(&[
        "order-event",
        &input.tenant_id,
        &input.order_id,
        input.event_type,
        &input.idempotency_key,
    ]);
    let event_no = format!("OE-{}-{}", input.event_type, input.request_no);
    sqlx::query(
        r#"
        INSERT INTO commerce_order_event
            (id, tenant_id, organization_id, event_no, order_id, event_type,
             from_status, to_status, actor_type, actor_id, reason_code, message,
             payload_json, request_id, idempotency_key, created_at)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, '{}', $13, $14, $15)
        "#,
    )
    // commerce_order_event.organization_id is NOT NULL with the platform
    // sentinel default (DATABASE_SPEC DB090); org-less orders store '0'.
    .bind(&normalize_organization_scope(
        input.organization_id.as_deref(),
    ))
    .bind(&event_no)
    .bind(&input.order_id)
    .bind(input.event_type)
    .bind(&input.from_status)
    .bind(input.to_status)
    .bind(input.actor_type)
    .bind(input.actor_id.as_deref())
    .bind(input.reason_code.as_deref())
    .bind(input.reason_message.as_deref())
    .bind(&input.request_no)
    .bind(&input.idempotency_key)
    .bind(&input.now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert order event", error))?;
    Ok(())
}

pub async fn insert_order_cancellation_postgres(
    tx: &mut Transaction<'_, Postgres>,
    input: &OrderLifecycleAuditInput,
) -> Result<(), CommerceServiceError> {
    let cancellation_id = stable_order_storage_id(&[
        "order-cancellation",
        &input.tenant_id,
        &input.order_id,
        &input.idempotency_key,
    ]);
    sqlx::query(
        r#"
        INSERT INTO commerce_order_cancellation
            (id, tenant_id, order_id, status, reason_code, reason_message, created_at)
        VALUES
            ($1, $2, $3, 'completed', $4, $5, $6)
        "#,
    )
    .bind(&cancellation_id)
    .bind(&input.tenant_id)
    .bind(&input.order_id)
    .bind(input.reason_code.as_deref().unwrap_or("user_cancel"))
    .bind(input.reason_message.as_deref())
    .bind(&input.now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert order cancellation", error))?;
    Ok(())
}

fn store_error(message: &str, error: impl std::fmt::Display) -> CommerceServiceError {
    crate::sql_store_error::map_sql_store_error(message, error)
}

#[cfg(test)]
mod tests {
    use super::normalize_organization_scope;

    #[test]
    fn organization_scope_missing_uses_platform_sentinel() {
        assert_eq!(normalize_organization_scope(None), "0");
    }

    #[test]
    fn organization_scope_blank_uses_platform_sentinel() {
        assert_eq!(normalize_organization_scope(Some("")), "0");
        assert_eq!(normalize_organization_scope(Some("   ")), "0");
    }

    #[test]
    fn organization_scope_present_is_trimmed() {
        assert_eq!(normalize_organization_scope(Some(" org-123 ")), "org-123");
    }
}
