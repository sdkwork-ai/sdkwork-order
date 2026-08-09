#![allow(clippy::too_many_arguments)]

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    checkout_quote_request_hash, checkout_session_request_hash, CheckoutQuoteView,
    CheckoutSessionDetailQuery, CheckoutSessionView, CreateCheckoutQuoteCommand,
    CreateCheckoutSessionCommand,
};
use sqlx::{Postgres, Row, Transaction};

use crate::{
    money_amount::{commerce_money, multiply_money_amount, sum_money_amounts},
    postgres_order::PostgresCommerceOrderStore,
};

const CHECKOUT_SESSION_CREATE_SCOPE: &str = "checkout.sessions.create";
const CHECKOUT_QUOTE_CREATE_SCOPE: &str = "checkout.sessions.quotes.create";

#[derive(Debug, Clone)]
struct ResolvedCheckoutLine {
    sku_id: String,
    product_id: Option<String>,
    _title: String,
    unit_price: String,
    quantity: i64,
    line_total: String,
    sku_snapshot_json: String,
    fulfillment_type: String,
    shop_id: Option<String>,
}

impl PostgresCommerceOrderStore {
    pub async fn create_checkout_session(
        &self,
        command: CreateCheckoutSessionCommand,
    ) -> Result<CheckoutSessionView, CommerceServiceError> {
        if let Some(existing) = self.find_checkout_session_by_idempotency(&command).await? {
            return Ok(existing);
        }

        let mut tx =
            self.pool().begin().await.map_err(|error| {
                store_error("failed to begin checkout session transaction", error)
            })?;
        let now = current_timestamp_string();
        let request_hash = checkout_session_request_hash(&command);
        if let Some(row) = load_checkout_idempotency_row(
            &mut tx,
            &command.tenant_id,
            CHECKOUT_SESSION_CREATE_SCOPE,
            &command.idempotency_key,
        )
        .await?
        {
            if string_cell(&row, "request_hash") != request_hash {
                return Err(CommerceServiceError::conflict(
                    "idempotency key was used with a different checkout session request",
                ));
            }
            if string_cell(&row, "status") == "completed" {
                let session = replay_checkout_session(&row)?;
                tx.commit().await.map_err(|error| {
                    store_error("failed to commit checkout session replay", error)
                })?;
                return Ok(session);
            }
            refresh_checkout_idempotency_lock(
                &mut tx,
                &command.tenant_id,
                CHECKOUT_SESSION_CREATE_SCOPE,
                &command.idempotency_key,
                &now,
            )
            .await?;
        } else {
            insert_checkout_idempotency_lock(
                &mut tx,
                &command.tenant_id,
                command.organization_id.as_deref(),
                CHECKOUT_SESSION_CREATE_SCOPE,
                &command.idempotency_key,
                &request_hash,
                &checkout_idempotency_id(
                    &command.tenant_id,
                    CHECKOUT_SESSION_CREATE_SCOPE,
                    &command.idempotency_key,
                ),
                &now,
            )
            .await?;
        }

        let lines = resolve_checkout_lines(&mut tx, &command).await?;
        let original_amount = sum_money_amounts(lines.iter().map(|line| line.line_total.as_str()))?;
        let discount_amount = "0".to_owned();
        let payable_amount = original_amount.clone();
        let session_id = checkout_session_id(&command);
        let quote_id = checkout_quote_id(&command.tenant_id, &session_id, &command.request_no);
        let expires_at = checkout_expires_at(&now);

        insert_checkout_session(
            &mut tx,
            &command,
            &session_id,
            &original_amount,
            &expires_at,
            &now,
        )
        .await?;
        insert_checkout_lines(&mut tx, &command, &session_id, &lines, &now).await?;
        insert_checkout_quote(
            &mut tx,
            &command,
            &session_id,
            &quote_id,
            &original_amount,
            &discount_amount,
            &payable_amount,
            &expires_at,
            &now,
        )
        .await?;

        let session = CheckoutSessionView {
            checkout_session_id: session_id,
            currency_code: command.currency_code.clone(),
            discount_amount: commerce_money(&discount_amount)?,
            original_amount: commerce_money(&original_amount)?,
            payable_amount: commerce_money(&payable_amount)?,
            quote_id: Some(quote_id),
            status: "active".to_owned(),
        };
        complete_checkout_idempotency(
            &mut tx,
            &command.tenant_id,
            CHECKOUT_SESSION_CREATE_SCOPE,
            &command.idempotency_key,
            &session,
            &now,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit checkout session transaction", error))?;
        Ok(session)
    }

    pub async fn retrieve_checkout_session(
        &self,
        query: CheckoutSessionDetailQuery,
    ) -> Result<Option<CheckoutSessionView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT s.id,
                   s.status,
                   s.currency_code,
                   CAST(COALESCE(q.original_amount, '0') AS TEXT) AS original_amount,
                   CAST(COALESCE(q.discount_amount, '0') AS TEXT) AS discount_amount,
                   CAST(COALESCE(q.payable_amount, '0') AS TEXT) AS payable_amount,
                   q.id AS quote_id
            FROM commerce_checkout_session s
            LEFT JOIN commerce_checkout_quote q
              ON q.tenant_id = s.tenant_id
             AND q.checkout_session_id = s.id
             AND LOWER(COALESCE(q.quote_status, '')) IN ('active', 'quoted', 'ready')
            WHERE s.tenant_id = CAST($1 AS TEXT)
              AND ((s.organization_id = CAST($2 AS TEXT)) OR (s.organization_id IS NULL AND $3 IS NULL) OR (s.organization_id = '0' AND $3 IS NULL))
              AND s.owner_user_id = CAST($4 AS TEXT)
              AND s.id = CAST($5 AS TEXT)
            ORDER BY q.created_at DESC, q.id DESC
            LIMIT 1
           "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(&query.checkout_session_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to retrieve checkout session", error))?;

        row.map(|row| {
            Ok(CheckoutSessionView {
                checkout_session_id: string_cell(&row, "id"),
                currency_code: string_cell(&row, "currency_code"),
                discount_amount: commerce_money(&string_cell(&row, "discount_amount"))?,
                original_amount: commerce_money(&string_cell(&row, "original_amount"))?,
                payable_amount: commerce_money(&string_cell(&row, "payable_amount"))?,
                quote_id: optional_string_cell(&row, "quote_id"),
                status: string_cell(&row, "status"),
            })
        })
        .transpose()
    }

    pub async fn create_checkout_quote(
        &self,
        command: CreateCheckoutQuoteCommand,
    ) -> Result<CheckoutQuoteView, CommerceServiceError> {
        let mut tx =
            self.pool().begin().await.map_err(|error| {
                store_error("failed to begin checkout quote transaction", error)
            })?;
        let now = current_timestamp_string();
        let request_hash = checkout_quote_request_hash(&command);
        if let Some(row) = load_checkout_idempotency_row(
            &mut tx,
            &command.tenant_id,
            CHECKOUT_QUOTE_CREATE_SCOPE,
            &command.idempotency_key,
        )
        .await?
        {
            if string_cell(&row, "request_hash") != request_hash {
                return Err(CommerceServiceError::conflict(
                    "idempotency key was used with a different checkout quote request",
                ));
            }
            if string_cell(&row, "status") == "completed" {
                let quote = replay_checkout_quote(&row)?;
                tx.commit().await.map_err(|error| {
                    store_error("failed to commit checkout quote replay", error)
                })?;
                return Ok(quote);
            }
            refresh_checkout_idempotency_lock(
                &mut tx,
                &command.tenant_id,
                CHECKOUT_QUOTE_CREATE_SCOPE,
                &command.idempotency_key,
                &now,
            )
            .await?;
        } else {
            insert_checkout_idempotency_lock(
                &mut tx,
                &command.tenant_id,
                command.organization_id.as_deref(),
                CHECKOUT_QUOTE_CREATE_SCOPE,
                &command.idempotency_key,
                &request_hash,
                &checkout_idempotency_id(
                    &command.tenant_id,
                    CHECKOUT_QUOTE_CREATE_SCOPE,
                    &command.idempotency_key,
                ),
                &now,
            )
            .await?;
        }

        let session = load_checkout_session_for_quote(&mut tx, &command).await?;
        let lines = load_checkout_lines_for_quote(&mut tx, &command).await?;
        if lines.is_empty() {
            return Err(CommerceServiceError::conflict(
                "checkout session has no selected lines",
            ));
        }
        let original_amount = sum_money_amounts(lines.iter().map(|line| line.line_total.as_str()))?;
        let discount_amount = "0".to_owned();
        let payable_amount = original_amount.clone();
        let quote_id = checkout_quote_id(
            &command.tenant_id,
            &command.checkout_session_id,
            &format!("{}:{}", command.request_no, command.idempotency_key),
        );
        let currency_code = string_cell(&session, "currency_code");
        let expires_at = optional_string_cell(&session, "expires_at")
            .unwrap_or_else(|| checkout_expires_at(&now));

        insert_checkout_quote_for_command(
            &mut tx,
            &command,
            &quote_id,
            &currency_code,
            &original_amount,
            &discount_amount,
            &payable_amount,
            &expires_at,
            &now,
        )
        .await?;
        update_checkout_session_status(&mut tx, &command, "quoted", &now).await?;

        let quote = CheckoutQuoteView {
            checkout_session_id: command.checkout_session_id.clone(),
            currency_code,
            discount_amount: commerce_money(&discount_amount)?,
            original_amount: commerce_money(&original_amount)?,
            payable_amount: commerce_money(&payable_amount)?,
            quote_id,
        };
        complete_checkout_quote_idempotency(
            &mut tx,
            &command.tenant_id,
            &command.idempotency_key,
            &quote,
            &now,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit checkout quote transaction", error))?;
        Ok(quote)
    }

    async fn find_checkout_session_by_idempotency(
        &self,
        command: &CreateCheckoutSessionCommand,
    ) -> Result<Option<CheckoutSessionView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT s.id,
                   s.status,
                   s.currency_code,
                   CAST(COALESCE(q.original_amount, '0') AS TEXT) AS original_amount,
                   CAST(COALESCE(q.discount_amount, '0') AS TEXT) AS discount_amount,
                   CAST(COALESCE(q.payable_amount, '0') AS TEXT) AS payable_amount,
                   q.id AS quote_id
            FROM commerce_idempotency_key i
            JOIN commerce_checkout_session s
              ON s.tenant_id = i.tenant_id
             AND s.idempotency_key = i.idempotency_key
            LEFT JOIN commerce_checkout_quote q
              ON q.tenant_id = s.tenant_id
             AND q.checkout_session_id = s.id
            WHERE i.tenant_id = CAST($1 AS TEXT)
              AND i.scope = $2
              AND i.idempotency_key = $3
              AND i.status = 'completed'
            ORDER BY q.created_at DESC, q.id DESC
            LIMIT 1
           "#,
        )
        .bind(&command.tenant_id)
        .bind(CHECKOUT_SESSION_CREATE_SCOPE)
        .bind(&command.idempotency_key)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| {
            store_error("failed to load checkout session idempotency replay", error)
        })?;

        row.map(|row| {
            Ok(CheckoutSessionView {
                checkout_session_id: string_cell(&row, "id"),
                currency_code: string_cell(&row, "currency_code"),
                discount_amount: commerce_money(&string_cell(&row, "discount_amount"))?,
                original_amount: commerce_money(&string_cell(&row, "original_amount"))?,
                payable_amount: commerce_money(&string_cell(&row, "payable_amount"))?,
                quote_id: optional_string_cell(&row, "quote_id"),
                status: string_cell(&row, "status"),
            })
        })
        .transpose()
    }
}

