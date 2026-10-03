//! PostgreSQL store for app-template orders — the order center's own record of
//! "who bought which published app template".
//!
//! The deployments module owns the catalog (`deploy_app_template`,
//! `deploy_app_template_version`); this store reads that catalog to snapshot the
//! listing's title, price, currency and published version onto the order, and
//! writes nothing back except the listing's install counter. There is no
//! module-owned purchase table anywhere: a settled `commerce_order` with subject
//! `app_template` *is* the entitlement.
//!
//! Free listings settle inside the creating transaction (zero amount, no payment
//! provider, order created `paid` + `fulfilled`); paid listings are created
//! `pending_payment` and settle through the normal payment notify pipeline.

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_order_service::{
    app_template_order_status_label, AppTemplateOrderListQuery, AppTemplateOrderPage,
    AppTemplateOrderSummary, CreateAppTemplateOrderCommand, CreateAppTemplateOrderOutcome,
    APP_TEMPLATE_ORDER_SUBJECT,
};
use sdkwork_utils_rust::{build_commerce_cashier_url, commerce_cashier_scene};
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::app_template_order_identity::{
    app_template_order_request_fingerprint, app_template_purchase_intent_key,
    ensure_app_template_request_fingerprint_matches,
};

/// Platform scope sentinel used by `commerce_order.organization_id`.
const PLATFORM_ORGANIZATION_SCOPE_SENTINEL: &str = "0";
/// Catalog statuses that make a listing purchasable.
const TEMPLATE_STATUS_PUBLISHED: &str = "PUBLISHED";
/// Only public listings are sold through the marketplace.
const TEMPLATE_VISIBILITY_PUBLIC: &str = "PUBLIC";
/// Pricing vocabulary of `deploy_app_template.pricing_model`.
const TEMPLATE_PRICING_FREE: &str = "FREE";

/// Order lifecycle values that mean "the buyer already owns this listing".
/// Kept next to the literal in `LOAD_SETTLED_ORDER` so the predicate is
/// reviewable in one place; the test below fails if the two drift apart.
#[cfg(test)]
const SETTLED_ORDER_STATUS_PREDICATE: &str =
    "o.status IN ('paid', 'fulfilled', 'completed', 'finished')";

const LOAD_TEMPLATE: &str = r#"
SELECT
    CAST(t.id AS TEXT) AS template_id,
    t.uuid AS template_uuid,
    t.display_name,
    CAST(t.price_minor AS TEXT) AS price_minor,
    COALESCE(NULLIF(t.currency, ''), 'CNY') AS currency_code,
    t.pricing_model,
    t.status,
    t.visibility,
    CAST(t.author_user_id AS TEXT) AS author_user_id,
    (
        SELECT v.uuid
        FROM deploy_app_template_version v
        WHERE v.template_id = t.id
          AND v.status = 'PUBLISHED'
          AND v.deleted_at IS NULL
        ORDER BY v.created_at DESC, v.id DESC
        LIMIT 1
    ) AS version_uuid
FROM deploy_app_template t
WHERE t.tenant_id = $1
  AND t.uuid = $2
  AND t.organization_id = COALESCE($3, 0)
  AND t.deleted_at IS NULL
ORDER BY t.id ASC
LIMIT 1
"#;

