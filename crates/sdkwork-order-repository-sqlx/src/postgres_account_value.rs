use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{SecondsFormat, Utc};
use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_order_service::{
    AccountValueAssetCode, AccountValueCatalogListQuery, AccountValueFuture,
    AccountValueOrderSubject, AccountValuePackageItem, AccountValuePackageListPage,
    AccountValueRequestDetailQuery, AccountValueRequestExecutionStore, AccountValueRequestListPage,
    AccountValueRequestListQuery, AccountValueRequestStatusCommand, AccountValueRequestView,
    CouponRedemptionBenefit, CreateAccountRechargeOrderCommand, CreateAccountRechargeOrderOutcome,
    CreateCashWithdrawalRequestCommand, CreateCouponRechargeOrderCommand,
    CreateOrderRefundRequestCommand, RetireAccountValuePackageCommand, RetireTokenBankPlanCommand,
    ReviewAccountValueRequestCommand, TokenBankPlanItem, TokenBankPlanListPage,
    TokenBankPlanPeriod, UpsertAccountValuePackageCommand, UpsertTokenBankPlanCommand,
};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::postgres_recharge::PostgresCommerceRechargeStore;

const LIST_ACCOUNT_VALUE_PACKAGES: &str = r#"
SELECT
    id,
    package_code,
    display_name,
    target_asset,
    CAST(grant_amount AS TEXT) AS grant_amount,
    CAST(COALESCE(bonus_amount, '0') AS TEXT) AS bonus_amount,
    CAST(price_amount AS TEXT) AS price_amount,
    currency_code,
    status,
    COUNT(*) OVER() AS total_count
FROM commerce_account_value_package
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
  AND (CAST($3 AS TEXT) IS NULL OR target_asset = CAST($3 AS TEXT))
  AND (CAST($4 AS TEXT) IS NULL OR status = CAST($4 AS TEXT))
ORDER BY sort_weight ASC, id ASC
LIMIT $5 OFFSET $6
"#;

const LOAD_ACCOUNT_VALUE_PACKAGE_BY_IDEMPOTENCY: &str = r#"
SELECT
    id,
    package_code,
    display_name,
    target_asset,
    CAST(grant_amount AS TEXT) AS grant_amount,
    CAST(COALESCE(bonus_amount, '0') AS TEXT) AS bonus_amount,
    CAST(price_amount AS TEXT) AS price_amount,
    currency_code,
    status
FROM commerce_account_value_package
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
  AND idempotency_key = CAST($3 AS TEXT)
LIMIT 1
"#;

const LOAD_ACCOUNT_VALUE_PACKAGE_BY_ID_OR_CODE: &str = r#"
SELECT
    id,
    package_code,
    display_name,
    target_asset,
    CAST(grant_amount AS TEXT) AS grant_amount,
    CAST(COALESCE(bonus_amount, '0') AS TEXT) AS bonus_amount,
    CAST(price_amount AS TEXT) AS price_amount,
    currency_code,
    status
FROM commerce_account_value_package
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
  AND (id = CAST($3 AS TEXT) OR package_code = CAST($4 AS TEXT))
ORDER BY CASE WHEN id = CAST($3 AS TEXT) THEN 0 ELSE 1 END ASC
LIMIT 1
"#;

const LIST_TOKEN_BANK_PLANS: &str = r#"
SELECT
    plan_code,
    display_name,
    plan_period,
    CAST(grant_amount AS TEXT) AS grant_amount,
    CAST(COALESCE(bonus_amount, '0') AS TEXT) AS bonus_amount,
    CAST(price_amount AS TEXT) AS price_amount,
    currency_code,
    renewal_policy,
    status,
    COUNT(*) OVER() AS total_count
FROM commerce_token_bank_plan
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
  AND (CAST($3 AS TEXT) IS NULL OR status = CAST($3 AS TEXT))
ORDER BY sort_weight ASC, plan_code ASC
LIMIT $4 OFFSET $5
"#;

const LOAD_TOKEN_BANK_PLAN_BY_IDEMPOTENCY: &str = r#"
SELECT
    plan_code,
    display_name,
    plan_period,
    CAST(grant_amount AS TEXT) AS grant_amount,
    CAST(COALESCE(bonus_amount, '0') AS TEXT) AS bonus_amount,
    CAST(price_amount AS TEXT) AS price_amount,
    currency_code,
    renewal_policy,
    status
FROM commerce_token_bank_plan
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
  AND idempotency_key = CAST($3 AS TEXT)
LIMIT 1
"#;

const LOAD_TOKEN_BANK_PLAN_BY_CODE: &str = r#"
SELECT
    plan_code,
    display_name,
    plan_period,
    CAST(grant_amount AS TEXT) AS grant_amount,
    CAST(COALESCE(bonus_amount, '0') AS TEXT) AS bonus_amount,
    CAST(price_amount AS TEXT) AS price_amount,
    currency_code,
    renewal_policy,
    status
FROM commerce_token_bank_plan
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
  AND plan_code = CAST($3 AS TEXT)
LIMIT 1
"#;

const LOAD_REFUND_REQUEST_BY_IDEMPOTENCY: &str = r#"
SELECT
    id,
    request_no,
    original_order_id,
    owner_user_id,
    target_asset,
    CAST(amount AS TEXT) AS amount,
    currency_code,
    CAST(provider_amount AS TEXT) AS provider_amount,
    provider_currency_code,
    status,
    provider_reference_id,
    account_effect_reference_id,
    created_at,
    updated_at
FROM commerce_order_refund_request
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
  AND owner_user_id = CAST($3 AS TEXT)
  AND idempotency_key = CAST($4 AS TEXT)
LIMIT 1
"#;

const LOAD_WITHDRAWAL_REQUEST_BY_IDEMPOTENCY: &str = r#"
SELECT
    id,
    request_no,
    owner_user_id,
    target_asset,
    CAST(amount AS TEXT) AS amount,
    currency_code,
    CAST(provider_amount AS TEXT) AS provider_amount,
    provider_currency_code,
    status,
    provider_reference_id,
    account_effect_reference_id,
    created_at,
    updated_at
FROM commerce_order_withdrawal_request
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
  AND owner_user_id = CAST($3 AS TEXT)
  AND idempotency_key = CAST($4 AS TEXT)
LIMIT 1
"#;

const LIST_REFUND_REQUESTS: &str = r#"
SELECT
    id,
    request_no,
    original_order_id,
    owner_user_id,
    target_asset,
    CAST(amount AS TEXT) AS amount,
    currency_code,
    CAST(provider_amount AS TEXT) AS provider_amount,
    provider_currency_code,
    status,
    provider_reference_id,
    account_effect_reference_id,
    created_at,
    updated_at,
    COUNT(*) OVER() AS total_count
FROM commerce_order_refund_request
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
  AND (CAST($3 AS TEXT) IS NULL OR owner_user_id = CAST($3 AS TEXT))
  AND (CAST($4 AS TEXT) IS NULL OR status = CAST($4 AS TEXT))
ORDER BY created_at DESC, id DESC
LIMIT $5 OFFSET $6
"#;

const LIST_WITHDRAWAL_REQUESTS: &str = r#"
SELECT
    id,
    request_no,
    owner_user_id,
    target_asset,
    CAST(amount AS TEXT) AS amount,
    currency_code,
    CAST(provider_amount AS TEXT) AS provider_amount,
    provider_currency_code,
    status,
    provider_reference_id,
    account_effect_reference_id,
    created_at,
    updated_at,
    COUNT(*) OVER() AS total_count
FROM commerce_order_withdrawal_request
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
  AND (CAST($3 AS TEXT) IS NULL OR owner_user_id = CAST($3 AS TEXT))
  AND (CAST($4 AS TEXT) IS NULL OR status = CAST($4 AS TEXT))
ORDER BY created_at DESC, id DESC
LIMIT $5 OFFSET $6
"#;

const LOAD_REFUND_REQUEST_BY_ID: &str = r#"
SELECT
    id,
    request_no,
    original_order_id,
    owner_user_id,
    target_asset,
    CAST(amount AS TEXT) AS amount,
    currency_code,
    CAST(provider_amount AS TEXT) AS provider_amount,
    provider_currency_code,
    status,
    provider_reference_id,
    account_effect_reference_id,
    created_at,
    updated_at
FROM commerce_order_refund_request
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
  AND (CAST($3 AS TEXT) IS NULL OR owner_user_id = CAST($3 AS TEXT))
  AND id = CAST($4 AS TEXT)
LIMIT 1
"#;

const LOAD_WITHDRAWAL_REQUEST_BY_ID: &str = r#"
SELECT
    id,
    request_no,
    owner_user_id,
    target_asset,
    CAST(amount AS TEXT) AS amount,
    currency_code,
    CAST(provider_amount AS TEXT) AS provider_amount,
    provider_currency_code,
    status,
    provider_reference_id,
    account_effect_reference_id,
    created_at,
    updated_at
