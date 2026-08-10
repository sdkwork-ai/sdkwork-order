use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_order_service::{CreateMembershipOrderCommand, CreateMembershipOrderOutcome};
use sdkwork_utils_rust::{build_commerce_cashier_url, commerce_cashier_scene};
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::membership_order_identity::{
    ensure_membership_request_fingerprint_matches, membership_order_request_fingerprint,
    membership_purchase_intent_key,
};
use crate::recharge_platform_catalog::materialize_platform_catalog_sql;

const PLATFORM_ORGANIZATION_SCOPE_SENTINEL: &str = "0";

/// 订阅期额度充值的合成包标识（不依赖目录套餐）。
const MEMBERSHIP_QUOTA_RECHARGE_PACKAGE_ID: &str = "membership-quota-recharge";

/// 订阅期额度充值合成包：数量与金额由请求声明，无时长。
fn recharge_membership_package(command: &CreateMembershipOrderCommand) -> MembershipPackageCatalog {
    MembershipPackageCatalog {
        package_external_id: MEMBERSHIP_QUOTA_RECHARGE_PACKAGE_ID.to_owned(),
        package_name: "Membership quota recharge".to_owned(),
        price_amount: command
            .amount
            .as_deref()
            .and_then(|amount| CommerceMoney::new(amount).ok())
            .expect("membership quota recharge amount is validated by the command"),
        currency_code: "CNY".to_owned(),
        duration_days: 0,
        sku_id: MEMBERSHIP_QUOTA_RECHARGE_PACKAGE_ID.to_owned(),
        product_name: "Membership quota recharge".to_owned(),
    }
}

fn catalog_sql(template: &'static str) -> String {
    materialize_platform_catalog_sql(template)
}

const LOAD_MEMBERSHIP_PACKAGE_BY_EXTERNAL_ID: &str = r#"
SELECT
    CAST(p.external_id AS TEXT) AS package_external_id,
    p.name AS package_name,
    CAST(p.price_amount AS TEXT) AS price_amount,
    COALESCE(NULLIF(p.currency_code, ''), 'CNY') AS currency_code,
    CAST(p.duration_days AS BIGINT) AS duration_days,
    p.sku_id,
    COALESCE(NULLIF(s.name, ''), NULLIF(s.title, ''), NULLIF(pr.title, ''), p.name) AS product_name
FROM membership_package p
JOIN membership_package_group g
    ON g.id = p.package_group_id
LEFT JOIN commerce_product_sku s
    ON s.id = p.sku_id
   AND COALESCE(
        NULLIF(to_jsonb(s) ->> 'sales_status', ''),
        NULLIF(to_jsonb(s) ->> 'status', ''),
        'active'
   ) = 'active'
LEFT JOIN commerce_product_spu pr
    ON pr.id = s.spu_id
   AND COALESCE(
        NULLIF(to_jsonb(pr) ->> 'sales_status', ''),
        NULLIF(to_jsonb(pr) ->> 'status', ''),
        'active'
   ) = 'active'
WHERE (
        (p.tenant_id = CAST($1 AS TEXT) AND p.organization_id = CAST($2 AS TEXT))
        OR (p.tenant_id = CAST($1 AS TEXT) AND p.organization_id = '0')
      )
  AND (
        (g.tenant_id = CAST($1 AS TEXT) AND g.organization_id = CAST($2 AS TEXT))
        OR (g.tenant_id = CAST($1 AS TEXT) AND g.organization_id = '0')
      )
  AND CAST(p.external_id AS TEXT) = $3
  AND p.status = 'active'
  AND g.status = 'active'
ORDER BY
    CASE
        WHEN p.tenant_id = CAST($1 AS TEXT) AND p.organization_id = CAST($2 AS TEXT) THEN 0
        WHEN p.tenant_id = CAST($1 AS TEXT) AND p.organization_id = '0' THEN 1
        ELSE 2
    END ASC,
    COALESCE(p.sort_weight, 0) ASC,
    p.id ASC
