//! Postgres settlement store for refund-request webhook post-processing.
//!
//! Resolves the `commerce_order_refund_request` behind a PSP refund
//! notification through the deterministic linkage the refund execution
//! wrote (`commerce_refund.idempotency_key =
//! 'refund-request:payment-refund:' || refund_request.id`), and terminalizes
//! the request with an in-flight guard so webhook replays and races with the
//! synchronous execution path collapse to no-ops.

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    RefundRequestSettlementContext, RefundRequestSettlementFuture, RefundRequestSettlementPort,
};
use sqlx::{PgPool, Row};

use crate::sql_store_error::map_sql_store_error;

#[derive(Clone)]
pub struct PostgresRefundRequestSettlementStore {
    pool: PgPool,
}

impl PostgresRefundRequestSettlementStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl RefundRequestSettlementPort for PostgresRefundRequestSettlementStore {
    fn load_refund_request_for_settlement<'a>(
        &'a self,
        tenant_id: &'a str,
        refund_no: &'a str,
    ) -> RefundRequestSettlementFuture<'a, Option<RefundRequestSettlementContext>> {
        Box::pin(async move {
            let row = sqlx::query(
                r#"
                SELECT r.id AS request_id, r.tenant_id, r.organization_id, r.owner_user_id,
                       r.original_order_id, r.target_asset, r.amount, r.currency_code,
                       r.request_no, r.status, r.account_effect_reference_id
                FROM commerce_order_refund_request r
                JOIN commerce_refund c
                  ON c.tenant_id = r.tenant_id
                 AND c.idempotency_key = 'refund-request:payment-refund:' || r.id
                WHERE c.tenant_id = CAST($1 AS TEXT)
                  AND c.refund_no = CAST($2 AS TEXT)
                  AND c.deleted_at IS NULL
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(refund_no)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| {
                map_sql_store_error("failed to load refund request for settlement", error)
            })?;
            let Some(row) = row else {
                return Ok(None);
            };
            Ok(Some(RefundRequestSettlementContext {
                request_id: row.try_get("request_id").map_err(|error| {
                    CommerceServiceError::storage(format!("refund request id: {error}"))
                })?,
                tenant_id: row.try_get("tenant_id").map_err(|error| {
                    CommerceServiceError::storage(format!("refund request tenant: {error}"))
                })?,
                organization_id: row
                    .try_get::<Option<String>, _>("organization_id")
                    .ok()
                    .flatten()
                    .filter(|value| !value.is_empty() && value != "0"),
                owner_user_id: row.try_get("owner_user_id").map_err(|error| {
                    CommerceServiceError::storage(format!("refund request owner: {error}"))
                })?,
                original_order_id: row.try_get("original_order_id").map_err(|error| {
                    CommerceServiceError::storage(format!("refund request order: {error}"))
                })?,
                target_asset: row.try_get("target_asset").map_err(|error| {
                    CommerceServiceError::storage(format!("refund request asset: {error}"))
                })?,
                amount: row.try_get("amount").map_err(|error| {
                    CommerceServiceError::storage(format!("refund request amount: {error}"))
                })?,
                currency_code: row.try_get("currency_code").map_err(|error| {
                    CommerceServiceError::storage(format!("refund request currency: {error}"))
                })?,
                request_no: row.try_get("request_no").map_err(|error| {
                    CommerceServiceError::storage(format!("refund request no: {error}"))
                })?,
                status: row.try_get("status").map_err(|error| {
                    CommerceServiceError::storage(format!("refund request status: {error}"))
                })?,
                hold_id: row
                    .try_get::<Option<String>, _>("account_effect_reference_id")
                    .ok()
                    .flatten()
                    .filter(|value| !value.is_empty()),
            }))
        })
    }

    fn mark_refund_request_settled<'a>(
        &'a self,
        tenant_id: &'a str,
        request_id: &'a str,
        target_status: &'a str,
        provider_reference_id: Option<&'a str>,
    ) -> RefundRequestSettlementFuture<'a, bool> {
        Box::pin(async move {
            let result = sqlx::query(
                r#"
                UPDATE commerce_order_refund_request
                SET status = CAST($2 AS TEXT),
                    updated_at = to_char(CURRENT_TIMESTAMP AT TIME ZONE 'UTC',
                                         'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'),
                    provider_reference_id = COALESCE(CAST($3 AS TEXT), provider_reference_id)
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND id = CAST($4 AS TEXT)
                  AND status = 'provider_refund_processing'
                "#,
            )
            .bind(tenant_id)
            .bind(target_status)
            .bind(provider_reference_id)
            .bind(request_id)
            .execute(&self.pool)
            .await
            .map_err(|error| map_sql_store_error("failed to mark refund request settled", error))?;
            Ok(result.rows_affected() == 1)
        })
    }
}