FROM commerce_order_withdrawal_request
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
  AND (CAST($3 AS TEXT) IS NULL OR owner_user_id = CAST($3 AS TEXT))
  AND id = CAST($4 AS TEXT)
LIMIT 1
"#;

impl PostgresCommerceRechargeStore {
    pub async fn list_account_value_packages(
        &self,
        query: AccountValueCatalogListQuery,
    ) -> Result<AccountValuePackageListPage, CommerceServiceError> {
        let target_asset = query.target_asset.map(|value| value.as_str().to_string());
        let rows = sqlx::query(LIST_ACCOUNT_VALUE_PACKAGES)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(target_asset.as_deref())
            .bind(query.status.as_deref())
            .bind(query.limit())
            .bind(query.offset())
            .fetch_all(self.pool())
            .await
            .map_err(|error| store_error("failed to list account value packages", error))?;
        let total = rows.first().map(total_count_cell).unwrap_or(0);
        let items = rows
            .iter()
            .map(map_account_value_package)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AccountValuePackageListPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn upsert_account_value_package(
        &self,
        command: UpsertAccountValuePackageCommand,
    ) -> Result<AccountValuePackageItem, CommerceServiceError> {
        if let Some(item) = self
            .load_account_value_package_by_idempotency(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.idempotency_key,
            )
            .await?
        {
            return Ok(item);
        }

        let package_id = command
            .package_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = current_command_timestamp();
        let mut tx =
            self.pool().begin().await.map_err(|error| {
                store_error("failed to begin account value package upsert", error)
            })?;
        let updated = sqlx::query(
            r#"
            UPDATE commerce_account_value_package
            SET display_name = $1,
                target_asset = $2,
                grant_amount = $3,
                bonus_amount = $4,
                price_amount = $5,
                currency_code = $6,
                status = $7,
                sort_weight = $8,
                valid_from = $9,
                valid_to = $10,
                request_no = $11,
                idempotency_key = $12,
                updated_at = $13,
                retired_at = CASE WHEN $14 = 'retired' THEN COALESCE(retired_at, $15) ELSE NULL END
            WHERE tenant_id = CAST($16 AS TEXT)
              AND ((organization_id = CAST($17 AS TEXT)) OR (organization_id IS NULL AND $17 IS NULL) OR (organization_id = '0' AND $17 IS NULL))
              AND (id = CAST($18 AS TEXT) OR package_code = CAST($19 AS TEXT))
            "#,
        )
        .bind(&command.display_name)
        .bind(command.target_asset.as_str())
        .bind(command.grant_amount.as_str())
        .bind(command.bonus_amount.as_str())
        .bind(command.price_amount.as_str())
        .bind(&command.currency_code)
        .bind(&command.status)
        .bind(command.sort_weight)
        .bind(command.valid_from.as_deref())
        .bind(command.valid_to.as_deref())
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&command.status)
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&package_id)
        .bind(&command.package_code)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to update account value package", error))?
        .rows_affected();