/// Loads one order for the caller, either by idempotency key (any lifecycle) or
/// by purchase intent (only while it is still awaiting payment and not expired).
///
/// The item snapshot is read through the row's JSON form: the published baseline
/// declares `sku_snapshot_json` as TEXT while test fixtures declare JSONB, and
/// `to_jsonb(row) ->> 'column'` yields the same JSON text for both.
const LOAD_ORDER: &str = r#"
SELECT
    o.id AS order_id,
    o.order_no,
    COALESCE(NULLIF(o.request_no, ''), o.order_no) AS out_trade_no,
    o.request_fingerprint,
    o.status AS order_status,
    TO_CHAR(o.expired_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS expires_at,
    CAST(COALESCE(ab.payable_amount, oi.total_amount, 0) AS TEXT) AS amount,
    COALESCE(NULLIF(o.currency_code, ''), 'CNY') AS currency_code,
    COALESCE(
        NULLIF(COALESCE(NULLIF(to_jsonb(oi) ->> 'sku_snapshot_json', ''), '{}')::jsonb ->> 'templateUuid', ''),
        NULLIF(to_jsonb(oi) ->> 'sku_id', '')
    ) AS template_uuid,
    COALESCE(
        NULLIF(COALESCE(NULLIF(to_jsonb(oi) ->> 'sku_snapshot_json', ''), '{}')::jsonb ->> 'templateName', ''),
        NULLIF(to_jsonb(oi) ->> 'title', ''),
        'App template'
    ) AS template_name,
    NULLIF(COALESCE(NULLIF(to_jsonb(oi) ->> 'sku_snapshot_json', ''), '{}')::jsonb ->> 'versionUuid', '') AS version_uuid
FROM commerce_order o
LEFT JOIN commerce_order_item oi
    ON oi.tenant_id = o.tenant_id
   AND oi.order_id = o.id
LEFT JOIN commerce_order_amount_breakdown ab
    ON ab.tenant_id = o.tenant_id
   AND ab.order_id = o.id
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND (
        (o.organization_id = CAST($2 AS TEXT))
        OR (o.organization_id = '0' AND $2 IS NULL)
        OR (o.organization_id IS NULL AND $2 IS NULL)
      )
  AND o.owner_user_id = CAST($3 AS TEXT)
  AND o.subject = $4
  AND (
        ($5::text IS NOT NULL AND o.idempotency_key = CAST($5 AS TEXT))
        OR
        ($6::text IS NOT NULL
         AND o.purchase_intent_key = CAST($6 AS TEXT)
         AND o.status IN ('draft', 'pending', 'pending_payment', 'unpaid', 'wait_pay', 'created')
         AND o.expired_at IS NOT NULL
         AND o.expired_at > $7::timestamptz)
      )
ORDER BY oi.created_at ASC NULLS LAST, oi.id ASC
LIMIT 1
"#;

/// Finds a settled order for the same buyer and listing: the entitlement the
/// buyer already holds, so a second purchase is answered instead of duplicated.
const LOAD_SETTLED_ORDER: &str = r#"
SELECT
    o.id AS order_id,
    o.order_no,
    COALESCE(NULLIF(o.request_no, ''), o.order_no) AS out_trade_no,
    o.status AS order_status,
    TO_CHAR(o.expired_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS expires_at,
    CAST(COALESCE(ab.payable_amount, oi.total_amount, 0) AS TEXT) AS amount,
    COALESCE(NULLIF(o.currency_code, ''), 'CNY') AS currency_code,
    COALESCE(
        NULLIF(COALESCE(NULLIF(to_jsonb(oi) ->> 'sku_snapshot_json', ''), '{}')::jsonb ->> 'templateUuid', ''),
        NULLIF(to_jsonb(oi) ->> 'sku_id', '')
    ) AS template_uuid,
    COALESCE(
        NULLIF(COALESCE(NULLIF(to_jsonb(oi) ->> 'sku_snapshot_json', ''), '{}')::jsonb ->> 'templateName', ''),
        NULLIF(to_jsonb(oi) ->> 'title', ''),
        'App template'
    ) AS template_name,
    NULLIF(COALESCE(NULLIF(to_jsonb(oi) ->> 'sku_snapshot_json', ''), '{}')::jsonb ->> 'versionUuid', '') AS version_uuid
FROM commerce_order o
LEFT JOIN commerce_order_item oi
    ON oi.tenant_id = o.tenant_id
   AND oi.order_id = o.id
LEFT JOIN commerce_order_amount_breakdown ab
    ON ab.tenant_id = o.tenant_id
   AND ab.order_id = o.id
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND (
        (o.organization_id = CAST($2 AS TEXT))
        OR (o.organization_id = '0' AND $2 IS NULL)
        OR (o.organization_id IS NULL AND $2 IS NULL)
      )
  AND o.owner_user_id = CAST($3 AS TEXT)
  AND o.subject = $4
  AND COALESCE(
        NULLIF(COALESCE(NULLIF(to_jsonb(oi) ->> 'sku_snapshot_json', ''), '{}')::jsonb ->> 'templateUuid', ''),
        NULLIF(to_jsonb(oi) ->> 'sku_id', '')
      ) = $5
  AND o.status IN ('paid', 'fulfilled', 'completed', 'finished')
ORDER BY o.created_at DESC, o.id DESC
LIMIT 1
"#;

#[derive(Debug, Clone)]
pub struct PostgresCommerceAppTemplateOrderStore {
    pool: PgPool,
}

#[derive(Debug, Clone)]
struct AppTemplateCatalog {
    id: i64,
    uuid: String,
    display_name: String,
    price_amount: CommerceMoney,
    currency_code: String,
    pricing_model: String,
    author_user_id: i64,
    version_uuid: Option<String>,
}

/// One stored order projected back into the create outcome.
struct StoredAppTemplateOrder {
    request_fingerprint: Option<String>,
    value: CreateAppTemplateOrderOutcome,
}

impl PostgresCommerceAppTemplateOrderStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_app_template_order(
        &self,
        command: CreateAppTemplateOrderCommand,
    ) -> Result<CreateAppTemplateOrderOutcome, CommerceServiceError> {
        let mut tx = self.pool.begin().await.map_err(|error| {
            store_error("failed to begin app template order transaction", error)
        })?;

        let template = load_published_template(&mut tx, &command).await?;
        let owner_user_id = parse_catalog_id("owner_user_id", &command.owner_user_id)?;
        if template.author_user_id == owner_user_id {
            return Err(CommerceServiceError::conflict(
                "the template author already holds this template",
            ));
        }
        let is_free = template.pricing_model == TEMPLATE_PRICING_FREE;
        let request_fingerprint = app_template_order_request_fingerprint(&command);
        let purchase_intent_key = app_template_purchase_intent_key(
            &command,
            template.price_amount.as_str(),
            &template.currency_code,
        );
        let method_key = normalize_method_key(&command.method);

        if let Some(stored) =
            load_order_in_tx(&mut tx, &command, Some(&command.idempotency_key), None).await?
        {
            ensure_app_template_request_fingerprint_matches(
                stored.request_fingerprint.as_deref().unwrap_or_default(),
                &request_fingerprint,
            )?;
            tx.commit()
                .await
                .map_err(|error| store_error("failed to commit app template replay", error))?;
            return Ok(stored.value);
        }

        // The buyer already owns the listing: answer with the settled order
        // instead of selling the same template twice.
        if let Some(stored) = load_settled_order_in_tx(&mut tx, &command, &template.uuid).await? {
            tx.commit()
                .await
                .map_err(|error| store_error("failed to commit owned template replay", error))?;
            return Ok(stored.value);
        }

        expire_stale_app_template_orders(&mut tx, &command, &purchase_intent_key).await?;
        if let Some(stored) =
            load_order_in_tx(&mut tx, &command, None, Some(&purchase_intent_key)).await?
        {
            tx.commit().await.map_err(|error| {
                store_error("failed to commit reusable app template order", error)
            })?;
            return Ok(stored.value);
        }

        let inserted = insert_order(
            &mut tx,
            &command,
            &template,
            &request_fingerprint,
            &purchase_intent_key,
            is_free,
        )
        .await?;
        if !inserted {
            if let Some(stored) =
                load_order_in_tx(&mut tx, &command, Some(&command.idempotency_key), None).await?
            {
                ensure_app_template_request_fingerprint_matches(
                    stored.request_fingerprint.as_deref().unwrap_or_default(),
                    &request_fingerprint,
                )?;
                tx.commit()
                    .await
                    .map_err(|error| store_error("failed to commit concurrent replay", error))?;
                return Ok(stored.value);
            }
            if let Some(stored) =
                load_order_in_tx(&mut tx, &command, None, Some(&purchase_intent_key)).await?
            {
                tx.commit().await.map_err(|error| {
                    store_error("failed to commit concurrent app template reuse", error)
                })?;
                return Ok(stored.value);
            }
            return Err(CommerceServiceError::conflict(
                "app template order creation conflicted with another request",
            ));
        }
        insert_order_item(&mut tx, &command, &template, &method_key, is_free).await?;
        insert_order_amount_breakdown(&mut tx, &command, &template).await?;
        if is_free {
            // Zero-amount listing: no provider is involved, so the order center
            // settles it here and counts the install in the same transaction.
            settle_free_order(&mut tx, &command, &template).await?;
        }

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit app template order", error))?;

        build_outcome(
            &command,
            &template,
            &method_key,
            if is_free { "paid" } else { "pending_payment" },
            false,
        )
    }

    /// The buyer's app-template orders: the entitlement inventory the client
    /// renders as "my templates".
    pub async fn list_app_template_orders(
        &self,
        query: &AppTemplateOrderListQuery,
    ) -> Result<AppTemplateOrderPage, CommerceServiceError> {
        let organization_id = query.organization_id.as_deref();
        let rows = sqlx::query(
            r#"
            SELECT
                o.id AS order_id,
                o.order_no,
                o.status AS order_status,
                COALESCE(o.fulfillment_status, '') AS fulfillment_status,
                TO_CHAR(o.paid_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS paid_at,
                TO_CHAR(o.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS created_at,
                CAST(COALESCE(ab.payable_amount, oi.total_amount, 0) AS TEXT) AS amount,
                COALESCE(NULLIF(o.currency_code, ''), 'CNY') AS currency_code,
                COALESCE(
                    NULLIF(COALESCE(NULLIF(to_jsonb(oi) ->> 'sku_snapshot_json', ''), '{}')::jsonb ->> 'templateUuid', ''),
                    NULLIF(to_jsonb(oi) ->> 'sku_id', '')
                ) AS template_uuid,
                COALESCE(
                    NULLIF(COALESCE(NULLIF(to_jsonb(oi) ->> 'sku_snapshot_json', ''), '{}')::jsonb ->> 'templateName', ''),
                    NULLIF(to_jsonb(oi) ->> 'title', ''),
                    'App template'
                ) AS template_name,
                NULLIF(COALESCE(NULLIF(to_jsonb(oi) ->> 'sku_snapshot_json', ''), '{}')::jsonb ->> 'versionUuid', '') AS version_uuid,
                COUNT(*) OVER() AS total
            FROM commerce_order o
            LEFT JOIN commerce_order_item oi
                ON oi.tenant_id = o.tenant_id
               AND oi.order_id = o.id
            LEFT JOIN commerce_order_amount_breakdown ab
                ON ab.tenant_id = o.tenant_id
               AND ab.order_id = o.id
            WHERE o.tenant_id = CAST($1 AS TEXT)
              AND (
                    (o.organization_id = CAST($2 AS TEXT))
                    OR (o.organization_id = '0' AND $2 IS NULL)
                    OR (o.organization_id IS NULL AND $2 IS NULL)
                  )
              AND o.owner_user_id = CAST($3 AS TEXT)
              AND o.subject = $4
            ORDER BY o.created_at DESC, o.id DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(&query.tenant_id)
        .bind(organization_id)
        .bind(&query.owner_user_id)
        .bind(APP_TEMPLATE_ORDER_SUBJECT)
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| store_error("failed to list app template orders", error))?;

        let total = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total").ok())
            .unwrap_or(0);
        let items = rows
            .iter()
            .map(map_app_template_order_summary)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(AppTemplateOrderPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }
}

async fn load_published_template(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateAppTemplateOrderCommand,
) -> Result<AppTemplateCatalog, CommerceServiceError> {
    let tenant_id = parse_catalog_id("tenant_id", &command.tenant_id)?;
    let organization_id = command
        .organization_id
        .as_deref()
        .map(|value| parse_catalog_id("organization_id", value))
        .transpose()?;

    let row = sqlx::query(LOAD_TEMPLATE)
        .bind(tenant_id)
        .bind(&command.template_uuid)
        .bind(organization_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| store_error("failed to load app template listing", error))?
        .ok_or_else(|| CommerceServiceError::not_found("app template listing was not found"))?;

    let status = string_cell(&row, "status");
    let visibility = string_cell(&row, "visibility");
    if status != TEMPLATE_STATUS_PUBLISHED || visibility != TEMPLATE_VISIBILITY_PUBLIC {
        return Err(CommerceServiceError::conflict(
            "app template listing is not published to the marketplace",
        ));
    }

    Ok(AppTemplateCatalog {
        id: required_integer_cell(&row, "template_id")?,
        uuid: string_cell(&row, "template_uuid"),
        display_name: string_cell(&row, "display_name"),
        price_amount: commerce_money_cell(&row, "price_minor", "app template price")?,
        currency_code: string_cell(&row, "currency_code")
            .trim()
            .to_ascii_uppercase(),
        pricing_model: string_cell(&row, "pricing_model"),
        author_user_id: required_integer_cell(&row, "author_user_id")?,
        version_uuid: optional_string_cell(&row, "version_uuid"),
    })
}

async fn load_order_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateAppTemplateOrderCommand,
    idempotency_key: Option<&str>,
    purchase_intent_key: Option<&str>,
) -> Result<Option<StoredAppTemplateOrder>, CommerceServiceError> {
    let method_key = normalize_method_key(&command.method);
    let row = sqlx::query(LOAD_ORDER)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(APP_TEMPLATE_ORDER_SUBJECT)
        .bind(idempotency_key)
        .bind(purchase_intent_key)
        .bind(&command.requested_at)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| store_error("failed to load app template order", error))?;

    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(StoredAppTemplateOrder {
        request_fingerprint: optional_string_cell(&row, "request_fingerprint"),
        value: map_create_outcome(&row, &method_key, true)?,
    }))
}

async fn load_settled_order_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateAppTemplateOrderCommand,
    template_uuid: &str,
) -> Result<Option<StoredAppTemplateOrder>, CommerceServiceError> {
    let method_key = normalize_method_key(&command.method);
    let row = sqlx::query(LOAD_SETTLED_ORDER)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(APP_TEMPLATE_ORDER_SUBJECT)
        .bind(template_uuid)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| store_error("failed to load owned app template order", error))?;

    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(StoredAppTemplateOrder {
        request_fingerprint: None,
        value: map_create_outcome(&row, &method_key, true)?,
    }))
}