LIMIT 1
"#;

const LOAD_MEMBERSHIP_PACKAGE_BY_EXTERNAL_ID_PUBLIC: &str = r#"
SELECT
    CAST(p.external_id AS TEXT) AS package_external_id,
    p.name AS package_name,
    CAST(p.price_amount AS TEXT) AS price_amount,
    COALESCE(NULLIF(p.currency_code, ''), 'CNY') AS currency_code,
    CAST(p.duration_days AS BIGINT) AS duration_days,
    p.sku_id,
    COALESCE(NULLIF(s.name, ''), NULLIF(s.title, ''), NULLIF(pr.title, ''), p.name) AS product_name
FROM membership_package p
JOIN membership_package_group g
    ON g.id = p.package_group_id
LEFT JOIN commerce_product_sku s
    ON s.id = p.sku_id
   AND COALESCE(
        NULLIF(to_jsonb(s) ->> 'sales_status', ''),
        NULLIF(to_jsonb(s) ->> 'status', ''),
        'active'
   ) = 'active'
LEFT JOIN commerce_product_spu pr
    ON pr.id = s.spu_id
   AND COALESCE(
        NULLIF(to_jsonb(pr) ->> 'sales_status', ''),
        NULLIF(to_jsonb(pr) ->> 'status', ''),
        'active'
   ) = 'active'
WHERE p.tenant_id = '__PLATFORM_TENANT__'
  AND (p.organization_id = '0' OR p.organization_id = '0')
  AND (g.tenant_id = '__PLATFORM_TENANT__' OR g.tenant_id IS NULL)
  AND (g.organization_id = '0' OR g.organization_id = '0')
  AND CAST(p.external_id AS TEXT) = $1
  AND p.status = 'active'
  AND g.status = 'active'
ORDER BY COALESCE(p.sort_weight, 0) ASC, p.id ASC
LIMIT 1
"#;

#[derive(Debug, Clone)]
pub struct PostgresCommerceMembershipOrderStore {
    pool: PgPool,
}

#[derive(Debug, Clone)]
struct MembershipPackageCatalog {
    package_external_id: String,
    package_name: String,
    price_amount: CommerceMoney,
    currency_code: String,
    duration_days: i64,
    sku_id: String,
    product_name: String,
}

impl PostgresCommerceMembershipOrderStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_membership_order(
        &self,
        command: CreateMembershipOrderCommand,
    ) -> Result<CreateMembershipOrderOutcome, CommerceServiceError> {
        let mut tx =
            self.pool.begin().await.map_err(|error| {
                store_error("failed to begin membership order transaction", error)
            })?;

        let package = if command.action == "recharge" {
            recharge_membership_package(&command)
        } else {
            load_membership_package(&mut tx, &command).await?
        };
        // The H5 cashier can resolve its provider after the order is created.
        // Persist the requested method without requiring a pre-seeded method row.
        let method_key = normalize_method_key(&command.method);
        let request_fingerprint = membership_order_request_fingerprint(&command);
        let purchase_intent_key = membership_purchase_intent_key(
            &command,
            &package.sku_id,
            package.price_amount.as_str(),
            &package.currency_code,
            package.duration_days,
        );

        if let Some(outcome) =
            load_membership_order_in_tx(&mut tx, &command, Some(&command.idempotency_key), None)
                .await?
        {
            ensure_membership_request_fingerprint_matches(
                outcome.request_fingerprint.as_deref().unwrap_or_default(),
                &request_fingerprint,
            )?;
            tx.commit().await.map_err(|error| {
                store_error("failed to commit membership idempotency replay", error)
            })?;
            return Ok(outcome.value);
        }

        expire_stale_membership_orders(&mut tx, &command, &purchase_intent_key).await?;
        if let Some(outcome) =
            load_membership_order_in_tx(&mut tx, &command, None, Some(&purchase_intent_key)).await?
        {
            tx.commit().await.map_err(|error| {
                store_error("failed to commit reusable membership order", error)
            })?;
            return Ok(outcome.value);
        }

        let inserted = insert_membership_order(
            &mut tx,
            &command,
            &package,
            &request_fingerprint,
            &purchase_intent_key,
        )
        .await?;
        if !inserted {
            if let Some(outcome) =
                load_membership_order_in_tx(&mut tx, &command, Some(&command.idempotency_key), None)
                    .await?
            {
                ensure_membership_request_fingerprint_matches(
                    outcome.request_fingerprint.as_deref().unwrap_or_default(),
                    &request_fingerprint,
                )?;
                tx.commit().await.map_err(|error| {
                    store_error("failed to commit concurrent membership replay", error)
                })?;
                return Ok(outcome.value);
            }
            if let Some(outcome) =
                load_membership_order_in_tx(&mut tx, &command, None, Some(&purchase_intent_key))
                    .await?
            {
                tx.commit().await.map_err(|error| {
                    store_error("failed to commit concurrent membership reuse", error)
                })?;
                return Ok(outcome.value);
            }
            return Err(CommerceServiceError::conflict(
                "membership order creation conflicted with another request",
            ));
        }
        insert_membership_order_item(&mut tx, &command, &package, &method_key).await?;
        insert_membership_order_amount_breakdown(&mut tx, &command, &package).await?;

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit membership order transaction", error))?;

        Ok(build_membership_order_outcome(
            &command,
            &package,
            &method_key,
        ))
    }
}