        if updated == 0 {
            sqlx::query(
                r#"
                INSERT INTO commerce_account_value_package
                    (id, tenant_id, organization_id, package_code, display_name, target_asset,
                     grant_amount, bonus_amount, price_amount, currency_code, status, sort_weight,
                     valid_from, valid_to, request_no, idempotency_key, created_at, updated_at,
                     retired_at)
                VALUES
                    ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), $4, $5, $6, $7, $8, $9, $10, $11,
                     $12, $13, $14, $15, $16, $17, $18,
                     CASE WHEN $19 = 'retired' THEN $20 ELSE NULL END)
                "#,
            )
            .bind(&package_id)
            .bind(&command.tenant_id)
            .bind(command.organization_id.as_deref())
            .bind(&command.package_code)
            .bind(&command.display_name)
            .bind(command.target_asset.as_str())
            .bind(command.grant_amount.as_str())
            .bind(command.bonus_amount.as_str())
            .bind(command.price_amount.as_str())
            .bind(&command.currency_code)
            .bind(&command.status)
            .bind(command.sort_weight)
            .bind(command.valid_from.as_deref())
            .bind(command.valid_to.as_deref())
            .bind(&command.request_no)
            .bind(&command.idempotency_key)
            .bind(&now)
            .bind(&now)
            .bind(&command.status)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|error| store_error("failed to insert account value package", error))?;
        }

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit account value package upsert", error))?;
        self.load_account_value_package_by_id_or_code(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &package_id,
            &command.package_code,
        )
        .await?
        .ok_or_else(|| CommerceServiceError::storage("account value package was not persisted"))
    }

    pub async fn retire_account_value_package(
        &self,
        command: RetireAccountValuePackageCommand,
    ) -> Result<(), CommerceServiceError> {
        if self
            .load_account_value_package_by_idempotency(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.idempotency_key,
            )
            .await?
            .is_some()
        {
            return Ok(());
        }
        let now = current_command_timestamp();
        let updated = sqlx::query(
            r#"
            UPDATE commerce_account_value_package
            SET status = 'retired',
                request_no = $1,
                idempotency_key = $2,
                retired_at = COALESCE(retired_at, $3),
                updated_at = $4
            WHERE tenant_id = CAST($5 AS TEXT)
              AND ((organization_id = CAST($6 AS TEXT)) OR (organization_id IS NULL AND $6 IS NULL) OR (organization_id = '0' AND $6 IS NULL))
              AND id = CAST($7 AS TEXT)
            "#,
        )
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.package_id)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to retire account value package", error))?
        .rows_affected();
        if updated == 0 {
            return Err(CommerceServiceError::not_found(
                "account value package was not found",
            ));
        }
        Ok(())
    }

    pub async fn list_token_bank_plans(
        &self,
        query: AccountValueCatalogListQuery,
    ) -> Result<TokenBankPlanListPage, CommerceServiceError> {
        let status = query.status.as_deref().or(Some("active"));
        let rows = sqlx::query(LIST_TOKEN_BANK_PLANS)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(status)
            .bind(query.limit())
            .bind(query.offset())
            .fetch_all(self.pool())
            .await
            .map_err(|error| store_error("failed to list Token Bank plans", error))?;
        let total = rows.first().map(total_count_cell).unwrap_or(0);
        let items = rows
            .iter()
            .map(map_token_bank_plan)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(TokenBankPlanListPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn upsert_token_bank_plan(
        &self,
        command: UpsertTokenBankPlanCommand,
    ) -> Result<TokenBankPlanItem, CommerceServiceError> {
        if let Some(item) = self
            .load_token_bank_plan_by_idempotency(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.idempotency_key,
            )
            .await?
        {
            return Ok(item);
        }
        let now = current_command_timestamp();
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|error| store_error("failed to begin Token Bank plan upsert", error))?;
        let updated = sqlx::query(
            r#"
            UPDATE commerce_token_bank_plan
            SET display_name = $1,
                plan_period = $2,
                grant_amount = $3,
                bonus_amount = $4,
                price_amount = $5,
                currency_code = $6,
                renewal_policy = $7,
                status = $8,
                sort_weight = $9,
                request_no = $10,
                idempotency_key = $11,
                updated_at = $12,
                retired_at = CASE WHEN $13 = 'retired' THEN COALESCE(retired_at, $14) ELSE NULL END
            WHERE tenant_id = CAST($15 AS TEXT)
              AND ((organization_id = CAST($16 AS TEXT)) OR (organization_id IS NULL AND $16 IS NULL) OR (organization_id = '0' AND $16 IS NULL))
              AND plan_code = CAST($17 AS TEXT)
            "#,
        )
        .bind(&command.display_name)
        .bind(command.plan_period.as_str())
        .bind(command.grant_amount.as_str())
        .bind(command.bonus_amount.as_str())
        .bind(command.price_amount.as_str())
        .bind(&command.currency_code)
        .bind(&command.renewal_policy)
        .bind(&command.status)
        .bind(command.sort_weight)
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&command.status)
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.plan_code)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to update Token Bank plan", error))?
        .rows_affected();

        if updated == 0 {
            sqlx::query(
                r#"
                INSERT INTO commerce_token_bank_plan
                    (id, tenant_id, organization_id, plan_code, display_name, plan_period,
                     grant_amount, bonus_amount, price_amount, currency_code, renewal_policy,
                     status, sort_weight, request_no, idempotency_key, created_at, updated_at,
                     retired_at)
                VALUES
                    ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), $4, $5, $6, $7, $8, $9, $10, $11,
                     $12, $13, $14, $15, $16, $17, CASE WHEN $18 = 'retired' THEN $19 ELSE NULL END)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&command.tenant_id)
            .bind(command.organization_id.as_deref())
            .bind(&command.plan_code)
            .bind(&command.display_name)
            .bind(command.plan_period.as_str())
            .bind(command.grant_amount.as_str())
            .bind(command.bonus_amount.as_str())
            .bind(command.price_amount.as_str())
            .bind(&command.currency_code)
            .bind(&command.renewal_policy)
            .bind(&command.status)
            .bind(command.sort_weight)
            .bind(&command.request_no)
            .bind(&command.idempotency_key)
            .bind(&now)
            .bind(&now)
            .bind(&command.status)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|error| store_error("failed to insert Token Bank plan", error))?;
        }

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit Token Bank plan upsert", error))?;
        self.load_token_bank_plan_by_code(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.plan_code,
        )
        .await?
        .ok_or_else(|| CommerceServiceError::storage("Token Bank plan was not persisted"))
    }

    pub async fn retire_token_bank_plan(
        &self,
        command: RetireTokenBankPlanCommand,
    ) -> Result<(), CommerceServiceError> {
        if self
            .load_token_bank_plan_by_idempotency(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.idempotency_key,
            )
            .await?
            .is_some()
        {
            return Ok(());
        }
        let now = current_command_timestamp();
        let updated = sqlx::query(
            r#"
            UPDATE commerce_token_bank_plan
            SET status = 'retired',
                request_no = $1,
                idempotency_key = $2,
                retired_at = COALESCE(retired_at, $3),
                updated_at = $4
            WHERE tenant_id = CAST($5 AS TEXT)
              AND ((organization_id = CAST($6 AS TEXT)) OR (organization_id IS NULL AND $6 IS NULL) OR (organization_id = '0' AND $6 IS NULL))
              AND plan_code = CAST($7 AS TEXT)
            "#,
        )
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.plan_code)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to retire Token Bank plan", error))?
        .rows_affected();
        if updated == 0 {
            return Err(CommerceServiceError::not_found(
                "Token Bank plan was not found",
            ));
        }
        Ok(())
    }

    pub async fn create_order_refund_request(
        &self,
        command: CreateOrderRefundRequestCommand,
    ) -> Result<AccountValueRequestView, CommerceServiceError> {
        if let Some(view) = self
            .load_refund_request_by_idempotency(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.owner_user_id,
                &command.idempotency_key,
            )
            .await?
        {
            self.validate_refund_request_replay(&command, &view).await?;
            return Ok(view);
        }
        // 退款申请仅对已支付订单开放（行业订单中心一致性：未支付/已取消/
        // 已关闭/已过期的订单不可发起资金退款）。
        let order_payment = sqlx::query_scalar::<_, String>(
            r#"
            SELECT COALESCE(NULLIF(payment_status, ''), 'none')
            FROM commerce_order
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3 IS NULL) OR (organization_id = '0' AND $3 IS NULL))
              AND owner_user_id = CAST($4 AS TEXT)
              AND id = CAST($5 AS TEXT)
            LIMIT 1
            "#,
        )
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&command.original_order_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to load order for refund request", error))?;
        match order_payment
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
        {
            Some(status) if matches!(status.as_str(), "success" | "succeeded" | "paid") => {}
            Some(_) => {
                return Err(CommerceServiceError::conflict(
                    "order is not paid; refund requests require a paid order",
                ))
            }
            None => {
                return Err(CommerceServiceError::not_found(
                    "order was not found for refund request",
                ))
            }
        }
        let now = current_command_timestamp();
        let inserted = sqlx::query(
            r#"
            INSERT INTO commerce_order_refund_request
                (id, tenant_id, organization_id, request_no, original_order_id, owner_user_id,
                 target_asset, amount, currency_code, provider_amount, provider_currency_code,
                 status, reason_code, reason_detail, review_comment, provider_reference_id,
                 account_effect_reference_id, idempotency_key, created_at, updated_at)
            VALUES
                ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), $4, $5, CAST($6 AS TEXT), $7, $8, $9,
                 $10, $11, 'requested', $12, $13, NULL, NULL, NULL, $14, $15, $16)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(&command.refund_request_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.request_no)
        .bind(&command.original_order_id)
        .bind(&command.owner_user_id)
        .bind(command.target_asset.as_str())
        .bind(command.amount.as_str())
        .bind(&command.currency_code)
        .bind(command.provider_amount.as_ref().map(CommerceMoney::as_str))
        .bind(command.provider_currency_code.as_deref())
        .bind(command.reason_code.as_deref())
        .bind(command.reason_detail.as_deref())
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&now)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to create refund request", error))?
        .rows_affected();
        if inserted == 0 {
            let view = self
                .load_refund_request_by_idempotency(
                    &command.tenant_id,
                    command.organization_id.as_deref(),
                    &command.owner_user_id,
                    &command.idempotency_key,
                )
                .await?
                .ok_or_else(|| {
                    CommerceServiceError::storage(
                        "refund request idempotency conflict has no persisted request",
                    )
                })?;
            self.validate_refund_request_replay(&command, &view).await?;
            return Ok(view);
        }
        self.retrieve_order_refund_request(AccountValueRequestDetailQuery {
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: Some(command.owner_user_id),
            subject: Some(AccountValueOrderSubject::RefundRequest),
            request_id: command.refund_request_id,
        })
        .await?
        .ok_or_else(|| CommerceServiceError::storage("refund request was not persisted"))
    }

    /// Admin-surface refund request creation.
    ///
    /// Safety contract against duplicate and over-refunds:
    /// 1. Replaying the same Idempotency-Key returns the persisted request
    ///    (idempotent replay validation first).
    /// 2. The order must be paid.
    /// 3. The cumulative in-flight refund amount plus this refund must not
    ///    exceed the order payable amount (evaluated in SQL).
    /// 4. `INSERT ... ON CONFLICT DO NOTHING` keeps the idempotency key
    ///    unique at the database level.
    pub async fn create_admin_order_refund_request(
        &self,
        command: CreateOrderRefundRequestCommand,
    ) -> Result<AccountValueRequestView, CommerceServiceError> {
        if let Some(view) = self
            .load_refund_request_by_idempotency(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.owner_user_id,
                &command.idempotency_key,
            )
            .await?
        {
            self.validate_refund_request_replay(&command, &view).await?;
            return Ok(view);
        }

        let bounds = sqlx::query(
            r#"
            SELECT
                COALESCE(NULLIF(o.payment_status, ''), 'none') AS payment_status,
                COALESCE(
                    (SELECT b.payable_amount FROM commerce_order_amount_breakdown b
                     WHERE b.tenant_id = o.tenant_id AND b.order_id = o.id
                       AND b.allocation_type = 'order_total'
                     LIMIT 1),
                    '0'
                )::NUMERIC
                >=
                COALESCE(
                    (SELECT SUM(CAST(r.amount AS NUMERIC)) FROM commerce_order_refund_request r
                     WHERE r.tenant_id = o.tenant_id AND r.original_order_id = o.id
                       AND r.status NOT IN ('rejected', 'provider_refund_failed')),
                    0
                ) + CAST($4 AS NUMERIC) AS within_refund_bounds
            FROM commerce_order o
            WHERE o.tenant_id = CAST($1 AS TEXT)
              AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id IS NULL AND $2 IS NULL) OR (o.organization_id = '0' AND $2 IS NULL))
              AND o.id = CAST($3 AS TEXT)
            LIMIT 1
            "#,
        )
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.original_order_id)
        .bind(command.amount.as_str())
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to load admin refund bounds", error))?;

        let (payment_status, within_refund_bounds) = match bounds {
            Some(row) => (
                row.get::<String, _>("payment_status"),
                row.get::<bool, _>("within_refund_bounds"),
            ),
            None => {
                return Err(CommerceServiceError::not_found(
                    "order was not found for refund request",
                ))
            }
        };
        if !matches!(
            payment_status.trim().to_ascii_lowercase().as_str(),
            "success" | "succeeded" | "paid"
        ) {
            return Err(CommerceServiceError::conflict(
                "order is not paid; refund requests require a paid order",
            ));
        }
        if !within_refund_bounds {
            return Err(CommerceServiceError::conflict(
                "refund amount exceeds the remaining refundable amount of the order",
            ));
        }

        let now = current_command_timestamp();
        let inserted = sqlx::query(
            r#"
            INSERT INTO commerce_order_refund_request
                (id, tenant_id, organization_id, request_no, original_order_id, owner_user_id,
                 target_asset, amount, currency_code, provider_amount, provider_currency_code,
                 status, reason_code, reason_detail, review_comment, provider_reference_id,
                 account_effect_reference_id, idempotency_key, created_at, updated_at)
            VALUES
                ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), $4, $5, CAST($6 AS TEXT), $7, $8, $9,
                 $10, $11, 'requested', $12, $13, NULL, NULL, NULL, $14, $15, $16)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(&command.refund_request_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.request_no)
        .bind(&command.original_order_id)
        .bind(&command.owner_user_id)
        .bind(command.target_asset.as_str())
        .bind(command.amount.as_str())
        .bind(&command.currency_code)
        .bind(command.provider_amount.as_ref().map(CommerceMoney::as_str))
        .bind(command.provider_currency_code.as_deref())
        .bind(command.reason_code.as_deref())
        .bind(command.reason_detail.as_deref())
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&now)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to create admin refund request", error))?
        .rows_affected();

        if inserted == 0 {
            let view = self
                .load_refund_request_by_idempotency(
                    &command.tenant_id,
                    command.organization_id.as_deref(),
                    &command.owner_user_id,
                    &command.idempotency_key,
                )
                .await?
                .ok_or_else(|| {
                    CommerceServiceError::storage(
                        "admin refund request idempotency conflict has no persisted request",
                    )
                })?;
            self.validate_refund_request_replay(&command, &view).await?;
            return Ok(view);
        }

        self.retrieve_order_refund_request(AccountValueRequestDetailQuery {
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: Some(command.owner_user_id),
            subject: Some(AccountValueOrderSubject::RefundRequest),
            request_id: command.refund_request_id,
        })
        .await?
        .ok_or_else(|| CommerceServiceError::storage("admin refund request was not persisted"))
    }

    pub async fn list_order_refund_requests(
        &self,
        query: AccountValueRequestListQuery,
    ) -> Result<AccountValueRequestListPage, CommerceServiceError> {
        let rows = sqlx::query(LIST_REFUND_REQUESTS)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.owner_user_id.as_deref())
            .bind(query.status.as_deref())
            .bind(query.limit())
            .bind(query.offset())
            .fetch_all(self.pool())
            .await
            .map_err(|error| store_error("failed to list refund requests", error))?;
        let total = rows.first().map(total_count_cell).unwrap_or(0);
        let items = rows
            .iter()
            .map(map_refund_request)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AccountValueRequestListPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn retrieve_order_refund_request(
        &self,
        query: AccountValueRequestDetailQuery,
    ) -> Result<Option<AccountValueRequestView>, CommerceServiceError> {
        let row = sqlx::query(LOAD_REFUND_REQUEST_BY_ID)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.owner_user_id.as_deref())
            .bind(&query.request_id)
            .fetch_optional(self.pool())
            .await
            .map_err(|error| store_error("failed to retrieve refund request", error))?;
        row.as_ref().map(map_refund_request).transpose()
    }

    pub async fn create_cash_withdrawal_request(
        &self,
        command: CreateCashWithdrawalRequestCommand,
    ) -> Result<AccountValueRequestView, CommerceServiceError> {
        if let Some(view) = self
            .load_withdrawal_request_by_idempotency(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.owner_user_id,
                &command.idempotency_key,
            )
            .await?
        {
            self.validate_withdrawal_request_replay(&command, &view)
                .await?;
            return Ok(view);
        }
        let now = current_command_timestamp();
        let inserted = sqlx::query(
            r#"
            INSERT INTO commerce_order_withdrawal_request
                (id, tenant_id, organization_id, request_no, owner_user_id, target_asset, amount,
                 currency_code, provider_amount, provider_currency_code, status, payout_method,
                 payout_account_ref, reason_code, review_comment, provider_reference_id,
                 account_effect_reference_id, idempotency_key, created_at, updated_at)
            VALUES
                ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), $4, CAST($5 AS TEXT), $6, $7, $8,
                 $9, $10, 'requested', $11, $12, $13, NULL, NULL, NULL, $14, $15, $16)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(&command.withdrawal_request_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.request_no)
        .bind(&command.owner_user_id)
        .bind(command.asset.as_str())
        .bind(command.amount.as_str())
        .bind(&command.currency_code)
        .bind(command.provider_amount.as_ref().map(CommerceMoney::as_str))
        .bind(command.provider_currency_code.as_deref())
        .bind(command.payout_method.as_deref())
        .bind(command.payout_account_ref.as_deref())
        .bind(command.reason_code.as_deref())
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&now)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to create withdrawal request", error))?
        .rows_affected();
        if inserted == 0 {
            let view = self
                .load_withdrawal_request_by_idempotency(
                    &command.tenant_id,
                    command.organization_id.as_deref(),
                    &command.owner_user_id,
                    &command.idempotency_key,
                )
                .await?
                .ok_or_else(|| {
                    CommerceServiceError::storage(
                        "withdrawal request idempotency conflict has no persisted request",
                    )
                })?;
            self.validate_withdrawal_request_replay(&command, &view)
                .await?;
            return Ok(view);
        }
        self.retrieve_cash_withdrawal_request(AccountValueRequestDetailQuery {
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: Some(command.owner_user_id),
            subject: Some(AccountValueOrderSubject::CashWithdrawal),
            request_id: command.withdrawal_request_id,
        })
        .await?
        .ok_or_else(|| CommerceServiceError::storage("withdrawal request was not persisted"))
    }

    pub async fn list_cash_withdrawal_requests(
        &self,
        query: AccountValueRequestListQuery,
    ) -> Result<AccountValueRequestListPage, CommerceServiceError> {
        let rows = sqlx::query(LIST_WITHDRAWAL_REQUESTS)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.owner_user_id.as_deref())
            .bind(query.status.as_deref())
            .bind(query.limit())
            .bind(query.offset())
            .fetch_all(self.pool())
            .await
            .map_err(|error| store_error("failed to list withdrawal requests", error))?;
        let total = rows.first().map(total_count_cell).unwrap_or(0);
        let items = rows
            .iter()
            .map(map_withdrawal_request)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AccountValueRequestListPage {
            items,
            page: query.page,
            page_size: query.page_size,
            total,
        })
    }

    pub async fn retrieve_cash_withdrawal_request(
        &self,
        query: AccountValueRequestDetailQuery,
    ) -> Result<Option<AccountValueRequestView>, CommerceServiceError> {
        let row = sqlx::query(LOAD_WITHDRAWAL_REQUEST_BY_ID)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.owner_user_id.as_deref())
            .bind(&query.request_id)
            .fetch_optional(self.pool())
            .await
            .map_err(|error| store_error("failed to retrieve withdrawal request", error))?;
        row.as_ref().map(map_withdrawal_request).transpose()
    }

    pub async fn review_account_value_request(
        &self,
        command: ReviewAccountValueRequestCommand,
    ) -> Result<AccountValueRequestView, CommerceServiceError> {
        match command.subject {
            AccountValueOrderSubject::RefundRequest => self.review_refund_request(command).await,
            AccountValueOrderSubject::CashWithdrawal => {
                self.review_withdrawal_request(command).await
            }
            _ => Err(CommerceServiceError::validation(
                "unsupported account value request subject",
            )),
        }
    }

    async fn update_account_value_request_status(
        &self,
        command: AccountValueRequestStatusCommand,
    ) -> Result<AccountValueRequestView, CommerceServiceError> {
        match command.subject {
            AccountValueOrderSubject::RefundRequest => {
                self.update_refund_request_status(command).await
            }
            AccountValueOrderSubject::CashWithdrawal => {
                self.update_withdrawal_request_status(command).await
            }
            _ => Err(CommerceServiceError::validation(
                "unsupported account value request subject",
            )),
        }
    }

    pub async fn create_account_recharge_order(
        &self,
        command: CreateAccountRechargeOrderCommand,
    ) -> Result<CreateAccountRechargeOrderOutcome, CommerceServiceError> {
        if let Some(outcome) = self
            .load_account_value_order_by_idempotency(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.owner_user_id,
                &command.idempotency_key,
            )
            .await?
        {
            return Ok(outcome);
        }
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|error| store_error("failed to begin account recharge order", error))?;
        insert_account_value_order(&mut tx, &command, false, None).await?;
        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit account recharge order", error))?;
        Ok(account_value_order_outcome_from_command(&command))
    }

    pub async fn create_coupon_recharge_order(
        &self,
        command: CreateCouponRechargeOrderCommand,
    ) -> Result<CreateAccountRechargeOrderOutcome, CommerceServiceError> {
        if let Some(outcome) = self
            .load_account_value_order_by_idempotency(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.owner_user_id,
                &command.idempotency_key,
            )
            .await?
        {
            return Ok(outcome);
        }
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|error| store_error("failed to begin coupon recharge order", error))?;
        insert_coupon_recharge_order(&mut tx, &command).await?;
        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit coupon recharge order", error))?;
        Ok(coupon_order_outcome_from_command(&command))
    }

    async fn load_account_value_package_by_idempotency(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        idempotency_key: &str,
    ) -> Result<Option<AccountValuePackageItem>, CommerceServiceError> {
        let row = sqlx::query(LOAD_ACCOUNT_VALUE_PACKAGE_BY_IDEMPOTENCY)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(idempotency_key)
            .fetch_optional(self.pool())
            .await
            .map_err(|error| {
                store_error("failed to load account value package idempotency", error)
            })?;
        row.as_ref().map(map_account_value_package).transpose()
    }

    async fn load_account_value_package_by_id_or_code(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        package_id: &str,
        package_code: &str,
    ) -> Result<Option<AccountValuePackageItem>, CommerceServiceError> {
        let row = sqlx::query(LOAD_ACCOUNT_VALUE_PACKAGE_BY_ID_OR_CODE)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(package_id)
            .bind(package_code)
            .fetch_optional(self.pool())
            .await
            .map_err(|error| store_error("failed to load account value package", error))?;
        row.as_ref().map(map_account_value_package).transpose()
    }

    async fn load_token_bank_plan_by_idempotency(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        idempotency_key: &str,
    ) -> Result<Option<TokenBankPlanItem>, CommerceServiceError> {
        let row = sqlx::query(LOAD_TOKEN_BANK_PLAN_BY_IDEMPOTENCY)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(idempotency_key)
            .fetch_optional(self.pool())
            .await
            .map_err(|error| store_error("failed to load Token Bank plan idempotency", error))?;
        row.as_ref().map(map_token_bank_plan).transpose()
    }

    async fn load_token_bank_plan_by_code(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        plan_code: &str,
    ) -> Result<Option<TokenBankPlanItem>, CommerceServiceError> {
        let row = sqlx::query(LOAD_TOKEN_BANK_PLAN_BY_CODE)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(plan_code)
            .fetch_optional(self.pool())
            .await
            .map_err(|error| store_error("failed to load Token Bank plan", error))?;
        row.as_ref().map(map_token_bank_plan).transpose()
    }

    async fn load_refund_request_by_idempotency(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<AccountValueRequestView>, CommerceServiceError> {
        let row = sqlx::query(LOAD_REFUND_REQUEST_BY_IDEMPOTENCY)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(owner_user_id)
            .bind(idempotency_key)
            .fetch_optional(self.pool())
            .await
            .map_err(|error| store_error("failed to load refund request idempotency", error))?;
        row.as_ref().map(map_refund_request).transpose()
    }

    async fn validate_refund_request_replay(
        &self,
        command: &CreateOrderRefundRequestCommand,
        view: &AccountValueRequestView,
    ) -> Result<(), CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT reason_code, reason_detail
            FROM commerce_order_refund_request
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
              AND owner_user_id = CAST($3 AS TEXT)
              AND id = CAST($4 AS TEXT)
            LIMIT 1
            "#,
        )
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&view.request_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to validate refund idempotency replay", error))?
        .ok_or_else(|| CommerceServiceError::storage("refund idempotency replay was not found"))?;
        let matches = view.request_id == command.refund_request_id
            && view.original_order_id.as_deref() == Some(command.original_order_id.as_str())
            && view.owner_user_id == command.owner_user_id
            && view.target_asset == command.target_asset
            && view.amount == command.amount
            && view.currency_code == command.currency_code
            && view.provider_amount == command.provider_amount
            && view.provider_currency_code == command.provider_currency_code
            && row
                .try_get::<Option<String>, _>("reason_code")
                .ok()
                .flatten()
                == command.reason_code
            && row
                .try_get::<Option<String>, _>("reason_detail")
                .ok()
                .flatten()
                == command.reason_detail;
        if !matches {
            return Err(CommerceServiceError::conflict(
                "idempotency key was used with a different refund request",
            ));
        }
        Ok(())
    }

    async fn load_withdrawal_request_by_idempotency(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<AccountValueRequestView>, CommerceServiceError> {
        let row = sqlx::query(LOAD_WITHDRAWAL_REQUEST_BY_IDEMPOTENCY)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(owner_user_id)
            .bind(idempotency_key)
            .fetch_optional(self.pool())
            .await
            .map_err(|error| store_error("failed to load withdrawal request idempotency", error))?;
        row.as_ref().map(map_withdrawal_request).transpose()
    }

    async fn validate_withdrawal_request_replay(
        &self,
        command: &CreateCashWithdrawalRequestCommand,
        view: &AccountValueRequestView,
    ) -> Result<(), CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT payout_method, payout_account_ref, reason_code
            FROM commerce_order_withdrawal_request
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL) OR (organization_id = '0' AND $2 IS NULL))
              AND owner_user_id = CAST($3 AS TEXT)
              AND id = CAST($4 AS TEXT)
            LIMIT 1
            "#,
        )
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&view.request_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to validate withdrawal idempotency replay", error))?
        .ok_or_else(|| {
            CommerceServiceError::storage("withdrawal idempotency replay was not found")
        })?;
        let matches = view.request_id == command.withdrawal_request_id
            && view.owner_user_id == command.owner_user_id
            && view.target_asset == command.asset
            && view.amount == command.amount
            && view.currency_code == command.currency_code
            && view.provider_amount == command.provider_amount
            && view.provider_currency_code == command.provider_currency_code
            && row
                .try_get::<Option<String>, _>("payout_method")
                .ok()
                .flatten()
                == command.payout_method
            && row
                .try_get::<Option<String>, _>("payout_account_ref")
                .ok()
                .flatten()
                == command.payout_account_ref
            && row
                .try_get::<Option<String>, _>("reason_code")
                .ok()
                .flatten()
                == command.reason_code;
        if !matches {
            return Err(CommerceServiceError::conflict(
                "idempotency key was used with a different withdrawal request",
            ));
        }
        Ok(())
    }

    async fn review_refund_request(
        &self,
        command: ReviewAccountValueRequestCommand,
    ) -> Result<AccountValueRequestView, CommerceServiceError> {
        let now = current_command_timestamp();
        let updated = sqlx::query(
            r#"
            UPDATE commerce_order_refund_request
            SET status = $1,
                reason_code = COALESCE($2, reason_code),
                review_comment = $3,
                updated_at = $4
            WHERE tenant_id = CAST($5 AS TEXT)
              AND ((organization_id = CAST($6 AS TEXT)) OR (organization_id IS NULL AND $6 IS NULL) OR (organization_id = '0' AND $6 IS NULL))
              AND id = CAST($7 AS TEXT)
            "#,
        )
        .bind(command.next_status())
        .bind(command.reason_code.as_deref())
        .bind(command.review_comment.as_deref())
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.request_id)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to review refund request", error))?
        .rows_affected();
        if updated == 0 {
            return Err(CommerceServiceError::not_found(
                "refund request was not found",
            ));
        }
        self.retrieve_order_refund_request(AccountValueRequestDetailQuery {
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: None,
            subject: Some(AccountValueOrderSubject::RefundRequest),
            request_id: command.request_id,
        })
        .await?
        .ok_or_else(|| {
            CommerceServiceError::storage("refund request was not persisted after review")
        })
    }

    async fn update_refund_request_status(
        &self,
        command: AccountValueRequestStatusCommand,
    ) -> Result<AccountValueRequestView, CommerceServiceError> {
        let now = current_command_timestamp();
        let updated = sqlx::query(
            r#"
            UPDATE commerce_order_refund_request
            SET status = $1,
                reason_code = COALESCE($2, reason_code),
                review_comment = COALESCE($3, review_comment),
                provider_reference_id = COALESCE($4, provider_reference_id),
                account_effect_reference_id = COALESCE($5, account_effect_reference_id),
                request_no = $6,
                idempotency_key = $7,
                updated_at = $8
            WHERE tenant_id = CAST($9 AS TEXT)
              AND ((organization_id = CAST($10 AS TEXT)) OR (organization_id IS NULL AND $10 IS NULL) OR (organization_id = '0' AND $10 IS NULL))
              AND id = CAST($11 AS TEXT)
            "#,
        )
        .bind(&command.status)
        .bind(command.reason_code.as_deref())
        .bind(command.review_comment.as_deref())
        .bind(command.provider_reference_id.as_deref())
        .bind(command.account_effect_reference_id.as_deref())
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.request_id)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to update refund request status", error))?
        .rows_affected();
        if updated == 0 {
            return Err(CommerceServiceError::not_found(
                "refund request was not found",
            ));
        }
        self.retrieve_order_refund_request(AccountValueRequestDetailQuery {
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: None,
            subject: Some(AccountValueOrderSubject::RefundRequest),
            request_id: command.request_id,
        })
        .await?
        .ok_or_else(|| {
            CommerceServiceError::storage("refund request was not persisted after status update")
        })
    }

    async fn review_withdrawal_request(
        &self,
        command: ReviewAccountValueRequestCommand,
    ) -> Result<AccountValueRequestView, CommerceServiceError> {
        let now = current_command_timestamp();
        let updated = sqlx::query(
            r#"
            UPDATE commerce_order_withdrawal_request
            SET status = $1,
                reason_code = COALESCE($2, reason_code),
                review_comment = $3,
                updated_at = $4
            WHERE tenant_id = CAST($5 AS TEXT)
              AND ((organization_id = CAST($6 AS TEXT)) OR (organization_id IS NULL AND $6 IS NULL) OR (organization_id = '0' AND $6 IS NULL))
              AND id = CAST($7 AS TEXT)
            "#,
        )
        .bind(command.next_status())
        .bind(command.reason_code.as_deref())
        .bind(command.review_comment.as_deref())
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.request_id)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to review withdrawal request", error))?
        .rows_affected();
        if updated == 0 {
            return Err(CommerceServiceError::not_found(
                "withdrawal request was not found",
            ));
        }
        self.retrieve_cash_withdrawal_request(AccountValueRequestDetailQuery {
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: None,
            subject: Some(AccountValueOrderSubject::CashWithdrawal),
            request_id: command.request_id,
        })
        .await?
        .ok_or_else(|| {
            CommerceServiceError::storage("withdrawal request was not persisted after review")
        })
    }

    async fn update_withdrawal_request_status(
        &self,
        command: AccountValueRequestStatusCommand,
    ) -> Result<AccountValueRequestView, CommerceServiceError> {
        let now = current_command_timestamp();
        let updated = sqlx::query(
            r#"
            UPDATE commerce_order_withdrawal_request
            SET status = $1,
                reason_code = COALESCE($2, reason_code),
                review_comment = COALESCE($3, review_comment),
                provider_reference_id = COALESCE($4, provider_reference_id),
                account_effect_reference_id = COALESCE($5, account_effect_reference_id),
                request_no = $6,
                idempotency_key = $7,
                updated_at = $8
            WHERE tenant_id = CAST($9 AS TEXT)
              AND ((organization_id = CAST($10 AS TEXT)) OR (organization_id IS NULL AND $10 IS NULL) OR (organization_id = '0' AND $10 IS NULL))
              AND id = CAST($11 AS TEXT)
            "#,
        )
        .bind(&command.status)
        .bind(command.reason_code.as_deref())
        .bind(command.review_comment.as_deref())
        .bind(command.provider_reference_id.as_deref())
        .bind(command.account_effect_reference_id.as_deref())
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.request_id)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to update withdrawal request status", error))?
        .rows_affected();
        if updated == 0 {
            return Err(CommerceServiceError::not_found(
                "withdrawal request was not found",
            ));
        }
        self.retrieve_cash_withdrawal_request(AccountValueRequestDetailQuery {
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: None,
            subject: Some(AccountValueOrderSubject::CashWithdrawal),
            request_id: command.request_id,
        })
        .await?
        .ok_or_else(|| {
            CommerceServiceError::storage(
                "withdrawal request was not persisted after status update",
            )
        })
    }

    pub async fn load_account_value_order_by_idempotency(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<CreateAccountRechargeOrderOutcome>, CommerceServiceError> {
        let organization_id = normalize_organization_scope(organization_id);
        let row = sqlx::query(
            r#"
            SELECT
                o.id AS order_id,
                o.order_no,
                COALESCE(NULLIF(COALESCE(NULLIF(oi.sku_snapshot_json, ''), '{}')::jsonb ->> 'outTradeNo', ''), o.order_no) AS out_trade_no,
                o.subject,
                COALESCE(NULLIF(COALESCE(NULLIF(oi.sku_snapshot_json, ''), '{}')::jsonb ->> 'targetAsset', ''), '') AS target_asset,
                CAST(COALESCE(b.payable_amount, '0') AS TEXT) AS amount,
                CAST(COALESCE(NULLIF(COALESCE(NULLIF(oi.sku_snapshot_json, ''), '{}')::jsonb ->> 'grantAmount', ''), '0') AS TEXT) AS grant_amount,
                o.currency_code,
                o.status
            FROM commerce_order o
            LEFT JOIN commerce_order_item oi
                ON oi.tenant_id = o.tenant_id AND oi.order_id = o.id
            LEFT JOIN commerce_order_amount_breakdown b
                ON b.tenant_id = o.tenant_id AND b.order_id = o.id AND b.allocation_type = 'order_total'
            WHERE o.tenant_id = CAST($1 AS TEXT)
              AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id IS NULL AND $2 IS NULL) OR (o.organization_id = '0' AND $2 IS NULL))
              AND o.owner_user_id = CAST($3 AS TEXT)
              AND o.idempotency_key = CAST($4 AS TEXT)
              AND o.subject IN ('token_bank_recharge', 'token_bank_plan_purchase', 'token_bank_plan_renewal', 'account_recharge_package', 'coupon_recharge')
            ORDER BY o.created_at DESC, o.id DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&organization_id)
        .bind(owner_user_id)
        .bind(idempotency_key)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to load account value order idempotency", error))?;
        row.as_ref()
            .map(map_account_value_order_outcome)
            .transpose()
    }
}

