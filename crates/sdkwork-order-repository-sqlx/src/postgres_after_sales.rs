#![allow(clippy::too_many_arguments)]

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

/// Platform rows persist the sentinel organization scope (`"0"`) so tenant
/// (personal) sessions never write NULL into the NOT NULL `organization_id`
/// column (mirrors `postgres_order.rs`).
fn normalize_organization_scope(organization_id: Option<&str>) -> String {
    organization_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("0")
        .to_owned()
}

use sdkwork_order_service::{
    AfterSalesEventListQuery, AfterSalesEventPage, AfterSalesEventView,
    AfterSalesManagementDetailQuery, AfterSalesManagementListQuery, AfterSalesRequestDetailQuery,
    AfterSalesRequestListQuery, AfterSalesRequestPage, AfterSalesRequestView,
    AfterSalesReturnShipmentListQuery, AfterSalesReturnShipmentPage, AfterSalesReturnShipmentView,
    CreateAfterSalesRequestCommand, CreateAfterSalesReturnShipmentCommand, OrderOwnerDetailQuery,
    ReviewAfterSalesRequestCommand, UpdateAfterSalesRequestCommand,
};
use sqlx::{Postgres, Row, Transaction};

use crate::money_amount::commerce_money_stored;
use crate::postgres_order::PostgresCommerceOrderStore;

impl PostgresCommerceOrderStore {
    pub async fn create_after_sales_request(
        &self,
        command: CreateAfterSalesRequestCommand,
    ) -> Result<AfterSalesRequestView, CommerceServiceError> {
        if let Some(existing) = self
            .find_after_sales_request_by_idempotency(&command)
            .await?
        {
            return Ok(existing);
        }

        let detail_query = OrderOwnerDetailQuery::new(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
            &command.order_id,
        )?;
        let Some(detail) = self.retrieve_owner_order(detail_query).await? else {
            return Err(CommerceServiceError::not_found("order was not found"));
        };
        validate_order_eligible_for_after_sales(&detail.summary.status)?;

        let mut tx = self.pool().begin().await.map_err(|error| {
            store_error("failed to begin after sales request transaction", error)
        })?;
        let now = current_timestamp_string();
        let request_id = after_sales_request_id(&command);
        let after_sales_no = format!("AS-{}", command.request_no);
        let requested_amount = command
            .requested_amount
            .clone()
            .unwrap_or_else(|| detail.summary.total_amount.as_str().to_owned());
        let currency_code = command
            .currency_code
            .clone()
            .unwrap_or_else(|| detail.summary.currency_code.clone());

        sqlx::query(
            r#"
            INSERT INTO commerce_after_sales_request
                (id, tenant_id, organization_id, after_sales_no, order_id, owner_user_id,
                 after_sales_type, status, refund_status, return_status, exchange_status,
                 reason_code, description, requested_amount, approved_amount, currency_code,
                 requested_by_type, requested_by, request_no, idempotency_key, created_at, updated_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, 'submitted', 'none', 'none', 'none', $8, $9, $10, '0', $11,
                 'buyer', $12, $13, $14, $15, $16)
           "#,
        )
        .bind(&request_id)
        .bind(&command.tenant_id)
        .bind(normalize_organization_scope(command.organization_id.as_deref()))
        .bind(&after_sales_no)
        .bind(&command.order_id)
        .bind(&command.owner_user_id)
        .bind(&command.after_sales_type)
        .bind(&command.reason_code)
        .bind(command.description.as_deref())
        .bind(&requested_amount)
        .bind(&currency_code)
        .bind(&command.owner_user_id)
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert after sales request", error))?;

        // 写入行项明细（部分退款 / 换货场景）。
        for item in &command.items {
            let item_id = stable_storage_id(&[
                "after-sales-item",
                &command.tenant_id,
                &request_id,
                &item.order_item_id,
            ]);
            let item_amount = item
                .requested_amount
                .clone()
                // Canonical integer smallest-unit string (after-sales readers
                // parse amounts with CommerceMoney::new).
                .unwrap_or_else(|| "0".to_string());
            sqlx::query(
                r#"
                INSERT INTO commerce_after_sales_request_item
                    (id, tenant_id, organization_id, after_sales_id, order_item_id,
                     quantity, requested_amount, currency_code, created_at)
                VALUES
                    ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               "#,
            )
            .bind(&item_id)
            .bind(&command.tenant_id)
            .bind(normalize_organization_scope(command.organization_id.as_deref()))
            .bind(&request_id)
            .bind(&item.order_item_id)
            .bind(item.quantity)
            .bind(&item_amount)
            .bind(&currency_code)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|error| store_error("failed to insert after sales request item", error))?;
        }

