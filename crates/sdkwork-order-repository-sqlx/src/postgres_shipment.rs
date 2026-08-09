#![allow(clippy::too_many_arguments)]

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    CreateShipmentPackageCommand, ShipmentDetailQuery, ShipmentManagementDetailQuery,
    ShipmentManagementListPage, ShipmentManagementListQuery, ShipmentPackageListQuery,
    ShipmentPackageManagementListQuery, ShipmentPackagePage, ShipmentPackageView,
    ShipmentTrackingEventListQuery, ShipmentTrackingEventPage, ShipmentTrackingEventView,
    ShipmentView, UpdateShipmentPackageCommand,
};
use sqlx::Row;

use crate::postgres_order::PostgresCommerceOrderStore;

impl PostgresCommerceOrderStore {
    pub async fn retrieve_owner_shipment(
        &self,
        query: ShipmentDetailQuery,
    ) -> Result<Option<ShipmentView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT s.id, s.shipment_no, s.fulfillment_id, s.carrier_code, s.tracking_no, s.status
            FROM commerce_shipment s
            INNER JOIN commerce_fulfillment_order f
                ON f.tenant_id = s.tenant_id
               AND f.id = s.fulfillment_id
            INNER JOIN commerce_order o
                ON o.tenant_id = f.tenant_id
               AND o.id = f.order_id
            WHERE s.tenant_id = CAST($1 AS TEXT)
              AND ((s.organization_id = CAST($2 AS TEXT)) OR (s.organization_id IS NULL AND $3 IS NULL) OR (s.organization_id = '0' AND $3 IS NULL))
              AND o.owner_user_id = CAST($4 AS TEXT)
              AND s.id = CAST($5 AS TEXT)
            LIMIT 1
           "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(&query.shipment_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to retrieve owner shipment", error))?;

        row.map(map_shipment_row).transpose()
    }

    /// 列出物流包裹（owner 域）。
    ///
    /// 先校验 shipment 归属再分页查询包裹，`COUNT(*) OVER()` 在一次往返中
    /// 给出无条件总数，配合 `LIMIT`/`OFFSET` 实现真正的数据库分页。
    pub async fn list_owner_shipment_packages(
        &self,
        query: ShipmentPackageListQuery,
    ) -> Result<ShipmentPackagePage, CommerceServiceError> {
        if self
            .retrieve_owner_shipment(ShipmentDetailQuery {
                tenant_id: query.tenant_id.clone(),
                organization_id: query.organization_id.clone(),
                owner_user_id: query.owner_user_id.clone(),
                shipment_id: query.shipment_id.clone(),
            })
            .await?
            .is_none()
        {
            return Err(CommerceServiceError::not_found("shipment was not found"));
        }

        let rows = sqlx::query(
            r#"
            SELECT
                id,
                shipment_id,
                package_no,
                package_type,
                tracking_no,
                status,
                COUNT(*) OVER() AS total_count
            FROM commerce_shipment_package
            WHERE tenant_id = CAST($1 AS TEXT)
              AND shipment_id = CAST($2 AS TEXT)
            ORDER BY created_at ASC, id ASC
            LIMIT $3 OFFSET $4
           "#,
        )
        .bind(&query.tenant_id)
        .bind(&query.shipment_id)
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to list shipment packages", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);

        let items = rows
            .into_iter()
            .map(|row| ShipmentPackageView {
                package_id: string_cell(&row, "id"),
                shipment_id: string_cell(&row, "shipment_id"),
                package_no: string_cell(&row, "package_no"),
                package_type: string_cell(&row, "package_type"),
                tracking_no: optional_string_cell(&row, "tracking_no"),
                status: string_cell(&row, "status"),
            })
            .collect();

        Ok(ShipmentPackagePage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    /// 列出物流轨迹事件（owner 域）。
    ///
    /// 先校验 shipment 归属再分页查询轨迹事件，`COUNT(*) OVER()` 在一次往返中
    /// 给出无条件总数，配合 `LIMIT`/`OFFSET` 实现真正的数据库分页。
    pub async fn list_owner_shipment_tracking_events(
        &self,
        query: ShipmentTrackingEventListQuery,
    ) -> Result<ShipmentTrackingEventPage, CommerceServiceError> {
        if self
            .retrieve_owner_shipment(ShipmentDetailQuery {
                tenant_id: query.tenant_id.clone(),
                organization_id: query.organization_id.clone(),
                owner_user_id: query.owner_user_id.clone(),
                shipment_id: query.shipment_id.clone(),
            })
            .await?
            .is_none()
        {
            return Err(CommerceServiceError::not_found("shipment was not found"));
        }

        let rows = sqlx::query(
            r#"
            SELECT
                id,
                shipment_id,
                tracking_event_no,
                event_type,
                event_status,
                event_time,
                location_text,
                COUNT(*) OVER() AS total_count
            FROM commerce_shipment_tracking_event
            WHERE tenant_id = CAST($1 AS TEXT)
              AND shipment_id = CAST($2 AS TEXT)
            ORDER BY event_time ASC, id ASC
            LIMIT $3 OFFSET $4
           "#,
        )
        .bind(&query.tenant_id)
        .bind(&query.shipment_id)
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to list shipment tracking events", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);

        let items = rows
            .into_iter()
            .map(|row| ShipmentTrackingEventView {
                event_id: string_cell(&row, "id"),
                shipment_id: string_cell(&row, "shipment_id"),
                tracking_event_no: string_cell(&row, "tracking_event_no"),
                event_type: string_cell(&row, "event_type"),
                event_status: optional_string_cell(&row, "event_status"),
                event_time: string_cell(&row, "event_time"),
                location_text: optional_string_cell(&row, "location_text"),
            })
            .collect();

        Ok(ShipmentTrackingEventPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn list_management_shipments(
        &self,
        query: ShipmentManagementListQuery,
    ) -> Result<ShipmentManagementListPage, CommerceServiceError> {
        let rows = sqlx::query(
            r#"
            SELECT s.id, s.shipment_no, s.fulfillment_id, s.carrier_code, s.tracking_no, s.status,
                   COUNT(*) OVER() AS total_count
            FROM commerce_shipment s
            LEFT JOIN commerce_fulfillment_order f
                ON f.tenant_id = s.tenant_id
               AND f.id = s.fulfillment_id
            WHERE s.tenant_id = CAST($1 AS TEXT)
              AND ((s.organization_id = CAST($2 AS TEXT)) OR (s.organization_id IS NULL AND $3 IS NULL) OR (s.organization_id = '0' AND $3 IS NULL))
              AND ($4 IS NULL OR f.order_id = CAST($5 AS TEXT))
              AND ($6 IS NULL OR s.fulfillment_id = CAST($7 AS TEXT))
              AND ($8 IS NULL OR s.status = CAST($9 AS TEXT))
            ORDER BY s.created_at DESC, s.id DESC
            LIMIT $10 OFFSET $11
           "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(query.order_id.as_deref())
        .bind(query.order_id.as_deref())
        .bind(query.fulfillment_id.as_deref())
        .bind(query.fulfillment_id.as_deref())
        .bind(query.status.as_deref())
        .bind(query.status.as_deref())
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to list management shipments", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
        let items = rows
            .into_iter()
            .map(map_shipment_row)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ShipmentManagementListPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn retrieve_management_shipment(
        &self,
        query: ShipmentManagementDetailQuery,
    ) -> Result<Option<ShipmentView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT s.id, s.shipment_no, s.fulfillment_id, s.carrier_code, s.tracking_no, s.status
            FROM commerce_shipment s
            WHERE s.tenant_id = CAST($1 AS TEXT)
              AND ((s.organization_id = CAST($2 AS TEXT)) OR (s.organization_id IS NULL AND $3 IS NULL) OR (s.organization_id = '0' AND $3 IS NULL))
              AND s.id = CAST($4 AS TEXT)
            LIMIT 1
           "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(&query.shipment_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to retrieve management shipment", error))?;

        row.map(map_shipment_row).transpose()
    }

    pub async fn list_management_shipment_packages(
        &self,
        query: ShipmentPackageManagementListQuery,
    ) -> Result<ShipmentPackagePage, CommerceServiceError> {
        if self
            .retrieve_management_shipment(ShipmentManagementDetailQuery {
                tenant_id: query.tenant_id.clone(),
                organization_id: query.organization_id.clone(),
                shipment_id: query.shipment_id.clone(),
            })
            .await?
            .is_none()
        {
            return Err(CommerceServiceError::not_found("shipment was not found"));
        }

        let rows = sqlx::query(
            r#"
            SELECT
                id,
                shipment_id,
                package_no,
                package_type,
                tracking_no,
                status,
                COUNT(*) OVER() AS total_count
            FROM commerce_shipment_package
            WHERE tenant_id = CAST($1 AS TEXT)
              AND shipment_id = CAST($2 AS TEXT)
            ORDER BY created_at ASC, id ASC
            LIMIT $3 OFFSET $4
           "#,
        )
        .bind(&query.tenant_id)
        .bind(&query.shipment_id)
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to list management shipment packages", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
        let items = rows
            .into_iter()
            .map(|row| ShipmentPackageView {
                package_id: string_cell(&row, "id"),
                shipment_id: string_cell(&row, "shipment_id"),
                package_no: string_cell(&row, "package_no"),
                package_type: string_cell(&row, "package_type"),
                tracking_no: optional_string_cell(&row, "tracking_no"),
                status: string_cell(&row, "status"),
            })
            .collect();

        Ok(ShipmentPackagePage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn create_management_shipment_package(
        &self,
        command: CreateShipmentPackageCommand,
    ) -> Result<ShipmentPackageView, CommerceServiceError> {
        if self
            .retrieve_management_shipment(ShipmentManagementDetailQuery {
                tenant_id: command.tenant_id.clone(),
                organization_id: command.organization_id.clone(),
                shipment_id: command.shipment_id.clone(),
            })
            .await?
            .is_none()
        {
            return Err(CommerceServiceError::not_found("shipment was not found"));
        }

        let package_id = shipment_package_storage_id(&command);
        if let Some(existing) = self
            .load_management_shipment_package(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.shipment_id,
                &package_id,
            )
            .await?
        {
            return Ok(existing);
        }

        let now = current_timestamp_string();
        let package_no = command
            .package_no
            .clone()
            .unwrap_or_else(|| format!("PKG-{}", command.request_no));
        let status = command
            .status
            .clone()
            .unwrap_or_else(|| "created".to_owned());

        sqlx::query(
            r#"
            INSERT INTO commerce_shipment_package
                (id, tenant_id, organization_id, shipment_id, package_no, package_type,
                 tracking_no, status, created_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(&package_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.shipment_id)
        .bind(&package_no)
        .bind(&command.package_type)
        .bind(command.tracking_no.as_deref())
        .bind(&status)
        .bind(&now)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to create shipment package", error))?;

        self.load_management_shipment_package(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.shipment_id,
            &package_id,
        )
        .await?
        .ok_or_else(|| CommerceServiceError::storage("created shipment package was not found"))
    }

    pub async fn update_management_shipment_package(
        &self,
        command: UpdateShipmentPackageCommand,
    ) -> Result<ShipmentPackageView, CommerceServiceError> {
        if self
            .load_management_shipment_package(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.shipment_id,
                &command.package_id,
            )
            .await?
            .is_none()
        {
            return Err(CommerceServiceError::not_found(
                "shipment package was not found",
            ));
        }

        sqlx::query(
            r#"
            UPDATE commerce_shipment_package
            SET package_type = COALESCE($1, package_type),
                tracking_no = COALESCE($2, tracking_no),
                status = COALESCE($3, status)
            WHERE tenant_id = CAST($4 AS TEXT)
              AND shipment_id = CAST($5 AS TEXT)
              AND id = CAST($6 AS TEXT)
            "#,
        )
        .bind(command.package_type.as_deref())
        .bind(command.tracking_no.as_deref())
        .bind(command.status.as_deref())
        .bind(&command.tenant_id)
        .bind(&command.shipment_id)
        .bind(&command.package_id)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to update shipment package", error))?;

        self.load_management_shipment_package(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.shipment_id,
            &command.package_id,
        )
        .await?
        .ok_or_else(|| CommerceServiceError::not_found("shipment package was not found"))
    }

    async fn load_management_shipment_package(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        shipment_id: &str,
        package_id: &str,
    ) -> Result<Option<ShipmentPackageView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT id, shipment_id, package_no, package_type, tracking_no, status
            FROM commerce_shipment_package
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3 IS NULL) OR (organization_id = '0' AND $3 IS NULL))
              AND shipment_id = CAST($4 AS TEXT)
              AND id = CAST($5 AS TEXT)
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(shipment_id)
        .bind(package_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to load shipment package", error))?;

        Ok(row.map(|row| ShipmentPackageView {
            package_id: string_cell(&row, "id"),
            shipment_id: string_cell(&row, "shipment_id"),
            package_no: string_cell(&row, "package_no"),
            package_type: string_cell(&row, "package_type"),
            tracking_no: optional_string_cell(&row, "tracking_no"),
            status: string_cell(&row, "status"),
        }))
    }
}

fn map_shipment_row(row: sqlx::postgres::PgRow) -> Result<ShipmentView, CommerceServiceError> {
    Ok(ShipmentView {
        shipment_id: string_cell(&row, "id"),
        shipment_no: string_cell(&row, "shipment_no"),
        fulfillment_id: string_cell(&row, "fulfillment_id"),
        carrier_code: string_cell(&row, "carrier_code"),
        tracking_no: optional_string_cell(&row, "tracking_no"),
        status: string_cell(&row, "status"),
    })
}

fn store_error(message: &str, error: impl std::fmt::Display) -> CommerceServiceError {
    crate::sql_store_error::map_sql_store_error(message, error)
}

fn shipment_package_storage_id(command: &CreateShipmentPackageCommand) -> String {
    stable_storage_id(&[
        "shipment-package",
        &command.tenant_id,
        &command.shipment_id,
        &command.idempotency_key,
    ])
}

fn stable_storage_id(parts: &[&str]) -> String {
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

fn current_timestamp_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{seconds}")
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}