impl AccountValueRequestExecutionStore for PostgresCommerceRechargeStore {
    fn load_account_value_request_for_execution<'a>(
        &'a self,
        command: &'a ReviewAccountValueRequestCommand,
    ) -> AccountValueFuture<'a, Option<AccountValueRequestView>> {
        Box::pin(async move {
            match command.subject {
                AccountValueOrderSubject::RefundRequest => {
                    self.retrieve_order_refund_request(AccountValueRequestDetailQuery {
                        tenant_id: command.tenant_id.clone(),
                        organization_id: command.organization_id.clone(),
                        owner_user_id: None,
                        subject: Some(AccountValueOrderSubject::RefundRequest),
                        request_id: command.request_id.clone(),
                    })
                    .await
                }
                AccountValueOrderSubject::CashWithdrawal => {
                    self.retrieve_cash_withdrawal_request(AccountValueRequestDetailQuery {
                        tenant_id: command.tenant_id.clone(),
                        organization_id: command.organization_id.clone(),
                        owner_user_id: None,
                        subject: Some(AccountValueOrderSubject::CashWithdrawal),
                        request_id: command.request_id.clone(),
                    })
                    .await
                }
                _ => Err(CommerceServiceError::validation(
                    "unsupported account value request subject",
                )),
            }
        })
    }

    fn mark_account_value_request_status<'a>(
        &'a self,
        command: AccountValueRequestStatusCommand,
    ) -> AccountValueFuture<'a, AccountValueRequestView> {
        Box::pin(async move { self.update_account_value_request_status(command).await })
    }

    fn mark_owner_order_refunded<'a>(
        &'a self,
        tenant_id: &'a str,
        organization_id: Option<&'a str>,
        owner_user_id: &'a str,
        order_id: &'a str,
        refund_status: &'a str,
    ) -> AccountValueFuture<'a, ()> {
        let pool = self.pool().clone();
        Box::pin(async move {
            let result = sqlx::query(
                r#"
                UPDATE commerce_order
                SET refund_status = $1, updated_at = $2
                WHERE tenant_id = CAST($3 AS TEXT)
                  AND ((organization_id = CAST($4 AS TEXT)) OR (organization_id IS NULL AND $5 IS NULL) OR (organization_id = '0' AND $5 IS NULL))
                  AND owner_user_id = CAST($6 AS TEXT)
                  AND id = CAST($7 AS TEXT)
                "#,
            )
            .bind(refund_status)
            .bind(current_command_timestamp())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(organization_id)
            .bind(owner_user_id)
            .bind(order_id)
            .execute(&pool)
            .await
            .map_err(|error| store_error("failed to mark owner order refunded", error))?;
            if result.rows_affected() == 0 {
                return Err(CommerceServiceError::not_found(
                    "owner order was not found for refund sync",
                ));
            }
            Ok(())
        })
    }
}