async fn resolve_checkout_lines(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateCheckoutSessionCommand,
) -> Result<Vec<ResolvedCheckoutLine>, CommerceServiceError> {
    if let Some(physical) = command.physical_checkout.as_ref() {
        return physical
            .lines
            .iter()
            .map(|line| {
                if !line
                    .currency_code
                    .eq_ignore_ascii_case(&command.currency_code)
                {
                    return Err(CommerceServiceError::conflict(
                        "checkout SKU currency does not match checkout currency",
                    ));
                }
                if !line.fulfillment_type_is_physical() {
                    return Err(CommerceServiceError::conflict(
                        "checkout only accepts physical shipment SKUs",
                    ));
                }
                let unit_price = line.unit_price.as_str().to_owned();
                Ok(ResolvedCheckoutLine {
                    sku_id: line.sku_id.clone(),
                    product_id: Some(line.product_id.clone()),
                    _title: line.title.clone(),
                    unit_price: unit_price.clone(),
                    quantity: line.quantity,
                    line_total: multiply_money_amount(&unit_price, line.quantity)?,
                    sku_snapshot_json: line.sku_snapshot_json.clone(),
                    fulfillment_type: "physical_shipment".to_owned(),
                    shop_id: Some(line.shop_id.clone()),
                })
            })
            .collect();
    }
    let mut resolved = Vec::with_capacity(command.lines.len());
    for line in &command.lines {
        let row = sqlx::query(
            r#"
            SELECT id, spu_id, COALESCE(NULLIF(title, ''), name, id) AS title,
                   CAST(price_amount AS TEXT) AS price_amount, currency_code,
                   fulfillment_type, spec_json
            FROM commerce_product_sku
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3 IS NULL) OR (organization_id = '0' AND $3 IS NULL))
              AND id = CAST($4 AS TEXT)
              AND LOWER(COALESCE(status, '')) = 'active'
            LIMIT 1
            "#,
        )
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(command.organization_id.as_deref())
        .bind(&line.sku_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| store_error("failed to load checkout sku", error))?
        .ok_or_else(|| CommerceServiceError::not_found("checkout sku was not found"))?;

        let unit_price = string_cell(&row, "price_amount");
        let line_total = multiply_money_amount(&unit_price, line.quantity)?;
        let title = string_cell(&row, "title");
        let fulfillment_type = string_cell(&row, "fulfillment_type");
        let snapshot = serde_json::json!({
            "title": title.as_str(),
            "fulfillment_type": fulfillment_type.as_str(),
            "product_type": fulfillment_type.as_str(),
        })
        .to_string();
        resolved.push(ResolvedCheckoutLine {
            sku_id: line.sku_id.clone(),
            product_id: optional_string_cell(&row, "spu_id"),
            _title: title,
            unit_price,
            quantity: line.quantity,
            line_total,
            sku_snapshot_json: snapshot,
            fulfillment_type,
            shop_id: None,
        });
    }
    Ok(resolved)
}