struct StoredMembershipOrderOutcome {
    request_fingerprint: Option<String>,
    value: CreateMembershipOrderOutcome,
}

async fn load_membership_order_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateMembershipOrderCommand,
    idempotency_key: Option<&str>,
    purchase_intent_key: Option<&str>,
) -> Result<Option<StoredMembershipOrderOutcome>, CommerceServiceError> {
    let organization_id = normalize_organization_scope(command.organization_id.as_deref());
    let row = sqlx::query(
            r#"
            SELECT
                o.id AS order_id,
                o.order_no,
                COALESCE(NULLIF(o.request_no, ''), o.order_no) AS out_trade_no,
                o.request_fingerprint,
                COALESCE(NULLIF(o.membership_action, ''), $1) AS membership_action,
                CAST(o.expired_at AS TEXT) AS expires_at,
                CAST(COALESCE(ab.payable_amount, oi.total_amount, '0') AS TEXT) AS amount,
                COALESCE(NULLIF(ab.currency_code, ''), 'CNY') AS currency_code,
                COALESCE(
                    NULLIF(COALESCE(to_jsonb(oi) ->> 'sku_snapshot_json', '{}')::jsonb ->> 'packageId', ''),
                    CAST(mp.external_id AS TEXT),
                    $2
                ) AS package_id,
                COALESCE(
                    NULLIF(COALESCE(to_jsonb(oi) ->> 'sku_snapshot_json', '{}')::jsonb ->> 'productName', ''),
                    NULLIF(to_jsonb(oi) ->> 'title', ''),
                    NULLIF(to_jsonb(oi) ->> 'item_title', ''),
                    NULLIF(mp.name, ''),
                    'Membership package'
                ) AS package_name,
                CAST(COALESCE(
                    NULLIF(COALESCE(to_jsonb(oi) ->> 'sku_snapshot_json', '{}')::jsonb ->> 'durationDays', ''),
                    CAST(mp.duration_days AS TEXT),
                    '0'
                ) AS BIGINT) AS duration_days,
                $3 AS payment_method,
                o.status AS order_status
            FROM commerce_order o
            LEFT JOIN commerce_order_item oi
                ON oi.tenant_id = o.tenant_id
               AND oi.order_id = o.id
            LEFT JOIN commerce_order_amount_breakdown ab
                ON ab.tenant_id = o.tenant_id
               AND ab.order_id = o.id
            LEFT JOIN membership_package mp
                ON mp.sku_id = oi.sku_id
               AND CAST(mp.external_id AS TEXT) = $2
               AND mp.status = 'active'
            WHERE o.tenant_id = CAST($4 AS TEXT)
              AND ((o.organization_id = CAST($5 AS TEXT)) OR (o.organization_id IS NULL AND $5 IS NULL) OR (o.organization_id = '0' AND $5 IS NULL))
              AND o.owner_user_id = CAST($6 AS TEXT)
              AND o.subject = 'membership'
              AND (
                    ($7::text IS NOT NULL AND o.idempotency_key = CAST($7 AS TEXT))
                    OR
                    ($8::text IS NOT NULL
                     AND o.purchase_intent_key = CAST($8 AS TEXT)
                     AND o.status IN ('draft', 'pending', 'pending_payment', 'unpaid', 'wait_pay', 'created')
                     AND o.expired_at IS NOT NULL
                     AND NULLIF(o.expired_at, '')::timestamptz > $9::timestamptz)
                  )
            ORDER BY oi.created_at ASC NULLS LAST, oi.id ASC
            LIMIT 1
            "#,
        )
        .bind(&command.action)
        .bind(&command.package_id)
        .bind(normalize_method_key(&command.method))
        .bind(&command.tenant_id)
        .bind(&organization_id)
        .bind(&command.owner_user_id)
        .bind(idempotency_key)
        .bind(purchase_intent_key)
        .bind(&command.requested_at)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| store_error("failed to load reusable membership order", error))?;

    let Some(row) = row else {
        return Ok(None);
    };

    let order_no = string_cell(&row, "order_no");
    let out_trade_no = string_cell(&row, "out_trade_no");
    let amount = commerce_money_cell(&row, "amount", "membership order amount")?;
    // 订阅期额度充值无时长概念，回放时按 0 处理
    let duration_days = if string_cell(&row, "membership_action") == "recharge" {
        0
    } else {
        required_positive_integer_cell(&row, "duration_days")?
    };

    let value = CreateMembershipOrderOutcome::new(
        &string_cell(&row, "order_id"),
        &string_cell(&row, "membership_action"),
        &order_no,
        &out_trade_no,
        amount,
        &string_cell(&row, "currency_code"),
        &string_cell(&row, "package_id"),
        &string_cell(&row, "package_name"),
        duration_days,
        &string_cell(&row, "expires_at"),
        &string_cell(&row, "payment_method"),
        membership_order_status_label(&string_cell(&row, "order_status")),
        true,
        &membership_cashier_url(&string_cell(&row, "order_id"), &out_trade_no),
    )?;
    Ok(Some(StoredMembershipOrderOutcome {
        request_fingerprint: optional_string_cell(&row, "request_fingerprint"),
        value,
    }))
}