async fn insert_account_value_order(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateAccountRechargeOrderCommand,
    coupon: bool,
    coupon_code: Option<&str>,
) -> Result<(), CommerceServiceError> {
    let title = account_value_order_title(command.subject);
    let organization_id = normalize_organization_scope(command.organization_id.as_deref());
    sqlx::query(
        r#"
        INSERT INTO commerce_order
            (id, tenant_id, organization_id, owner_user_id, order_no, status, payment_status,
             fulfillment_status, refund_status, subject, currency_code, request_no,
             idempotency_key, created_at, paid_at, cancelled_at, expired_at, updated_at)
        VALUES
            ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, 'pending_payment',
             'pending', 'unfulfilled', 'none', $6, $7, $8, $9, $10, NULL, NULL, $11, $12)
        "#,
    )
    .bind(&command.order_id)
    .bind(&command.tenant_id)
    .bind(&organization_id)
    .bind(&command.owner_user_id)
    .bind(&command.order_no)
    .bind(command.subject.as_str())
    .bind(&command.currency_code)
    .bind(&command.order_no)
    .bind(&command.idempotency_key)
    .bind(&command.requested_at)
    .bind(&command.expire_at)
    .bind(&command.requested_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert account value order", error))?;

    sqlx::query(
        r#"
        INSERT INTO commerce_order_item
            (id, tenant_id, order_id, sku_id, sku_snapshot_json, title, quantity, unit_price_amount,
             total_amount, fulfillment_status, refund_status, created_at)
        VALUES
            ($1, CAST($2 AS TEXT), $3, $4, $5, $6, 1, $7, $8, 'unfulfilled', 'none', $9)
        "#,
    )
    .bind(&command.order_item_id)
    .bind(&command.tenant_id)
    .bind(&command.order_id)
    .bind(account_value_sku_id(command, coupon_code))
    .bind(account_value_order_item_snapshot_json(
        command,
        coupon,
        coupon_code,
    ))
    .bind(title)
    .bind(command.amount.as_str())
    .bind(command.amount.as_str())
    .bind(&command.requested_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert account value order item", error))?;

    insert_account_value_amount_breakdown(
        tx,
        &command.tenant_id,
        Some(&organization_id),
        &command.order_id,
        command.amount.as_str(),
        &command.currency_code,
        &command.requested_at,
    )
    .await
}

async fn insert_coupon_recharge_order(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateCouponRechargeOrderCommand,
) -> Result<(), CommerceServiceError> {
    let now = rfc3339_command_timestamp();
    let organization_id = normalize_organization_scope(command.organization_id.as_deref());
    let expires_at = if command.payment_required {
        Some(rfc3339_after_payment_window(&now))
    } else {
        None
    };
    let order_status = if command.payment_required {
        "pending_payment"
    } else {
        "paid"
    };
    let payment_status = if command.payment_required {
        "pending"
    } else {
        "succeeded"
    };
    sqlx::query(
        r#"
        INSERT INTO commerce_order
            (id, tenant_id, organization_id, owner_user_id, order_no, status, payment_status,
             fulfillment_status, refund_status, subject, currency_code, request_no,
             idempotency_key, created_at, paid_at, cancelled_at, expired_at, updated_at)
        VALUES
            ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, $6, $7, 'unfulfilled',
             'none', 'coupon_recharge', $8, $9, $10, $11, NULL, NULL, $12, $13)
        "#,
    )
    .bind(&command.order_id)
    .bind(&command.tenant_id)
    .bind(&organization_id)
    .bind(&command.owner_user_id)
    .bind(&command.order_no)
    .bind(order_status)
    .bind(payment_status)
    .bind(&command.currency_code)
    .bind(&command.order_no)
    .bind(&command.idempotency_key)
    .bind(&now)
    .bind(expires_at.as_deref())
    .bind(&now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert coupon recharge order", error))?;

    let title = account_value_order_title(command.subject);
    let snapshot = serde_json::json!({
        "subject": command.subject.as_str(),
        "targetAsset": command.target_asset.as_str(),
        "assetCode": command.target_asset.as_str(),
        "assetUnitCode": command.target_asset.default_unit_code(),
        "grantAmount": command.grant_amount.as_str(),
        "bonusAmount": "0",
        "couponCode": command.coupon_code,
        "couponBenefit": coupon_benefit_snapshot(&command.benefit),
        "paymentRequired": command.payment_required,
        "outTradeNo": command.out_trade_no,
    })
    .to_string();
    sqlx::query(
        r#"
        INSERT INTO commerce_order_item
            (id, tenant_id, order_id, sku_id, sku_snapshot_json, title, quantity, unit_price_amount,
             total_amount, fulfillment_status, refund_status, created_at)
        VALUES
            ($1, CAST($2 AS TEXT), $3, $4, $5, $6, 1, $7, $8, 'unfulfilled', 'none', $9)
        "#,
    )
    .bind(&command.order_item_id)
    .bind(&command.tenant_id)
    .bind(&command.order_id)
    .bind(format!("coupon-{}", command.coupon_code))
    .bind(snapshot)
    .bind(title)
    .bind(command.amount.as_str())
    .bind(command.amount.as_str())
    .bind(&now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert coupon recharge order item", error))?;

    insert_account_value_amount_breakdown(
        tx,
        &command.tenant_id,
        Some(&organization_id),
        &command.order_id,
        command.amount.as_str(),
        &command.currency_code,
        &now,
    )
    .await
}

fn coupon_benefit_snapshot(benefit: &CouponRedemptionBenefit) -> serde_json::Value {
    match benefit {
        CouponRedemptionBenefit::TokenBankCredit { grant_amount } => serde_json::json!({
            "kind": "token_bank_credit",
            "targetAsset": "token_bank",
            "grantAmount": grant_amount.as_str(),
        }),
        CouponRedemptionBenefit::PointsCredit { grant_amount } => serde_json::json!({
            "kind": "points_credit",
            "grantPoints": grant_amount.as_str(),
        }),
        CouponRedemptionBenefit::CashCredit { grant_amount } => serde_json::json!({
            "kind": "cash_credit",
            "grantAmount": grant_amount.as_str(),
        }),
        CouponRedemptionBenefit::Subscription {
            product_id,
            sku_id,
            package_id,
            period,
            duration_days,
            daily_quota,
            total_quota,
        } => serde_json::json!({
            "kind": "subscription",
            "productId": product_id,
            "skuId": sku_id,
            "packageId": package_id.to_string(),
            "period": period.as_str(),
            "durationDays": duration_days,
            "dailyQuota": daily_quota.to_string(),
            "totalQuota": total_quota.to_string(),
        }),
    }
}

async fn insert_account_value_amount_breakdown(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
    amount: &str,
    currency_code: &str,
    created_at: &str,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        INSERT INTO commerce_order_amount_breakdown
            (id, tenant_id, organization_id, order_id, order_item_id, allocation_type,
             original_amount, discount_amount, payable_amount, currency_code, created_at)
        VALUES
            ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), $4, NULL, 'order_total', $5, '0', $6, $7, CAST($8 AS timestamptz))
        "#,
    )
    .bind(format!("{order_id}-amount"))
    .bind(tenant_id)
    .bind(organization_id)
    .bind(order_id)
    .bind(amount)
    .bind(amount)
    .bind(currency_code)
    .bind(created_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert account value amount breakdown", error))?;
    Ok(())
}