async fn expire_stale_app_template_orders(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateAppTemplateOrderCommand,
    purchase_intent_key: &str,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        UPDATE commerce_order
        SET status = 'expired', payment_status = 'expired', updated_at = CAST($1 AS TIMESTAMPTZ)
        WHERE tenant_id = CAST($2 AS TEXT)
          AND organization_id = CAST($3 AS TEXT)
          AND owner_user_id = CAST($4 AS TEXT)
          AND subject = $5
          AND purchase_intent_key = CAST($6 AS TEXT)
          AND status IN ('draft', 'pending', 'pending_payment', 'unpaid', 'wait_pay', 'created')
          AND expired_at IS NOT NULL
          AND expired_at <= CAST($7 AS TIMESTAMPTZ)
        "#,
    )
    .bind(&command.requested_at)
    .bind(&command.tenant_id)
    .bind(normalize_organization_scope(
        command.organization_id.as_deref(),
    ))
    .bind(&command.owner_user_id)
    .bind(APP_TEMPLATE_ORDER_SUBJECT)
    .bind(purchase_intent_key)
    .bind(&command.requested_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to expire stale app template orders", error))?;
    Ok(())
}

async fn insert_order(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateAppTemplateOrderCommand,
    template: &AppTemplateCatalog,
    request_fingerprint: &str,
    purchase_intent_key: &str,
    is_free: bool,
) -> Result<bool, CommerceServiceError> {
    let payload = serde_json::json!({
        "id": command.order_id,
        "tenant_id": command.tenant_id,
        "organization_id": normalize_organization_scope(command.organization_id.as_deref()),
        "owner_user_id": command.owner_user_id,
        "order_no": command.order_no,
        "subject": APP_TEMPLATE_ORDER_SUBJECT,
        // A free listing has nothing to collect: it is created settled.
        "status": if is_free { "paid" } else { "pending_payment" },
        "payment_status": if is_free { "success" } else { "pending" },
        "fulfillment_status": if is_free { "fulfilled" } else { "unfulfilled" },
        "refund_status": "none",
        "currency_code": template.currency_code,
        "request_no": command.out_trade_no,
        "idempotency_key": command.idempotency_key,
        "request_fingerprint": request_fingerprint,
        "purchase_intent_key": purchase_intent_key,
        "created_at": command.requested_at,
        "paid_at": if is_free { serde_json::Value::String(command.requested_at.clone()) } else { serde_json::Value::Null },
        "cancelled_at": serde_json::Value::Null,
        "expired_at": command.expire_at,
        "updated_at": command.requested_at,
    });
    let result = sqlx::query(
        r#"
        INSERT INTO commerce_order
        SELECT * FROM jsonb_populate_record(NULL::commerce_order, $1::jsonb)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(payload.to_string())
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert app template order", error))?;
    Ok(result.rows_affected() == 1)
}

async fn insert_order_item(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateAppTemplateOrderCommand,
    template: &AppTemplateCatalog,
    payment_method: &str,
    is_free: bool,
) -> Result<(), CommerceServiceError> {
    let payload = serde_json::json!({
        "id": command.order_item_id,
        "tenant_id": command.tenant_id,
        "organization_id": normalize_organization_scope(command.organization_id.as_deref()),
        "order_id": command.order_id,
        "sku_id": template.uuid,
        "sku_snapshot_json": order_item_snapshot_json(template, command, payment_method),
        "title": template.display_name,
        "quantity": 1,
        "unit_price_amount": template.price_amount.as_str(),
        "discount_amount": "0",
        "tax_amount": "0",
        "total_amount": template.price_amount.as_str(),
        "fulfillment_status": if is_free { "fulfilled" } else { "unfulfilled" },
        "refund_status": "none",
        "created_at": command.requested_at,
    });
    sqlx::query(
        r#"
        INSERT INTO commerce_order_item
        SELECT * FROM jsonb_populate_record(NULL::commerce_order_item, $1::jsonb)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(payload.to_string())
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert app template order item", error))?;
    Ok(())
}

async fn insert_order_amount_breakdown(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateAppTemplateOrderCommand,
    template: &AppTemplateCatalog,
) -> Result<(), CommerceServiceError> {
    let payload = serde_json::json!({
        "id": format!("{}-amount", command.order_id),
        "tenant_id": command.tenant_id,
        "organization_id": normalize_organization_scope(command.organization_id.as_deref()),
        "order_id": command.order_id,
        "order_item_id": command.order_item_id,
        "allocation_type": "order_total",
        "original_amount": template.price_amount.as_str(),
        "discount_amount": "0",
        "payable_amount": template.price_amount.as_str(),
        "currency_code": template.currency_code,
        "created_at": command.requested_at,
    });
    sqlx::query(
        r#"
        INSERT INTO commerce_order_amount_breakdown
        SELECT * FROM jsonb_populate_record(NULL::commerce_order_amount_breakdown, $1::jsonb)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(payload.to_string())
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert app template amount breakdown", error))?;
    Ok(())
}

/// Settles a zero-amount listing: the order is already paid, the entitlement is
/// live, and the catalog owner counts the install exactly once.
async fn settle_free_order(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateAppTemplateOrderCommand,
    template: &AppTemplateCatalog,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        "UPDATE deploy_app_template
         SET install_count = install_count + 1, updated_at = CAST($1 AS TIMESTAMPTZ)
         WHERE id = $2 AND deleted_at IS NULL",
    )
    .bind(&command.requested_at)
    .bind(template.id)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to count free app template install", error))?;
    Ok(())
}

/// The order item's SKU snapshot. Deliberately carries no `product_type` /
/// `fulfillment_type` key: `stable_order_settlement_subject` prefers those keys
/// over the stored subject, so adding one would re-route settlement away from
/// the app-template handler.
fn order_item_snapshot_json(
    template: &AppTemplateCatalog,
    command: &CreateAppTemplateOrderCommand,
    payment_method: &str,
) -> String {
    serde_json::json!({
        "templateUuid": template.uuid,
        "templateName": template.display_name,
        "versionUuid": template.version_uuid,
        "pricingModel": template.pricing_model,
        "clientRequestNo": command.client_request_no,
        "source": command.source,
        "paymentMethod": payment_method,
        "paymentProduct": command.payment_product,
    })
    .to_string()
}

fn build_outcome(
    command: &CreateAppTemplateOrderCommand,
    template: &AppTemplateCatalog,
    payment_method: &str,
    status: &str,
    reused: bool,
) -> Result<CreateAppTemplateOrderOutcome, CommerceServiceError> {
    CreateAppTemplateOrderOutcome::new(
        &command.order_id,
        &command.order_no,
        &command.out_trade_no,
        template.price_amount.clone(),
        &template.currency_code,
        &template.uuid,
        &template.display_name,
        template.version_uuid.as_deref(),
        &command.expire_at,
        payment_method,
        status,
        reused,
        &app_template_cashier_url(&command.order_id, &command.out_trade_no),
    )
}

fn map_create_outcome(
    row: &sqlx::postgres::PgRow,
    payment_method: &str,
    reused: bool,
) -> Result<CreateAppTemplateOrderOutcome, CommerceServiceError> {
    CreateAppTemplateOrderOutcome::new(
        &string_cell(row, "order_id"),
        &string_cell(row, "order_no"),
        &string_cell(row, "out_trade_no"),
        commerce_money_cell(row, "amount", "app template order amount")?,
        &string_cell(row, "currency_code"),
        &string_cell(row, "template_uuid"),
        &string_cell(row, "template_name"),
        optional_string_cell(row, "version_uuid").as_deref(),
        &string_cell(row, "expires_at"),
        payment_method,
        app_template_order_status_label(&string_cell(row, "order_status")),
        reused,
        &app_template_cashier_url(
            &string_cell(row, "order_id"),
            &string_cell(row, "out_trade_no"),
        ),
    )
}

fn map_app_template_order_summary(
    row: &sqlx::postgres::PgRow,
) -> Result<AppTemplateOrderSummary, CommerceServiceError> {
    Ok(AppTemplateOrderSummary {
        amount: commerce_money_cell(row, "amount", "app template order amount")?,
        created_at: string_cell(row, "created_at"),
        currency_code: string_cell(row, "currency_code"),
        fulfillment_status: string_cell(row, "fulfillment_status"),
        order_id: string_cell(row, "order_id"),
        order_no: string_cell(row, "order_no"),
        paid_at: optional_string_cell(row, "paid_at"),
        status: app_template_order_status_label(&string_cell(row, "order_status")).to_owned(),
        template_name: string_cell(row, "template_name"),
        template_uuid: string_cell(row, "template_uuid"),
        version_uuid: optional_string_cell(row, "version_uuid"),
    })
}

fn app_template_cashier_url(order_id: &str, out_trade_no: &str) -> String {
    build_commerce_cashier_url(
        commerce_cashier_scene(Some(APP_TEMPLATE_ORDER_SUBJECT)),
        order_id,
        out_trade_no,
    )
}

fn normalize_organization_scope(organization_id: Option<&str>) -> String {
    organization_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(PLATFORM_ORGANIZATION_SCOPE_SENTINEL)
        .to_owned()
}

fn normalize_method_key(method: &str) -> String {
    match method.trim().to_ascii_lowercase().as_str() {
        "wechat" => "wechat_pay".to_string(),
        other => other.to_string(),
    }
}

fn parse_catalog_id(field: &str, value: &str) -> Result<i64, CommerceServiceError> {
    value
        .trim()
        .parse::<i64>()
        .map_err(|_| CommerceServiceError::validation(format!("{field} must be a decimal id")))
}

fn commerce_money_cell(
    row: &sqlx::postgres::PgRow,
    column: &str,
    field_name: &str,
) -> Result<CommerceMoney, CommerceServiceError> {
    let value = string_cell(row, column);
    // Canonical smallest-unit integer strings pass through; legacy major-unit
    // decimals are folded. `deploy_app_template.price_minor` is already minor.
    let normalized = crate::money_amount::normalize_stored_money(&value)
        .map_err(|_| CommerceServiceError::storage(format!("invalid {field_name}: {value}")))?;
    CommerceMoney::new(&normalized)
        .map_err(|message| CommerceServiceError::storage(format!("{message}: {value}")))
}

fn required_integer_cell(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<i64, CommerceServiceError> {
    row.try_get::<Option<i64>, _>(column)
        .ok()
        .flatten()
        .or_else(|| optional_string_cell(row, column).and_then(|value| value.parse().ok()))
        .ok_or_else(|| CommerceServiceError::storage(format!("invalid integer column {column}")))
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}

fn store_error(context: &str, error: sqlx::Error) -> CommerceServiceError {
    crate::sql_store_error::map_sqlx_store_error(context, error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_order_service::stable_order_settlement_subject;

    fn catalog() -> AppTemplateCatalog {
        AppTemplateCatalog {
            id: 7,
            uuid: "template-1".to_owned(),
            display_name: "Delivery console".to_owned(),
            price_amount: CommerceMoney::new("9900").expect("price"),
            currency_code: "CNY".to_owned(),
            pricing_model: "PAID".to_owned(),
            author_user_id: 42,
            version_uuid: Some("version-1".to_owned()),
        }
    }

    fn command() -> CreateAppTemplateOrderCommand {
        CreateAppTemplateOrderCommand::new(
            "1",
            Some("0"),
            "9",
            "template-1",
            "wechat_pay",
            "wechat_native",
            "order-1",
            "item-1",
            "AT1",
            "APPTEMPLATE1",
            "2026-07-26T00:00:00Z",
            "2026-07-26T00:30:00Z",
            "idem-1",
            None,
            None,
        )
        .expect("app template command")
    }

    #[test]
    fn order_item_snapshot_keeps_settlement_on_the_app_template_subject() {
        let snapshot = order_item_snapshot_json(&catalog(), &command(), "wechat_pay");
        let value: serde_json::Value = serde_json::from_str(&snapshot).expect("snapshot json");

        assert_eq!(value["templateUuid"], "template-1");
        assert_eq!(value["versionUuid"], "version-1");
        // The settlement subject resolver prefers these keys over the stored
        // subject; carrying one would route the order to the external handler.
        assert!(value.get("product_type").is_none());
        assert!(value.get("fulfillment_type").is_none());
        assert_eq!(
            stable_order_settlement_subject(Some(APP_TEMPLATE_ORDER_SUBJECT), Some(&snapshot)),
            APP_TEMPLATE_ORDER_SUBJECT
        );
    }

    #[test]
    fn catalog_queries_read_the_deployments_owned_tables() {
        assert!(LOAD_TEMPLATE.contains("FROM deploy_app_template t"));
        assert!(LOAD_TEMPLATE.contains("deploy_app_template_version v"));
        // The listing must be purchasable, not merely present.
        assert!(LOAD_TEMPLATE.contains("v.status = 'PUBLISHED'"));
        assert!(LOAD_TEMPLATE.contains("t.deleted_at IS NULL"));
    }

    #[test]
    fn order_queries_read_the_snapshot_portably() {
        // TEXT in the published baseline, JSONB in test fixtures: both have to
        // render through the row's JSON form.
        for sql in [LOAD_ORDER, LOAD_SETTLED_ORDER] {
            assert!(sql.contains("to_jsonb(oi) ->> 'sku_snapshot_json'"));
            assert!(!sql.contains("NULLIF(oi.sku_snapshot_json"));
        }
    }

    #[test]
    fn settled_statuses_match_the_entitlement_predicate() {
        assert!(
            LOAD_SETTLED_ORDER.contains(SETTLED_ORDER_STATUS_PREDICATE),
            "the owned-template lookup must use the settled status predicate"
        );
        let summary = AppTemplateOrderSummary {
            amount: CommerceMoney::new("0").expect("amount"),
            created_at: "2026-07-26T00:00:00.000Z".to_owned(),
            currency_code: "CNY".to_owned(),
            fulfillment_status: "fulfilled".to_owned(),
            order_id: "order-1".to_owned(),
            order_no: "AT1".to_owned(),
            paid_at: None,
            status: "paid".to_owned(),
            template_name: "Delivery console".to_owned(),
            template_uuid: "template-1".to_owned(),
            version_uuid: None,
        };
        assert!(summary.is_entitled());
    }

    #[test]
    fn catalog_price_is_read_as_canonical_minor_units() {
        // `deploy_app_template.price_minor` is already the smallest unit, so a
        // 99.00 listing is stored as 9900 and must come back as 9900.
        let normalized =
            crate::money_amount::normalize_stored_money("9900").expect("canonical minor units");
        let money = CommerceMoney::new(&normalized).expect("money");
        assert_eq!(money.as_str(), "9900");
    }

    #[test]
    fn cashier_url_targets_the_virtual_scene() {
        let url = app_template_cashier_url("order-1", "APPTEMPLATE1");
        assert!(url.contains("scene=virtual"));
        assert!(url.contains("/cashier/order-1"));
    }

    #[test]
    fn organization_scope_falls_back_to_the_platform_sentinel() {
        assert_eq!(normalize_organization_scope(None), "0");
        assert_eq!(normalize_organization_scope(Some("  ")), "0");
        assert_eq!(normalize_organization_scope(Some("12")), "12");
    }
}