        insert_after_sales_event(
            &mut tx,
            &command.tenant_id,
            command.organization_id.as_deref(),
            &request_id,
            "created",
            "submitted",
            &command.request_no,
            &command.idempotency_key,
            &now,
        )
        .await?;

        tx.commit().await.map_err(|error| {
            store_error("failed to commit after sales request transaction", error)
        })?;

        Ok(AfterSalesRequestView {
            after_sales_request_id: request_id,
            after_sales_no,
            order_id: command.order_id,
            after_sales_type: command.after_sales_type,
            reason_code: command.reason_code,
            requested_amount: CommerceMoney::new(&requested_amount)
                .map_err(CommerceServiceError::storage)?,
            currency_code,
            status: "submitted".to_owned(),
        })
    }

    pub async fn retrieve_after_sales_request(
        &self,
        query: AfterSalesRequestDetailQuery,
    ) -> Result<Option<AfterSalesRequestView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT id, after_sales_no, order_id, after_sales_type, reason_code,
                   CAST(requested_amount AS TEXT) AS requested_amount, currency_code, status
            FROM commerce_after_sales_request
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3 IS NULL) OR (organization_id = '0' AND $3 IS NULL))
              AND owner_user_id = CAST($4 AS TEXT)
              AND id = CAST($5 AS TEXT)
            LIMIT 1
           "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(&query.after_sales_request_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to retrieve after sales request", error))?;

        row.map(map_after_sales_request_row).transpose()
    }

    pub async fn update_after_sales_request(
        &self,
        command: UpdateAfterSalesRequestCommand,
    ) -> Result<AfterSalesRequestView, CommerceServiceError> {
        let existing = self
            .retrieve_after_sales_request(AfterSalesRequestDetailQuery {
                after_sales_request_id: command.after_sales_request_id.clone(),
                organization_id: command.organization_id.clone(),
                owner_user_id: command.owner_user_id.clone(),
                tenant_id: command.tenant_id.clone(),
            })
            .await?
            .ok_or_else(|| CommerceServiceError::not_found("after sales request was not found"))?;

        if let Some(status) = command.status.as_deref() {
            validate_owner_after_sales_status_transition(&existing.status, status)?;
        }

        let mut tx = self.pool().begin().await.map_err(|error| {
            store_error("failed to begin after sales update transaction", error)
        })?;
        let now = current_timestamp_string();
        let next_status = command
            .status
            .as_deref()
            .unwrap_or(existing.status.as_str());

        sqlx::query(
            r#"
            UPDATE commerce_after_sales_request
            SET status = $1,
                reason_code = COALESCE($2, reason_code),
                description = COALESCE($3, description),
                requested_amount = COALESCE($4, requested_amount),
                currency_code = COALESCE($5, currency_code),
                updated_at = $6
            WHERE tenant_id = CAST($7 AS TEXT)
              AND ((organization_id = CAST($8 AS TEXT)) OR (organization_id IS NULL AND $9 IS NULL) OR (organization_id = '0' AND $9 IS NULL))
              AND owner_user_id = CAST($10 AS TEXT)
              AND id = CAST($11 AS TEXT)
            "#,
        )
        .bind(next_status)
        .bind(command.reason_code.as_deref())
        .bind(command.description.as_deref())
        .bind(command.requested_amount.as_deref())
        .bind(command.currency_code.as_deref())
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&command.after_sales_request_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to update after sales request", error))?;

        if command.status.is_some() {
            insert_after_sales_event(
                &mut tx,
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.after_sales_request_id,
                "updated",
                next_status,
                &command.request_no,
                &command.idempotency_key,
                &now,
            )
            .await?;
        }

        tx.commit().await.map_err(|error| {
            store_error("failed to commit after sales update transaction", error)
        })?;

        self.retrieve_after_sales_request(AfterSalesRequestDetailQuery {
            after_sales_request_id: command.after_sales_request_id,
            organization_id: command.organization_id,
            owner_user_id: command.owner_user_id,
            tenant_id: command.tenant_id,
        })
        .await?
        .ok_or_else(|| CommerceServiceError::not_found("after sales request was not found"))
    }

    pub async fn list_after_sales_requests(
        &self,
        query: AfterSalesRequestListQuery,
    ) -> Result<AfterSalesRequestPage, CommerceServiceError> {
        let rows = sqlx::query(
            r#"
            SELECT id, after_sales_no, order_id, after_sales_type, reason_code,
                   CAST(requested_amount AS TEXT) AS requested_amount, currency_code, status,
                   COUNT(*) OVER() AS total_count
            FROM commerce_after_sales_request
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3 IS NULL) OR (organization_id = '0' AND $3 IS NULL))
              AND owner_user_id = CAST($4 AS TEXT)
              AND ($5 IS NULL OR order_id = CAST($6 AS TEXT))
              AND ($7 IS NULL OR after_sales_type = CAST($8 AS TEXT))
              AND ($9 IS NULL OR status = CAST($10 AS TEXT))
              AND ($11 IS NULL OR id = CAST($12 AS TEXT))
            ORDER BY created_at DESC, id DESC
            LIMIT $13 OFFSET $14
           "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(query.order_id.as_deref())
        .bind(query.order_id.as_deref())
        .bind(query.after_sales_type.as_deref())
        .bind(query.after_sales_type.as_deref())
        .bind(query.status.as_deref())
        .bind(query.status.as_deref())
        .bind(query.after_sales_request_id.as_deref())
        .bind(query.after_sales_request_id.as_deref())
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to list after sales requests", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
        let items = rows
            .into_iter()
            .map(map_after_sales_request_row)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(AfterSalesRequestPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn list_management_after_sales_requests(
        &self,
        query: AfterSalesManagementListQuery,
    ) -> Result<AfterSalesRequestPage, CommerceServiceError> {
        let rows = sqlx::query(
            r#"
            SELECT id, after_sales_no, order_id, after_sales_type, reason_code,
                   CAST(requested_amount AS TEXT) AS requested_amount, currency_code, status,
                   COUNT(*) OVER() AS total_count
            FROM commerce_after_sales_request
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3 IS NULL) OR (organization_id = '0' AND $3 IS NULL))
              AND ($4 IS NULL OR order_id = CAST($5 AS TEXT))
              AND ($6 IS NULL OR after_sales_type = CAST($7 AS TEXT))
              AND ($8 IS NULL OR status = CAST($9 AS TEXT))
              AND ($10 IS NULL OR id = CAST($11 AS TEXT))
            ORDER BY created_at DESC, id DESC
            LIMIT $12 OFFSET $13
           "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(query.order_id.as_deref())
        .bind(query.order_id.as_deref())
        .bind(query.after_sales_type.as_deref())
        .bind(query.after_sales_type.as_deref())
        .bind(query.status.as_deref())
        .bind(query.status.as_deref())
        .bind(query.after_sales_request_id.as_deref())
        .bind(query.after_sales_request_id.as_deref())
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to list management after sales requests", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
        let items = rows
            .into_iter()
            .map(map_after_sales_request_row)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(AfterSalesRequestPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn retrieve_management_after_sales_request(
        &self,
        query: AfterSalesManagementDetailQuery,
    ) -> Result<Option<AfterSalesRequestView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT id, after_sales_no, order_id, after_sales_type, reason_code,
                   CAST(requested_amount AS TEXT) AS requested_amount, currency_code, status
            FROM commerce_after_sales_request
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3 IS NULL) OR (organization_id = '0' AND $3 IS NULL))
              AND id = CAST($4 AS TEXT)
            LIMIT 1
           "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(&query.after_sales_request_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to retrieve management after sales request", error))?;

        row.map(map_after_sales_request_row).transpose()
    }

    pub async fn review_after_sales_request(
        &self,
        command: ReviewAfterSalesRequestCommand,
    ) -> Result<AfterSalesRequestView, CommerceServiceError> {
        let existing = self
            .retrieve_management_after_sales_request(AfterSalesManagementDetailQuery {
                after_sales_request_id: command.after_sales_request_id.clone(),
                organization_id: command.organization_id.clone(),
                tenant_id: command.tenant_id.clone(),
            })
            .await?
            .ok_or_else(|| CommerceServiceError::not_found("after sales request was not found"))?;

        let next_status = command.resolved_status();
        validate_management_after_sales_status_transition(&existing.status, &next_status)?;

        let mut tx = self.pool().begin().await.map_err(|error| {
            store_error("failed to begin after sales review transaction", error)
        })?;
        let now = current_timestamp_string();

        sqlx::query(
            r#"
            UPDATE commerce_after_sales_request
            SET status = $1,
                reason_code = COALESCE($2, reason_code),
                approved_amount = COALESCE($3, approved_amount),
                refund_status = COALESCE($4, refund_status),
                return_status = COALESCE($5, return_status),
                exchange_status = COALESCE($6, exchange_status),
                description = COALESCE($7, description),
                updated_at = $8
            WHERE tenant_id = CAST($9 AS TEXT)
              AND ((organization_id = CAST($10 AS TEXT)) OR (organization_id IS NULL AND $11 IS NULL) OR (organization_id = '0' AND $11 IS NULL))
              AND id = CAST($12 AS TEXT)
            "#,
        )
        .bind(&next_status)
        .bind(command.reason_code.as_deref())
        .bind(command.approved_amount.as_deref())
        .bind(command.refund_status.as_deref())
        .bind(command.return_status.as_deref())
        .bind(command.exchange_status.as_deref())
        .bind(command.review_comment.as_deref())
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(command.organization_id.as_deref())
        .bind(&command.after_sales_request_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to review after sales request", error))?;

        insert_after_sales_event(
            &mut tx,
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.after_sales_request_id,
            "reviewed",
            &next_status,
            &command.request_no,
            &command.idempotency_key,
            &now,
        )
        .await?;

        tx.commit().await.map_err(|error| {
            store_error("failed to commit after sales review transaction", error)
        })?;

        self.retrieve_management_after_sales_request(AfterSalesManagementDetailQuery {
            after_sales_request_id: command.after_sales_request_id,
            organization_id: command.organization_id,
            tenant_id: command.tenant_id,
        })
        .await?
        .ok_or_else(|| CommerceServiceError::not_found("after sales request was not found"))
    }

    pub async fn list_after_sales_events(
        &self,
        query: AfterSalesEventListQuery,
    ) -> Result<AfterSalesEventPage, CommerceServiceError> {
        let exists = self
            .retrieve_after_sales_request(AfterSalesRequestDetailQuery {
                after_sales_request_id: query.after_sales_request_id.clone(),
                organization_id: query.organization_id.clone(),
                owner_user_id: query.owner_user_id.clone(),
                tenant_id: query.tenant_id.clone(),
            })
            .await?;
        if exists.is_none() {
            return Err(CommerceServiceError::not_found(
                "after sales request was not found",
            ));
        }

        let rows = sqlx::query(
            r#"
            SELECT id, after_sales_id, event_no, event_type, to_status,
                   COUNT(*) OVER() AS total_count
            FROM commerce_after_sales_event
            WHERE tenant_id = CAST($1 AS TEXT)
              AND after_sales_id = CAST($2 AS TEXT)
            ORDER BY created_at ASC, id ASC
            LIMIT $3 OFFSET $4
           "#,
        )
        .bind(&query.tenant_id)
        .bind(&query.after_sales_request_id)
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to list after sales events", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
        let items = rows
            .into_iter()
            .map(|row| AfterSalesEventView {
                event_id: string_cell(&row, "id"),
                after_sales_request_id: string_cell(&row, "after_sales_id"),
                event_no: string_cell(&row, "event_no"),
                event_type: string_cell(&row, "event_type"),
                to_status: string_cell(&row, "to_status"),
            })
            .collect();

        Ok(AfterSalesEventPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn list_after_sales_return_shipments(
        &self,
        query: AfterSalesReturnShipmentListQuery,
    ) -> Result<AfterSalesReturnShipmentPage, CommerceServiceError> {
        let exists = self
            .retrieve_after_sales_request(AfterSalesRequestDetailQuery {
                after_sales_request_id: query.after_sales_request_id.clone(),
                organization_id: query.organization_id.clone(),
                owner_user_id: query.owner_user_id.clone(),
                tenant_id: query.tenant_id.clone(),
            })
            .await?;
        if exists.is_none() {
            return Err(CommerceServiceError::not_found(
                "after sales request was not found",
            ));
        }

        let rows = sqlx::query(
            r#"
            SELECT id, after_sales_id, return_shipment_no, tracking_no, status,
                   COUNT(*) OVER() AS total_count
            FROM commerce_after_sales_return_shipment
            WHERE tenant_id = CAST($1 AS TEXT)
              AND after_sales_id = CAST($2 AS TEXT)
              AND ($3 IS NULL OR status = CAST($4 AS TEXT))
            ORDER BY created_at DESC, id DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(&query.tenant_id)
        .bind(&query.after_sales_request_id)
        .bind(query.status.as_deref())
        .bind(query.status.as_deref())
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to list after sales return shipments", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
        let items = rows
            .into_iter()
            .map(|row| AfterSalesReturnShipmentView {
                return_shipment_id: string_cell(&row, "id"),
                after_sales_request_id: string_cell(&row, "after_sales_id"),
                return_shipment_no: string_cell(&row, "return_shipment_no"),
                status: string_cell(&row, "status"),
                tracking_no: optional_string_cell(&row, "tracking_no"),
            })
            .collect();

        Ok(AfterSalesReturnShipmentPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn create_after_sales_return_shipment(
        &self,
        command: CreateAfterSalesReturnShipmentCommand,
    ) -> Result<AfterSalesReturnShipmentView, CommerceServiceError> {
        if let Some(existing) = self
            .find_after_sales_return_shipment_by_idempotency(&command)
            .await?
        {
            return Ok(existing);
        }

        let request = self
            .retrieve_after_sales_request(AfterSalesRequestDetailQuery {
                after_sales_request_id: command.after_sales_request_id.clone(),
                organization_id: command.organization_id.clone(),
                owner_user_id: command.owner_user_id.clone(),
                tenant_id: command.tenant_id.clone(),
            })
            .await?
            .ok_or_else(|| CommerceServiceError::not_found("after sales request was not found"))?;
        let _ = request;

        let mut tx = self.pool().begin().await.map_err(|error| {
            store_error(
                "failed to begin after sales return shipment transaction",
                error,
            )
        })?;
        let now = current_timestamp_string();
        let shipment_id = after_sales_return_shipment_id(&command);
        let return_shipment_no = format!("RS-{}", command.request_no);
        let tracking_no = command
            .tracking_no
            .clone()
            .unwrap_or_else(|| return_shipment_no.clone());

        sqlx::query(
            r#"
            INSERT INTO commerce_after_sales_return_shipment
                (id, tenant_id, organization_id, after_sales_id, return_shipment_no,
                 carrier_code, tracking_no, status, request_no, idempotency_key, created_at, updated_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, 'submitted', $8, $9, $10, $11)
           "#,
        )
        .bind(&shipment_id)
        .bind(&command.tenant_id)
        .bind(normalize_organization_scope(command.organization_id.as_deref()))
        .bind(&command.after_sales_request_id)
        .bind(&return_shipment_no)
        .bind(command.carrier_code.as_deref())
        .bind(&tracking_no)
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert after sales return shipment", error))?;

        sqlx::query(
            r#"
            UPDATE commerce_after_sales_request
            SET return_status = 'submitted', updated_at = $1
            WHERE tenant_id = CAST($2 AS TEXT)
              AND id = CAST($3 AS TEXT)
           "#,
        )
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(&command.after_sales_request_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to update after sales return status", error))?;

        insert_after_sales_event(
            &mut tx,
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.after_sales_request_id,
            "return_shipment_created",
            "submitted",
            &command.request_no,
            &command.idempotency_key,
            &now,
        )
        .await?;

        tx.commit().await.map_err(|error| {
            store_error(
                "failed to commit after sales return shipment transaction",
                error,
            )
        })?;

        Ok(AfterSalesReturnShipmentView {
            return_shipment_id: shipment_id,
            after_sales_request_id: command.after_sales_request_id,
            return_shipment_no,
            tracking_no: Some(tracking_no),
            status: "submitted".to_owned(),
        })
    }

    async fn find_after_sales_request_by_idempotency(
        &self,
        command: &CreateAfterSalesRequestCommand,
    ) -> Result<Option<AfterSalesRequestView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT id, after_sales_no, order_id, after_sales_type, reason_code,
                   CAST(requested_amount AS TEXT) AS requested_amount, currency_code, status
            FROM commerce_after_sales_request
            WHERE tenant_id = CAST($1 AS TEXT)
              AND order_id = CAST($2 AS TEXT)
              AND idempotency_key = CAST($3 AS TEXT)
            LIMIT 1
           "#,
        )
        .bind(&command.tenant_id)
        .bind(&command.order_id)
        .bind(&command.idempotency_key)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to load after sales idempotency replay", error))?;

        row.map(map_after_sales_request_row).transpose()
    }

    async fn find_after_sales_return_shipment_by_idempotency(
        &self,
        command: &CreateAfterSalesReturnShipmentCommand,
    ) -> Result<Option<AfterSalesReturnShipmentView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT id, after_sales_id, return_shipment_no, tracking_no, status
            FROM commerce_after_sales_return_shipment
            WHERE tenant_id = CAST($1 AS TEXT)
              AND after_sales_id = CAST($2 AS TEXT)
              AND idempotency_key = CAST($3 AS TEXT)
            LIMIT 1
           "#,
        )
        .bind(&command.tenant_id)
        .bind(&command.after_sales_request_id)
        .bind(&command.idempotency_key)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| {
            store_error(
                "failed to load after sales return shipment idempotency replay",
                error,
            )
        })?;

        row.map(|row| {
            Ok(AfterSalesReturnShipmentView {
                return_shipment_id: string_cell(&row, "id"),
                after_sales_request_id: string_cell(&row, "after_sales_id"),
                return_shipment_no: string_cell(&row, "return_shipment_no"),
                tracking_no: optional_string_cell(&row, "tracking_no"),
                status: string_cell(&row, "status"),
            })
        })
        .transpose()
    }
}

async fn insert_after_sales_event(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    organization_id: Option<&str>,
    after_sales_id: &str,
    event_type: &str,
    to_status: &str,
    request_no: &str,
    idempotency_key: &str,
    now: &str,
) -> Result<(), CommerceServiceError> {
    let event_id = stable_storage_id(&[
        "after-sales-event",
        tenant_id,
        after_sales_id,
        event_type,
        idempotency_key,
    ]);
    let event_no = format!("ASE-{event_type}-{request_no}");
    sqlx::query(
        r#"
        INSERT INTO commerce_after_sales_event
            (id, tenant_id, organization_id, after_sales_id, event_no, event_type,
             from_status, to_status, actor_type, actor_id, request_id, idempotency_key, created_at)
        VALUES
            ($1, $2, $3, $4, $5, $6, NULL, $7, 'buyer', NULL, $8, $9, $10)
       "#,
    )
    .bind(&event_id)
    .bind(tenant_id)
    .bind(organization_id)
    .bind(after_sales_id)
    .bind(&event_no)
    .bind(event_type)
    .bind(to_status)
    .bind(request_no)
    .bind(idempotency_key)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert after sales event", error))?;
    Ok(())
}

fn map_after_sales_request_row(
    row: sqlx::postgres::PgRow,
) -> Result<AfterSalesRequestView, CommerceServiceError> {
    let after_sales_request_id = string_cell(&row, "id");
    // Stored money columns may carry legacy major-unit decimals; never let
    // a single legacy row fail the whole after-sales list.
    let requested_amount = commerce_money_stored(
        &string_cell(&row, "requested_amount"),
        "requested_amount",
        &after_sales_request_id,
    )?;
    Ok(AfterSalesRequestView {
        after_sales_request_id,
        after_sales_no: string_cell(&row, "after_sales_no"),
        order_id: string_cell(&row, "order_id"),
        after_sales_type: string_cell(&row, "after_sales_type"),
        reason_code: string_cell(&row, "reason_code"),
        requested_amount,
        currency_code: string_cell(&row, "currency_code"),
        status: string_cell(&row, "status"),
    })
}

fn after_sales_request_id(command: &CreateAfterSalesRequestCommand) -> String {
    stable_storage_id(&[
        "after-sales-request",
        &command.tenant_id,
        &command.order_id,
        &command.idempotency_key,
    ])
}

fn validate_order_eligible_for_after_sales(order_status: &str) -> Result<(), CommerceServiceError> {
    let status = order_status.trim().to_ascii_lowercase();
    if matches!(
        status.as_str(),
        "pending_payment" | "unpaid" | "wait_pay" | "draft" | "cancelled" | "closed" | "expired"
    ) {
        return Err(CommerceServiceError::conflict(
            "after sales requests are not allowed for orders in the current status",
        ));
    }
    Ok(())
}

fn validate_owner_after_sales_status_transition(
    current_status: &str,
    next_status: &str,
) -> Result<(), CommerceServiceError> {
    let current = current_status.trim().to_ascii_lowercase();
    let next = next_status.trim().to_ascii_lowercase();
    if current == next {
        return Ok(());
    }
    let allowed = matches!(
        (current.as_str(), next.as_str()),
        ("submitted", "cancelled") | ("submitted", "withdrawn")
    );
    if allowed {
        Ok(())
    } else {
        Err(CommerceServiceError::conflict(
            "after sales request status transition is not allowed",
        ))
    }
}

fn validate_management_after_sales_status_transition(
    current_status: &str,
    next_status: &str,
) -> Result<(), CommerceServiceError> {
    let current = current_status.trim().to_ascii_lowercase();
    let next = next_status.trim().to_ascii_lowercase();
    if current == next {
        return Ok(());
    }
    let allowed = matches!(
        (current.as_str(), next.as_str()),
        ("submitted", "approved")
            | ("submitted", "rejected")
            | ("submitted", "cancelled")
            | ("approved", "processing")
            | ("processing", "completed")
            | ("approved", "rejected")
    );
    if allowed {
        Ok(())
    } else {
        Err(CommerceServiceError::conflict(
            "after sales management status transition is not allowed",
        ))
    }
}

fn after_sales_return_shipment_id(command: &CreateAfterSalesReturnShipmentCommand) -> String {
    stable_storage_id(&[
        "after-sales-return",
        &command.tenant_id,
        &command.after_sales_request_id,
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

fn store_error(message: &str, error: impl std::fmt::Display) -> CommerceServiceError {
    crate::sql_store_error::map_sql_store_error(message, error)
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