fn account_value_order_item_snapshot_json(
    command: &CreateAccountRechargeOrderCommand,
    coupon: bool,
    coupon_code: Option<&str>,
) -> String {
    serde_json::json!({
        "subject": command.subject.as_str(),
        "targetAsset": command.target_asset.as_str(),
        "assetCode": command.target_asset.as_str(),
        "assetUnitCode": command.target_asset.default_unit_code(),
        "grantAmount": command.grant_amount.as_str(),
        "bonusAmount": "0",
        "packageId": command.package_id,
        "planCode": command.plan_code,
        "planPeriod": command.plan_period.map(TokenBankPlanPeriod::as_str),
        "clientRequestNo": command.client_request_no,
        "coupon": coupon,
        "couponCode": coupon_code,
        "outTradeNo": command.out_trade_no,
    })
    .to_string()
}

fn account_value_sku_id(
    command: &CreateAccountRechargeOrderCommand,
    coupon_code: Option<&str>,
) -> String {
    command
        .package_id
        .clone()
        .or_else(|| command.plan_code.clone())
        .or_else(|| coupon_code.map(str::to_owned))
        .unwrap_or_else(|| format!("account-value-{}", command.target_asset.as_str()))
}

fn account_value_order_title(subject: AccountValueOrderSubject) -> &'static str {
    match subject {
        AccountValueOrderSubject::TokenBankRecharge => "Token Bank recharge",
        AccountValueOrderSubject::TokenBankPlanPurchase => "Token Bank plan purchase",
        AccountValueOrderSubject::TokenBankPlanRenewal => "Token Bank plan renewal",
        AccountValueOrderSubject::AccountRechargePackage => "Account recharge package",
        AccountValueOrderSubject::CouponRecharge => "Coupon recharge",
        _ => "Account value order",
    }
}