async fn insert_checkout_session(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateCheckoutSessionCommand,
    session_id: &str,
    _original_amount: &str,
    expires_at: &str,
    now: &str,
) -> Result<(), CommerceServiceError> {
    let request_hash = checkout_session_request_hash(command);
    sqlx::query(
        r#"
        INSERT INTO commerce_checkout_session
            (id, tenant_id, organization_id, checkout_session_no, owner_user_id, source_type,
             status, currency_code, promotion_snapshot_json, request_hash, request_no,
             idempotency_key, shop_id, merchant_organization_id, shop_snapshot_json,
             shipping_address_snapshot_json, expires_at, created_at, updated_at)
        VALUES
            ($1, $2, $3, $4, $5, 'cart', 'active', $6, '[]', $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
       "#,
    )
    .bind(session_id)
    .bind(&command.tenant_id)
    .bind(command.organization_id.as_deref())
    .bind(&command.request_no)
    .bind(&command.owner_user_id)
    .bind(&command.currency_code)
    .bind(&request_hash)
    .bind(&command.request_no)
    .bind(&command.idempotency_key)
    .bind(command.physical_checkout.as_ref().map(|value| value.shop_id.as_str()))
    .bind(
        command
            .physical_checkout
            .as_ref()
            .map(|value| value.merchant_organization_id.as_str()),
    )
    .bind(
        command
            .physical_checkout
            .as_ref()
            .map(|value| value.shop_snapshot_json.as_str()),
    )
    .bind(
        command
            .physical_checkout
            .as_ref()
            .map(|value| value.shipping_address.snapshot_json())
            .transpose()?,
    )
    .bind(expires_at)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert checkout session", error))?;
    Ok(())
}

async fn insert_checkout_lines(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateCheckoutSessionCommand,
    session_id: &str,
    lines: &[ResolvedCheckoutLine],
    now: &str,
) -> Result<(), CommerceServiceError> {
    for (index, line) in lines.iter().enumerate() {
        let line_id = format!("{session_id}-line-{index}");
        sqlx::query(
            r#"
            INSERT INTO commerce_checkout_line
                (id, tenant_id, organization_id, checkout_session_id, product_id, sku_id,
                 shop_id, sku_snapshot_json, selected_options_hash, quantity, purchase_type,
                 fulfillment_type, price_amount_snapshot, currency_code, selected, created_at,
                 updated_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, 'default', $9, 'one_time', $10, $11, $12, 1, $13, $14)
           "#,
        )
        .bind(&line_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(session_id)
        .bind(line.product_id.as_deref())
        .bind(&line.sku_id)
        .bind(line.shop_id.as_deref())
        .bind(&line.sku_snapshot_json)
        .bind(line.quantity)
        .bind(&line.fulfillment_type)
        .bind(&line.unit_price)
        .bind(&command.currency_code)
        .bind(now)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(|error| store_error("failed to insert checkout line", error))?;
    }
    Ok(())
}

async fn insert_checkout_quote(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateCheckoutSessionCommand,
    session_id: &str,
    quote_id: &str,
    original_amount: &str,
    discount_amount: &str,
    payable_amount: &str,
    expires_at: &str,
    now: &str,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        INSERT INTO commerce_checkout_quote
            (id, tenant_id, organization_id, checkout_session_id, quote_no, original_amount,
             discount_amount, payable_amount, currency_code, quote_status, expires_at, created_at)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'ready', $10, $11)
       "#,
    )
    .bind(quote_id)
    .bind(&command.tenant_id)
    .bind(command.organization_id.as_deref())
    .bind(session_id)
    .bind(&command.request_no)
    .bind(original_amount)
    .bind(discount_amount)
    .bind(payable_amount)
    .bind(&command.currency_code)
    .bind(expires_at)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert checkout quote", error))?;
    Ok(())
}