async fn expire_stale_membership_orders(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateMembershipOrderCommand,
    purchase_intent_key: &str,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        UPDATE commerce_order
        SET status = 'expired', payment_status = 'expired', updated_at = CAST($1 AS TEXT)
        WHERE tenant_id = CAST($2 AS TEXT)
          AND organization_id = CAST($3 AS TEXT)
          AND owner_user_id = CAST($4 AS TEXT)
          AND subject = 'membership'
          AND purchase_intent_key = CAST($5 AS TEXT)
          AND status IN ('draft', 'pending', 'pending_payment', 'unpaid', 'wait_pay', 'created')
          AND expired_at IS NOT NULL
          AND NULLIF(expired_at, '')::timestamptz <= $6::timestamptz
        "#,
    )
    .bind(&command.requested_at)
    .bind(&command.tenant_id)
    .bind(normalize_organization_scope(
        command.organization_id.as_deref(),
    ))
    .bind(&command.owner_user_id)
    .bind(purchase_intent_key)
    .bind(&command.requested_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to expire stale membership orders", error))?;
    Ok(())
}

async fn load_membership_package(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateMembershipOrderCommand,
) -> Result<MembershipPackageCatalog, CommerceServiceError> {
    let organization_id = normalize_organization_scope(command.organization_id.as_deref());
    let row = if command.tenant_id.trim().is_empty() {
        sqlx::query(sqlx::AssertSqlSafe(catalog_sql(LOAD_MEMBERSHIP_PACKAGE_BY_EXTERNAL_ID_PUBLIC)))
            .bind(&command.package_id)
            .fetch_optional(&mut **tx)
            .await
    } else {
        let scoped_row = sqlx::query(LOAD_MEMBERSHIP_PACKAGE_BY_EXTERNAL_ID)
            .bind(&command.tenant_id)
            .bind(&organization_id)
            .bind(&command.package_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|error| store_error("failed to load membership package", error))?;
        if scoped_row.is_some() {
            Ok(scoped_row)
        } else {
            sqlx::query(sqlx::AssertSqlSafe(catalog_sql(LOAD_MEMBERSHIP_PACKAGE_BY_EXTERNAL_ID_PUBLIC)))
                .bind(&command.package_id)
                .fetch_optional(&mut **tx)
                .await
        }
    }
    .map_err(|error| store_error("failed to load membership package", error))?
    .ok_or_else(|| CommerceServiceError::conflict("membership package is unavailable"))?;

    map_membership_package_row(&row)
}

