use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    FulfillmentDetailQuery, FulfillmentListPage, FulfillmentListQuery, FulfillmentView,
};
use sqlx::Row;

use crate::postgres_order::PostgresCommerceOrderStore;

impl PostgresCommerceOrderStore {
    /// 列出履约（owner 域）。
    ///
    /// 通过 `INNER JOIN commerce_order` 限定 `owner_user_id` 归属，避免越权读取。
    /// `COUNT(*) OVER()` 在一次往返中给出无条件总数，配合 `LIMIT`/`OFFSET`
    /// 实现真正的数据库分页（避免内存分页导致的 OOM 与性能问题）。
    pub async fn list_owner_fulfillments(
        &self,
        query: FulfillmentListQuery,
    ) -> Result<FulfillmentListPage, CommerceServiceError> {
        let rows = sqlx::query(
            r#"
            SELECT
                f.id,
                f.fulfillment_no,
                f.order_id,
                f.fulfillment_type,
                f.status,
                COUNT(*) OVER() AS total_count
            FROM commerce_fulfillment_order f
            INNER JOIN commerce_order o
                ON o.tenant_id = f.tenant_id
               AND o.id = f.order_id
            WHERE f.tenant_id = CAST($1 AS TEXT)
              AND ((f.organization_id = CAST($2 AS TEXT)) OR (f.organization_id IS NULL AND $3 IS NULL) OR (f.organization_id = '0' AND $3 IS NULL))
              AND o.owner_user_id = CAST($4 AS TEXT)
              AND ($5 IS NULL OR f.order_id = CAST($6 AS TEXT))
              AND ($7 IS NULL OR LOWER(f.status) = LOWER(CAST($8 AS TEXT)))
            ORDER BY f.created_at DESC, f.id DESC
            LIMIT $9 OFFSET $10
            "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(query.order_id.as_deref())
        .bind(query.order_id.as_deref())
        .bind(query.status.as_deref())
        .bind(query.status.as_deref())
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to list owner fulfillments", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);

        let items = rows
            .into_iter()
            .map(|row| FulfillmentView {
                fulfillment_id: string_cell(&row, "id"),
                fulfillment_no: string_cell(&row, "fulfillment_no"),
                order_id: string_cell(&row, "order_id"),
                fulfillment_type: string_cell(&row, "fulfillment_type"),
                status: string_cell(&row, "status"),
            })
            .collect();

        Ok(FulfillmentListPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn retrieve_owner_fulfillment(
        &self,
        query: FulfillmentDetailQuery,
    ) -> Result<Option<FulfillmentView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT f.id, f.fulfillment_no, f.order_id, f.fulfillment_type, f.status
            FROM commerce_fulfillment_order f
            INNER JOIN commerce_order o
                ON o.tenant_id = f.tenant_id
               AND o.id = f.order_id
            WHERE f.tenant_id = CAST($1 AS TEXT)
              AND ((f.organization_id = CAST($2 AS TEXT)) OR (f.organization_id IS NULL AND $3 IS NULL) OR (f.organization_id = '0' AND $3 IS NULL))
              AND o.owner_user_id = CAST($4 AS TEXT)
              AND f.id = CAST($5 AS TEXT)
            LIMIT 1
            "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(&query.fulfillment_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to retrieve owner fulfillment", error))?;

        Ok(row.map(|row| FulfillmentView {
            fulfillment_id: string_cell(&row, "id"),
            fulfillment_no: string_cell(&row, "fulfillment_no"),
            order_id: string_cell(&row, "order_id"),
            fulfillment_type: string_cell(&row, "fulfillment_type"),
            status: string_cell(&row, "status"),
        }))
    }
}

fn store_error(message: &str, error: impl std::fmt::Display) -> CommerceServiceError {
    crate::sql_store_error::map_sql_store_error(message, error)
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}