async fn load_checkout_session_for_quote(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateCheckoutQuoteCommand,
) -> Result<sqlx::postgres::PgRow, CommerceServiceError> {
    sqlx::query(
        r#"
        SELECT currency_code, expires_at, status
        FROM commerce_checkout_session
        WHERE id = CAST($1 AS TEXT)
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
    .map_err(|error| store_error("failed to load checkout session for quote", error))?
    .ok_or_else(|| CommerceServiceError::conflict("checkout session is not quotable"))
}

async fn load_checkout_lines_for_quote(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateCheckoutQuoteCommand,
) -> Result<Vec<ResolvedCheckoutLine>, CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT sku_id, product_id, shop_id, sku_snapshot_json, quantity, price_amount_snapshot, fulfillment_type
        FROM commerce_checkout_line
        WHERE tenant_id = CAST($1 AS TEXT)
          AND checkout_session_id = CAST($2 AS TEXT)
          AND selected = 1
        ORDER BY created_at ASC, id ASC
       "#,
    )
    .bind(&command.tenant_id)
    .bind(&command.checkout_session_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load checkout lines for quote", error))?;

    rows.iter()
        .map(
            |row| -> Result<ResolvedCheckoutLine, CommerceServiceError> {
                let unit_price = string_cell(row, "price_amount_snapshot");
                let quantity = row.try_get::<i64, _>("quantity").map_err(|error| {
                    store_error("failed to decode checkout line quantity", error)
                })?;
                Ok(ResolvedCheckoutLine {
                    sku_id: string_cell(row, "sku_id"),
                    product_id: optional_string_cell(row, "product_id"),
                    _title: checkout_line_title(row),
                    unit_price: unit_price.clone(),
                    quantity,
                    line_total: multiply_money_amount(&unit_price, quantity)?,
                    sku_snapshot_json: string_cell(row, "sku_snapshot_json"),
                    fulfillment_type: string_cell(row, "fulfillment_type"),
                    shop_id: optional_string_cell(row, "shop_id"),
                })
            },
        )
        .collect()
}