fn map_membership_package_row(
    row: &sqlx::postgres::PgRow,
) -> Result<MembershipPackageCatalog, CommerceServiceError> {
    let sku_id = string_cell(row, "sku_id");
    if sku_id.trim().is_empty() {
        return Err(CommerceServiceError::conflict(
            "membership package product sku is unavailable",
        ));
    }

    Ok(MembershipPackageCatalog {
        package_external_id: string_cell(row, "package_external_id"),
        package_name: string_cell(row, "package_name"),
        price_amount: commerce_money_cell(row, "price_amount", "membership package price amount")?,
        currency_code: string_cell(row, "currency_code")
            .trim()
            .to_ascii_uppercase(),
        duration_days: required_positive_integer_cell(row, "duration_days")?,
        sku_id,
        product_name: string_cell(row, "product_name"),
    })
}

async fn insert_membership_order(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateMembershipOrderCommand,
    package: &MembershipPackageCatalog,
    request_fingerprint: &str,
    purchase_intent_key: &str,
) -> Result<bool, CommerceServiceError> {
    let organization_id = normalize_organization_scope(command.organization_id.as_deref());
    let payload = serde_json::json!({
        "id": command.order_id,
        "tenant_id": command.tenant_id,
        "organization_id": organization_id,
        "owner_user_id": command.owner_user_id,
        "order_no": command.order_no,
        "order_type": "membership",
        "subject": "membership",
        "status": "pending_payment",
        "pay_status": "pending",
        "payment_status": "pending",
        "fulfillment_status": "unfulfilled",
        "refund_status": "none",
        "total_amount": package.price_amount.as_str(),
        "currency_code": package.currency_code,
        "request_no": command.out_trade_no,
        "idempotency_key": command.idempotency_key,
        "request_fingerprint": request_fingerprint,
        "purchase_intent_key": purchase_intent_key,
        "membership_action": command.action,
        "created_at": command.requested_at,
        "paid_at": null,
        "cancelled_at": null,
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
    .map_err(|error| store_error("failed to insert membership order", error))?;
    Ok(result.rows_affected() == 1)
}

async fn insert_membership_order_item(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateMembershipOrderCommand,
    package: &MembershipPackageCatalog,
    payment_method: &str,
) -> Result<(), CommerceServiceError> {
    let snapshot = membership_order_item_snapshot_json(package, command, payment_method);
    let payload = serde_json::json!({
        "id": command.order_item_id,
        "tenant_id": command.tenant_id,
        "organization_id": normalize_organization_scope(command.organization_id.as_deref()),
        "order_id": command.order_id,
        "sku_id": package.sku_id,
        "sku_snapshot_json": snapshot,
        "title": package.package_name,
        "item_title": package.package_name,
        "quantity": 1,
        "unit_price_amount": package.price_amount.as_str(),
        // Amount columns store canonical integer smallest-unit strings; the
        // owner list/detail readers parse them with CommerceMoney::new.
        "discount_amount": "0",
        "tax_amount": "0",
        "total_amount": package.price_amount.as_str(),
        "fulfillment_status": "unfulfilled",
        "refund_status": "none",
        "created_at": command.requested_at,
    });
    sqlx::query(
        r#"
        INSERT INTO commerce_order_item
        SELECT * FROM jsonb_populate_record(NULL::commerce_order_item, $1::jsonb)
        "#,
    )
    .bind(payload.to_string())
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert membership order item", error))?;
    Ok(())
}

async fn insert_membership_order_amount_breakdown(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateMembershipOrderCommand,
    package: &MembershipPackageCatalog,
) -> Result<(), CommerceServiceError> {
    let payload = serde_json::json!({
        "id": format!("{}-amount", command.order_id),
        "tenant_id": command.tenant_id,
        "organization_id": normalize_organization_scope(command.organization_id.as_deref()),
        "order_id": command.order_id,
        "order_item_id": null,
        "allocation_type": "order_total",
        "original_amount": package.price_amount.as_str(),
        // Canonical integer smallest-unit string; the owner order list parses
        // discount_amount with CommerceMoney::new and rejects decimals.
        "discount_amount": "0",
        "payable_amount": package.price_amount.as_str(),
        "currency_code": package.currency_code,
        "created_at": command.requested_at,
        "updated_at": command.requested_at,
    });
    sqlx::query(
        r#"
        INSERT INTO commerce_order_amount_breakdown
        SELECT * FROM jsonb_populate_record(NULL::commerce_order_amount_breakdown, $1::jsonb)
        "#,
    )
    .bind(payload.to_string())
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert membership order amount breakdown", error))?;
    Ok(())
}

fn membership_order_item_snapshot_json(
    package: &MembershipPackageCatalog,
    command: &CreateMembershipOrderCommand,
    payment_method: &str,
) -> String {
    let mut snapshot = serde_json::json!({
        "skuId": package.sku_id,
        "productName": package.product_name,
        "packageId": package.package_external_id,
        "durationDays": package.duration_days,
        "clientRequestNo": command.client_request_no,
        "source": command.source,
        "action": command.action,
        "paymentMethod": payment_method,
        "paymentProduct": command.payment_product,
    });
    // 订阅期额度充值：快照携带充值数量与金额（结算分发时使用）
    if let Some(quantity) = command.grant_quantity {
        snapshot["grantQuantity"] = serde_json::json!(quantity);
        snapshot["rechargeAmount"] = serde_json::json!(command.amount);
    }
    snapshot.to_string()
}

fn build_membership_order_outcome(
    command: &CreateMembershipOrderCommand,
    package: &MembershipPackageCatalog,
    payment_method: &str,
) -> CreateMembershipOrderOutcome {
    CreateMembershipOrderOutcome::new(
        &command.order_id,
        &command.action,
        &command.order_no,
        &command.out_trade_no,
        package.price_amount.clone(),
        &package.currency_code,
        &package.package_external_id,
        &package.package_name,
        package.duration_days,
        &command.expire_at,
        payment_method,
        "pending_payment",
        false,
        &membership_cashier_url(&command.order_id, &command.out_trade_no),
    )
    .expect("membership order outcome should be valid")
}

fn membership_cashier_url(order_id: &str, out_trade_no: &str) -> String {
    build_commerce_cashier_url(
        commerce_cashier_scene(Some("membership")),
        order_id,
        out_trade_no,
    )
}

fn membership_order_status_label(status: &str) -> &str {
    match status.trim().to_ascii_lowercase().as_str() {
        "paid" | "fulfilled" | "completed" => "paid",
        "cancelled" | "canceled" | "expired" => "closed",
        _ => "pending_payment",
    }
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

fn commerce_money_cell(
    row: &sqlx::postgres::PgRow,
    column: &str,
    field_name: &str,
) -> Result<CommerceMoney, CommerceServiceError> {
    let value = string_cell(row, column);
    let cents = money_cents(&value)
        .map_err(|_| CommerceServiceError::storage(format!("invalid {field_name}: {value}")))?;
    CommerceMoney::new(&cents.to_string())
        .map_err(|message| CommerceServiceError::storage(format!("{message}: {value}")))
}

fn money_cents(amount: &str) -> Result<i64, CommerceServiceError> {
    let value = amount.trim();
    let mut parts = value.split('.');
    let whole = parts
        .next()
        .unwrap_or_default()
        .parse::<i64>()
        .map_err(|_| {
            CommerceServiceError::storage(format!("invalid commerce money amount: {value}"))
        })?;
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some() || fraction.len() > 2 {
        return Err(CommerceServiceError::storage(format!(
            "invalid commerce money amount: {value}"
        )));
    }
    let mut padded = fraction.to_string();
    while padded.len() < 2 {
        padded.push('0');
    }
    let cents = if padded.is_empty() {
        0
    } else {
        padded.parse::<i64>().map_err(|_| {
            CommerceServiceError::storage(format!("invalid commerce money amount: {value}"))
        })?
    };
    whole
        .checked_mul(100)
        .and_then(|amount| amount.checked_add(cents))
        .ok_or_else(|| {
            CommerceServiceError::storage(format!("invalid commerce money amount: {value}"))
        })
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}

fn required_positive_integer_cell(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<i64, CommerceServiceError> {
    let value = row
        .try_get::<Option<i64>, _>(column)
        .ok()
        .flatten()
        .or_else(|| {
            row.try_get::<Option<i32>, _>(column)
                .ok()
                .flatten()
                .map(i64::from)
        })
        .or_else(|| {
            optional_string_cell(row, column).and_then(|value| value.trim().parse::<i64>().ok())
        })
        .ok_or_else(|| CommerceServiceError::storage(format!("invalid integer column {column}")))?;
    if value <= 0 {
        return Err(CommerceServiceError::storage(format!(
            "integer column {column} must be greater than zero"
        )));
    }
    Ok(value)
}

fn store_error(context: &str, error: sqlx::Error) -> CommerceServiceError {
    crate::sql_store_error::map_sqlx_store_error(context, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_membership_cashier_url_uses_virtual_scene() {
        let url = membership_cashier_url("MB123", "MEMBERSHIP123");
        assert!(url.contains("scene=virtual"));
        assert!(url.contains("/cashier/MB123"));
    }

    #[test]
    fn postgres_membership_sql_supports_legacy_and_current_commerce_rows() {
        assert!(LOAD_MEMBERSHIP_PACKAGE_BY_EXTERNAL_ID.contains("to_jsonb(s)"));
        assert!(LOAD_MEMBERSHIP_PACKAGE_BY_EXTERNAL_ID.contains("sales_status"));
        assert!(LOAD_MEMBERSHIP_PACKAGE_BY_EXTERNAL_ID.contains("status"));

        let source = include_str!("postgres_membership_order.rs");
        assert!(source.contains("jsonb_populate_record(NULL::commerce_order,"));
        assert!(source.contains("jsonb_populate_record(NULL::commerce_order_item,"));
        assert!(source.contains("jsonb_populate_record(NULL::commerce_order_amount_breakdown,"));
        assert!(source.contains("to_jsonb(oi) ->> 'sku_snapshot_json'"));
        assert!(source.contains("to_jsonb(oi) ->> 'item_title'"));
        assert!(source.contains("NULLIF(o.expired_at, '')::timestamptz > $9::timestamptz"));
        assert!(source.contains("NULLIF(expired_at, '')::timestamptz <= $6::timestamptz"));
    }

    #[test]
    fn postgres_membership_catalog_queries_resolve_the_platform_tenant_placeholder() {
        let sql = catalog_sql(LOAD_MEMBERSHIP_PACKAGE_BY_EXTERNAL_ID_PUBLIC);
        assert!(!sql.contains("__PLATFORM_TENANT__"));
    }
}