fn account_value_order_outcome_from_command(
    command: &CreateAccountRechargeOrderCommand,
) -> CreateAccountRechargeOrderOutcome {
    CreateAccountRechargeOrderOutcome {
        success: true,
        order_id: command.order_id.clone(),
        order_no: command.order_no.clone(),
        out_trade_no: command.out_trade_no.clone(),
        subject: command.subject,
        target_asset: command.target_asset,
        amount: command.amount.clone(),
        grant_amount: command.grant_amount.clone(),
        currency_code: command.currency_code.clone(),
        provider_code: String::new(),
        payment_method: String::new(),
        payment_product: String::new(),
        status: "pending".to_string(),
        next_action: "pay".to_string(),
        cashier_url: String::new(),
        qr_code_payload: String::new(),
        request_payment_payload: None,
    }
}

fn coupon_order_outcome_from_command(
    command: &CreateCouponRechargeOrderCommand,
) -> CreateAccountRechargeOrderOutcome {
    CreateAccountRechargeOrderOutcome {
        success: true,
        order_id: command.order_id.clone(),
        order_no: command.order_no.clone(),
        out_trade_no: command.out_trade_no.clone(),
        subject: command.subject,
        target_asset: command.target_asset,
        amount: command.amount.clone(),
        grant_amount: command.grant_amount.clone(),
        currency_code: command.currency_code.clone(),
        provider_code: String::new(),
        payment_method: String::new(),
        payment_product: String::new(),
        status: if command.payment_required {
            "pending".to_string()
        } else {
            "pending_fulfillment".to_string()
        },
        next_action: if command.payment_required {
            "pay".to_string()
        } else {
            "fulfill".to_string()
        },
        cashier_url: String::new(),
        qr_code_payload: String::new(),
        request_payment_payload: None,
    }
}