async fn insert_checkout_quote_for_command(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateCheckoutQuoteCommand,
    quote_id: &str,
    currency_code: &str,
    original_amount: &str,
    discount_amount: &str,
    payable_amount: &str,
    expires_at: &str,
    now: &str,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        INSERT INTO commerce_checkout_quote
            (id, tenant_id, organization_id, checkout_session_id, quote_no, original_amount,
             discount_amount, payable_amount, currency_code, quote_status, expires_at, created_at)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'ready', $10, $11)
       "#,
    )
    .bind(quote_id)
    .bind(&command.tenant_id)
    .bind(command.organization_id.as_deref())
    .bind(&command.checkout_session_id)
    .bind(format!(
        "{}:{}",
        command.request_no, command.idempotency_key
    ))
    .bind(original_amount)
    .bind(discount_amount)
    .bind(payable_amount)
    .bind(currency_code)
    .bind(expires_at)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert checkout quote", error))?;
    Ok(())
}

async fn update_checkout_session_status(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateCheckoutQuoteCommand,
    status: &str,
    now: &str,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        UPDATE commerce_checkout_session
        SET status = $1, updated_at = $2
        WHERE id = CAST($3 AS TEXT)
          AND tenant_id = CAST($4 AS TEXT)
          AND owner_user_id = CAST($5 AS TEXT)
       "#,
    )
    .bind(status)
    .bind(now)
    .bind(&command.checkout_session_id)
    .bind(&command.tenant_id)
    .bind(&command.owner_user_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to update checkout session status", error))?;
    Ok(())
}

