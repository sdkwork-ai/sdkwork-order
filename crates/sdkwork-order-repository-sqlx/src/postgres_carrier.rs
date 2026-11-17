use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    CarrierListQuery, CarrierPage, CarrierView, CreateCarrierCommand, UpdateCarrierCommand,
};
use sqlx::Row;

use crate::postgres_order::PostgresCommerceOrderStore;

impl PostgresCommerceOrderStore {
    pub async fn list_management_carriers(
        &self,
        query: CarrierListQuery,
    ) -> Result<CarrierPage, CommerceServiceError> {
        let rows = sqlx::query(
            r#"
            SELECT id, carrier_code, carrier_name, tracking_url_template, status, sort_order,
                   COUNT(*) OVER() AS total_count
            FROM commerce_carrier
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id = '0' AND $3 IS NULL))
              AND ($4 IS NULL OR status = CAST($4 AS TEXT))
            ORDER BY sort_order ASC, carrier_code ASC, id ASC
            LIMIT $5 OFFSET $6
           "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(query.status.as_deref())
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to list carriers", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
        let items = rows.iter().map(map_carrier_row).collect();

        Ok(CarrierPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    /// Creates a carrier once per tenant/organization/code.
    ///
    /// The storage id is derived from the natural key, so a retried create with
    /// the same code is a no-op and returns the existing carrier instead of
    /// failing on the unique index.
    pub async fn create_management_carrier(
        &self,
        command: CreateCarrierCommand,
    ) -> Result<CarrierView, CommerceServiceError> {
        let carrier_id = carrier_storage_id(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.carrier_code,
        );
        let organization_id = command
            .organization_id
            .clone()
            .unwrap_or_else(|| "0".to_owned());
        let now = current_timestamp_string();

        sqlx::query(
            r#"
            INSERT INTO commerce_carrier
                (id, tenant_id, organization_id, carrier_code, carrier_name,
                 tracking_url_template, status, sort_order, created_at, updated_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(&carrier_id)
        .bind(&command.tenant_id)
        .bind(&organization_id)
        .bind(&command.carrier_code)
        .bind(&command.carrier_name)
        .bind(command.tracking_url_template.as_deref())
        .bind(&command.status)
        .bind(command.sort_order)
        .bind(&now)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to create carrier", error))?;

        self.load_management_carrier(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &carrier_id,
        )
        .await?
        .ok_or_else(|| CommerceServiceError::storage("created carrier was not found"))
    }

    pub async fn update_management_carrier(
        &self,
        command: UpdateCarrierCommand,
    ) -> Result<CarrierView, CommerceServiceError> {
        let organization_id = command.organization_id.clone().unwrap_or_default();
        let now = current_timestamp_string();

        let outcome = sqlx::query(
            r#"
            UPDATE commerce_carrier
            SET carrier_name = COALESCE($1, carrier_name),
                tracking_url_template = COALESCE($2, tracking_url_template),
                status = COALESCE($3, status),
                sort_order = COALESCE($4, sort_order),
                updated_at = $5
            WHERE tenant_id = CAST($6 AS TEXT)
              AND id = CAST($7 AS TEXT)
              AND ($8 = '' OR organization_id = CAST($8 AS TEXT))
            "#,
        )
        .bind(command.carrier_name.as_deref())
        .bind(command.tracking_url_template.as_deref())
        .bind(command.status.as_deref())
        .bind(command.sort_order)
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(&command.carrier_id)
        .bind(&organization_id)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to update carrier", error))?;

        if outcome.rows_affected() == 0 {
            return Err(CommerceServiceError::not_found("carrier was not found"));
        }

        self.load_management_carrier(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.carrier_id,
        )
        .await?
        .ok_or_else(|| CommerceServiceError::storage("updated carrier was not found"))
    }

    async fn load_management_carrier(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        carrier_id: &str,
    ) -> Result<Option<CarrierView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT id, carrier_code, carrier_name, tracking_url_template, status, sort_order
            FROM commerce_carrier
            WHERE tenant_id = CAST($1 AS TEXT)
              AND id = CAST($2 AS TEXT)
              AND ((organization_id = CAST($3 AS TEXT)) OR (organization_id = '0' AND $4 IS NULL))
            LIMIT 1
           "#,
        )
        .bind(tenant_id)
        .bind(carrier_id)
        .bind(organization_id)
        .bind(organization_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to retrieve carrier", error))?;

        Ok(row.map(|row| map_carrier_row(&row)))
    }
}

fn map_carrier_row(row: &sqlx::postgres::PgRow) -> CarrierView {
    CarrierView {
        carrier_id: string_cell(row, "id"),
        carrier_code: string_cell(row, "carrier_code"),
        carrier_name: string_cell(row, "carrier_name"),
        tracking_url_template: optional_string_cell(row, "tracking_url_template"),
        status: string_cell(row, "status"),
        sort_order: row.try_get::<i64, _>("sort_order").unwrap_or(0),
    }
}

fn carrier_storage_id(tenant_id: &str, organization_id: Option<&str>, carrier_code: &str) -> String {
    let organization_id = organization_id.unwrap_or("0");
    [tenant_id, organization_id, carrier_code]
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

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn store_error(message: &str, error: impl std::fmt::Display) -> CommerceServiceError {
    CommerceServiceError::storage(format!("{message}: {error}"))
}

fn current_timestamp_string() -> String {
    crate::store_clock::now_canonical()
}