fn map_account_value_order_outcome(
    row: &sqlx::postgres::PgRow,
) -> Result<CreateAccountRechargeOrderOutcome, CommerceServiceError> {
    let subject = AccountValueOrderSubject::parse(&string_cell(row, "subject"))?;
    let target_asset = AccountValueAssetCode::parse(&string_cell(row, "target_asset"))?;
    Ok(CreateAccountRechargeOrderOutcome {
        success: true,
        order_id: string_cell(row, "order_id"),
        order_no: string_cell(row, "order_no"),
        out_trade_no: string_cell(row, "out_trade_no"),
        subject,
        target_asset,
        amount: commerce_money_cell(row, "amount", "account value order amount")?,
        grant_amount: commerce_money_cell(row, "grant_amount", "account value grant amount")?,
        currency_code: string_cell(row, "currency_code").to_ascii_uppercase(),
        provider_code: String::new(),
        payment_method: String::new(),
        payment_product: String::new(),
        status: string_cell(row, "status"),
        next_action: "pay".to_string(),
        cashier_url: String::new(),
        qr_code_payload: String::new(),
        request_payment_payload: None,
    })
}

fn map_account_value_package(
    row: &sqlx::postgres::PgRow,
) -> Result<AccountValuePackageItem, CommerceServiceError> {
    AccountValuePackageItem::new(
        &string_cell(row, "id"),
        &string_cell(row, "package_code"),
        &string_cell(row, "display_name"),
        AccountValueAssetCode::parse(&string_cell(row, "target_asset"))?,
        commerce_money_cell(row, "grant_amount", "account value package grant amount")?,
        commerce_money_cell(row, "bonus_amount", "account value package bonus amount")?,
        commerce_money_cell(row, "price_amount", "account value package price amount")?,
        &string_cell(row, "currency_code"),
        &string_cell(row, "status"),
    )
}

fn map_token_bank_plan(
    row: &sqlx::postgres::PgRow,
) -> Result<TokenBankPlanItem, CommerceServiceError> {
    TokenBankPlanItem::new(
        &string_cell(row, "plan_code"),
        &string_cell(row, "display_name"),
        TokenBankPlanPeriod::parse(&string_cell(row, "plan_period"))?,
        commerce_money_cell(row, "grant_amount", "Token Bank plan grant amount")?,
        commerce_money_cell(row, "bonus_amount", "Token Bank plan bonus amount")?,
        commerce_money_cell(row, "price_amount", "Token Bank plan price amount")?,
        &string_cell(row, "currency_code"),
        &string_cell(row, "renewal_policy"),
        &string_cell(row, "status"),
    )
}

fn map_refund_request(
    row: &sqlx::postgres::PgRow,
) -> Result<AccountValueRequestView, CommerceServiceError> {
    let mut view = AccountValueRequestView::new(
        &string_cell(row, "id"),
        &string_cell(row, "request_no"),
        optional_string_cell(row, "original_order_id").as_deref(),
        &string_cell(row, "owner_user_id"),
        AccountValueOrderSubject::RefundRequest,
        AccountValueAssetCode::parse(&string_cell(row, "target_asset"))?,
        commerce_money_cell(row, "amount", "refund request amount")?,
        &string_cell(row, "currency_code"),
        &string_cell(row, "status"),
        optional_string_cell(row, "provider_reference_id").as_deref(),
        &string_cell(row, "created_at"),
        &string_cell(row, "updated_at"),
    )?;
    if let (Some(amount), Some(currency_code)) = (
        optional_money_cell(row, "provider_amount", "refund provider amount")?,
        optional_string_cell(row, "provider_currency_code"),
    ) {
        view = view.with_provider_execution_amount(amount, &currency_code)?;
    }
    Ok(view.with_account_effect_reference_id(
        optional_string_cell(row, "account_effect_reference_id").as_deref(),
    ))
}

fn map_withdrawal_request(
    row: &sqlx::postgres::PgRow,
) -> Result<AccountValueRequestView, CommerceServiceError> {
    let mut view = AccountValueRequestView::new(
        &string_cell(row, "id"),
        &string_cell(row, "request_no"),
        None,
        &string_cell(row, "owner_user_id"),
        AccountValueOrderSubject::CashWithdrawal,
        AccountValueAssetCode::parse(&string_cell(row, "target_asset"))?,
        commerce_money_cell(row, "amount", "withdrawal request amount")?,
        &string_cell(row, "currency_code"),
        &string_cell(row, "status"),
        optional_string_cell(row, "provider_reference_id").as_deref(),
        &string_cell(row, "created_at"),
        &string_cell(row, "updated_at"),
    )?;
    if let (Some(amount), Some(currency_code)) = (
        optional_money_cell(row, "provider_amount", "withdrawal provider amount")?,
        optional_string_cell(row, "provider_currency_code"),
    ) {
        view = view.with_provider_execution_amount(amount, &currency_code)?;
    }
    Ok(view.with_account_effect_reference_id(
        optional_string_cell(row, "account_effect_reference_id").as_deref(),
    ))
}

fn commerce_money_cell(
    row: &sqlx::postgres::PgRow,
    column: &str,
    field_name: &str,
) -> Result<CommerceMoney, CommerceServiceError> {
    let value = string_cell(row, column);
    let normalized = normalize_money_minor_units(&value)
        .map_err(|_| CommerceServiceError::storage(format!("invalid {field_name}: {value}")))?;
    CommerceMoney::new(&normalized)
        .map_err(|message| CommerceServiceError::storage(format!("{message}: {value}")))
}

fn optional_money_cell(
    row: &sqlx::postgres::PgRow,
    column: &str,
    field_name: &str,
) -> Result<Option<CommerceMoney>, CommerceServiceError> {
    let Some(value) = optional_string_cell(row, column) else {
        return Ok(None);
    };
    let normalized = normalize_money_minor_units(&value)
        .map_err(|_| CommerceServiceError::storage(format!("invalid {field_name}: {value}")))?;
    CommerceMoney::new(&normalized)
        .map(Some)
        .map_err(|message| CommerceServiceError::storage(format!("{message}: {value}")))
}

fn normalize_money_minor_units(amount: &str) -> Result<String, CommerceServiceError> {
    let value = amount.trim();
    if value.contains('.') {
        return money_cents(value).map(|cents| cents.to_string());
    }
    let parsed = value.parse::<i64>().map_err(|_| {
        CommerceServiceError::storage(format!("invalid commerce money amount: {value}"))
    })?;
    if parsed < 0 {
        return Err(CommerceServiceError::storage(format!(
            "invalid commerce money amount: {value}"
        )));
    }
    Ok(parsed.to_string())
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

fn total_count_cell(row: &sqlx::postgres::PgRow) -> i64 {
    row.try_get::<i64, _>("total_count")
        .ok()
        .or_else(|| row.try_get::<i32, _>("total_count").ok().map(i64::from))
        .or_else(|| {
            row.try_get::<String, _>("total_count")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
        })
        .unwrap_or(0)
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column)
        .ok()
        .flatten()
        .filter(|value| !value.trim().is_empty())
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}

fn current_command_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    seconds.to_string()
}

/// RFC3339 timestamp for columns typed `timestamptz` (e.g.
/// `commerce_order_amount_breakdown.created_at`); `current_command_timestamp`
/// returns Unix seconds which PostgreSQL cannot cast to timestamptz.
fn rfc3339_command_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// RFC3339 timestamp shifted by the shared payment expiry window
/// (`SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS`, default 30 minutes).
fn rfc3339_after_payment_window(now: &str) -> String {
    let seconds = sdkwork_order_service::payment_expire_seconds();
    let base = chrono::DateTime::parse_from_rfc3339(now)
        .map(|value| value.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    (base + chrono::Duration::seconds(seconds)).to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Platform orders persist the sentinel organization scope (`"0"`) so that
/// list/status queries that normalize an empty organization to the sentinel
/// stay consistent with the stored row (same convention as recharge.rs).
fn normalize_organization_scope(organization_id: Option<&str>) -> String {
    organization_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(PLATFORM_ORGANIZATION_SCOPE_SENTINEL)
        .to_owned()
}

const PLATFORM_ORGANIZATION_SCOPE_SENTINEL: &str = "0";

fn store_error(context: &str, error: sqlx::Error) -> CommerceServiceError {
    crate::sql_store_error::map_sqlx_store_error(context, error)
}