async fn load_checkout_idempotency_row(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    scope: &str,
    idempotency_key: &str,
) -> Result<Option<sqlx::postgres::PgRow>, CommerceServiceError> {
    sqlx::query(
        r#"
        SELECT request_hash, response_json, status
        FROM commerce_idempotency_key
        WHERE tenant_id = $1 AND scope = $2 AND idempotency_key = $3
        LIMIT 1
       "#,
    )
    .bind(tenant_id)
    .bind(scope)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load checkout idempotency record", error))
}

async fn refresh_checkout_idempotency_lock(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    scope: &str,
    idempotency_key: &str,
    now: &str,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        UPDATE commerce_idempotency_key
        SET status = 'locked', locked_until = $1, expires_at = $2, updated_at = $3
        WHERE tenant_id = $4 AND scope = $5 AND idempotency_key = $6
       "#,
    )
    .bind(now)
    .bind(now)
    .bind(now)
    .bind(tenant_id)
    .bind(scope)
    .bind(idempotency_key)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to refresh checkout idempotency lock", error))?;
    Ok(())
}

async fn insert_checkout_idempotency_lock(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    organization_id: Option<&str>,
    scope: &str,
    idempotency_key: &str,
    request_hash: &str,
    id: &str,
    now: &str,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        INSERT INTO commerce_idempotency_key
            (id, tenant_id, organization_id, scope, idempotency_key, request_hash,
             status, locked_until, expires_at, created_at, updated_at)
        VALUES
            ($1, $2, $3, $4, $5, $6, 'locked', $7, $8, $9, $10)
       "#,
    )
    .bind(id)
    .bind(tenant_id)
    .bind(organization_id)
    .bind(scope)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(now)
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert checkout idempotency lock", error))?;
    Ok(())
}

async fn complete_checkout_idempotency(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    scope: &str,
    idempotency_key: &str,
    session: &CheckoutSessionView,
    now: &str,
) -> Result<(), CommerceServiceError> {
    let response_json = serde_json::json!({
        "checkoutSessionId": session.checkout_session_id,
        "status": session.status,
        "currencyCode": session.currency_code,
        "originalAmount": session.original_amount.as_str(),
        "discountAmount": session.discount_amount.as_str(),
        "payableAmount": session.payable_amount.as_str(),
        "quoteId": session.quote_id,
    })
    .to_string();
    sqlx::query(
        r#"
        UPDATE commerce_idempotency_key
        SET response_json = $1, status = 'completed', locked_until = NULL, updated_at = $2
        WHERE tenant_id = $3 AND scope = $4 AND idempotency_key = $5
       "#,
    )
    .bind(response_json)
    .bind(now)
    .bind(tenant_id)
    .bind(scope)
    .bind(idempotency_key)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to complete checkout idempotency record", error))?;
    Ok(())
}

async fn complete_checkout_quote_idempotency(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    idempotency_key: &str,
    quote: &CheckoutQuoteView,
    now: &str,
) -> Result<(), CommerceServiceError> {
    let response_json = serde_json::json!({
        "checkoutSessionId": quote.checkout_session_id,
        "quoteId": quote.quote_id,
        "currencyCode": quote.currency_code,
        "originalAmount": quote.original_amount.as_str(),
        "discountAmount": quote.discount_amount.as_str(),
        "payableAmount": quote.payable_amount.as_str(),
    })
    .to_string();
    sqlx::query(
        r#"
        UPDATE commerce_idempotency_key
        SET response_json = $1, status = 'completed', locked_until = NULL, updated_at = $2
        WHERE tenant_id = $3 AND scope = $4 AND idempotency_key = $5
       "#,
    )
    .bind(response_json)
    .bind(now)
    .bind(tenant_id)
    .bind(CHECKOUT_QUOTE_CREATE_SCOPE)
    .bind(idempotency_key)
    .execute(&mut **tx)
    .await
    .map_err(|error| {
        store_error(
            "failed to complete checkout quote idempotency record",
            error,
        )
    })?;
    Ok(())
}

fn replay_checkout_session(
    row: &sqlx::postgres::PgRow,
) -> Result<CheckoutSessionView, CommerceServiceError> {
    let response_json = optional_string_cell(row, "response_json").ok_or_else(|| {
        CommerceServiceError::invalid_state("checkout idempotency record has no response")
    })?;
    let value: serde_json::Value = serde_json::from_str(&response_json).map_err(|error| {
        CommerceServiceError::storage(format!("invalid checkout idempotency response: {error}"))
    })?;
    Ok(CheckoutSessionView {
        checkout_session_id: json_string(&value, "checkoutSessionId")?,
        status: json_string(&value, "status")?,
        currency_code: json_string(&value, "currencyCode")?,
        original_amount: commerce_money(&json_string(&value, "originalAmount")?)?,
        discount_amount: commerce_money(&json_string(&value, "discountAmount")?)?,
        payable_amount: commerce_money(&json_string(&value, "payableAmount")?)?,
        quote_id: value
            .get("quoteId")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    })
}

fn replay_checkout_quote(
    row: &sqlx::postgres::PgRow,
) -> Result<CheckoutQuoteView, CommerceServiceError> {
    let response_json = optional_string_cell(row, "response_json").ok_or_else(|| {
        CommerceServiceError::invalid_state("checkout quote idempotency record has no response")
    })?;
    let value: serde_json::Value = serde_json::from_str(&response_json).map_err(|error| {
        CommerceServiceError::storage(format!(
            "invalid checkout quote idempotency response: {error}"
        ))
    })?;
    Ok(CheckoutQuoteView {
        checkout_session_id: json_string(&value, "checkoutSessionId")?,
        quote_id: json_string(&value, "quoteId")?,
        currency_code: json_string(&value, "currencyCode")?,
        original_amount: commerce_money(&json_string(&value, "originalAmount")?)?,
        discount_amount: commerce_money(&json_string(&value, "discountAmount")?)?,
        payable_amount: commerce_money(&json_string(&value, "payableAmount")?)?,
    })
}

fn checkout_session_id(command: &CreateCheckoutSessionCommand) -> String {
    stable_storage_id(&[
        "checkout-session",
        &command.tenant_id,
        &command.idempotency_key,
    ])
}

fn checkout_quote_id(tenant_id: &str, session_id: &str, request_no: &str) -> String {
    stable_storage_id(&["checkout-quote", tenant_id, session_id, request_no])
}

fn checkout_idempotency_id(tenant_id: &str, scope: &str, idempotency_key: &str) -> String {
    stable_storage_id(&["checkout-idempotency", tenant_id, scope, idempotency_key])
}

fn checkout_expires_at(now: &str) -> String {
    now.to_owned()
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

fn json_string(value: &serde_json::Value, field: &str) -> Result<String, CommerceServiceError> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            CommerceServiceError::storage(format!("checkout response {field} is missing"))
        })
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
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
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
