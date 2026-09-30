use sdkwork_account_repository_sqlx::{hold_request_hash, PostgresCommerceAccountStore};
use sdkwork_account_service::{
    AppendLedgerEntryCommand, CreateAccountHoldCommand, HoldMutationOutcome,
    ReleaseAccountHoldCommand, SettleAccountHoldCommand,
};
use sdkwork_contract_service::{
    CommerceAccountAssetType, CommerceLedgerDirection, CommerceMoney, CommerceRequestHash,
    CommerceServiceError,
};
use sdkwork_order_service::{
    AccountPointsCreditFuture, AccountPointsCreditPort, AccountValueAssetCode, AccountValueFuture,
    AccountValueLedgerCommand, AccountValueLedgerOperation, AccountValueLedgerOutcome,
    AccountValueLedgerPort, PointsRechargeCreditOutcome, PointsRechargeCreditRequest,
    POINTS_RECHARGE_LEDGER_BUSINESS_TYPE,
};
use sdkwork_utils_rust::sha256_hash;
use sqlx::PgPool;

enum StoreKind {
    Postgres(PostgresCommerceAccountStore),
}

pub struct StoreAccountPointsCreditAdapter {
    store: StoreKind,
}

impl StoreAccountPointsCreditAdapter {
    pub fn postgres(pool: PgPool) -> Self {
        Self {
            store: StoreKind::Postgres(PostgresCommerceAccountStore::new(pool)),
        }
    }
}

impl AccountPointsCreditPort for StoreAccountPointsCreditAdapter {
    fn credit_points_recharge<'a>(
        &'a self,
        request: PointsRechargeCreditRequest,
    ) -> AccountPointsCreditFuture<'a, PointsRechargeCreditOutcome> {
        Box::pin(async move {
            self.append_points_adjustment(request, CommerceLedgerDirection::Credit)
                .await
        })
    }

    fn reverse_points_recharge_credit<'a>(
        &'a self,
        request: PointsRechargeCreditRequest,
    ) -> AccountPointsCreditFuture<'a, PointsRechargeCreditOutcome> {
        Box::pin(async move {
            self.append_points_adjustment(request, CommerceLedgerDirection::Debit)
                .await
        })
    }
}

impl AccountValueLedgerPort for StoreAccountPointsCreditAdapter {
    fn apply_account_value_ledger_command<'a>(
        &'a self,
        command: AccountValueLedgerCommand,
    ) -> AccountValueFuture<'a, AccountValueLedgerOutcome> {
        Box::pin(async move {
            match command.operation {
                AccountValueLedgerOperation::Credit
                | AccountValueLedgerOperation::Debit
                | AccountValueLedgerOperation::Reversal => {
                    self.append_account_value_adjustment(command).await
                }
                AccountValueLedgerOperation::Hold => self.create_account_value_hold(command).await,
                AccountValueLedgerOperation::HoldSettle => {
                    self.settle_account_value_hold(command).await
                }
                AccountValueLedgerOperation::HoldRelease => {
                    self.release_account_value_hold(command).await
                }
            }
        })
    }
}

impl StoreAccountPointsCreditAdapter {
    async fn append_points_adjustment(
        &self,
        request: PointsRechargeCreditRequest,
        direction: CommerceLedgerDirection,
    ) -> Result<PointsRechargeCreditOutcome, CommerceServiceError> {
        let points_text = request.points.to_string();
        let amount = CommerceMoney::new(&points_text)
            .map_err(|error| CommerceServiceError::validation(error.to_string()))?;
        let direction_text = match direction {
            CommerceLedgerDirection::Credit => "credit",
            CommerceLedgerDirection::Debit => "debit",
        };
        let request_hash = ledger_request_hash(&request, direction_text)?;
        let command = AppendLedgerEntryCommand::new(
            &request.tenant_id,
            request.organization_id.as_deref(),
            "",
            &request.owner_user_id,
            CommerceAccountAssetType::Points,
            Some("POINT"),
            direction,
            amount,
            POINTS_RECHARGE_LEDGER_BUSINESS_TYPE,
            &request.transaction_no,
            &request.request_no,
            &request.idempotency_key,
        )?;
        let outcome = match &self.store {
            StoreKind::Postgres(store) => store.append_ledger_entry(command, request_hash).await?,
        };
        Ok(PointsRechargeCreditOutcome {
            accepted: true,
            replayed: outcome.replayed,
        })
    }

    async fn append_account_value_adjustment(
        &self,
        command: AccountValueLedgerCommand,
    ) -> Result<AccountValueLedgerOutcome, CommerceServiceError> {
        let transaction_no = account_value_transaction_no(&command);
        let request_hash = account_value_request_hash(&command, &transaction_no)?;
        let ledger_command = AppendLedgerEntryCommand::new(
            &command.tenant_id,
            command.organization_id.as_deref(),
            "",
            &command.owner_user_id,
            asset_type_for_account_value(command.asset),
            Some(&command.currency_code),
            command.direction,
            command.amount,
            &command.business_type,
            &transaction_no,
            &command.request_no,
            &command.idempotency_key,
        )?;
        let outcome = match &self.store {
            StoreKind::Postgres(store) => {
                store
                    .append_ledger_entry(ledger_command, request_hash)
                    .await?
            }
        };
        Ok(AccountValueLedgerOutcome {
            accepted: true,
            replayed: outcome.replayed,
            ledger_entry_id: Some(outcome.ledger_entry.id),
            account_effect_reference_id: None,
        })
    }

    async fn create_account_value_hold(
        &self,
        command: AccountValueLedgerCommand,
    ) -> Result<AccountValueLedgerOutcome, CommerceServiceError> {
        let request_hash = account_value_hold_request_hash(&command)?;
        let hold_command = CreateAccountHoldCommand::new(
            &command.tenant_id,
            command.organization_id.as_deref(),
            "",
            &command.owner_user_id,
            asset_type_for_account_value(command.asset),
            command.amount,
            &command.business_type,
            &command.resource_id,
            "commerce_order_request",
            &stable_numeric_source_id(&command.resource_id),
            &command.request_no,
            &command.idempotency_key,
            None,
        )?;
        let outcome = match &self.store {
            StoreKind::Postgres(store) => {
                store
                    .create_account_hold(hold_command, request_hash)
                    .await?
            }
        };
        hold_outcome_to_account_value_outcome(outcome)
    }

    async fn settle_account_value_hold(
        &self,
        command: AccountValueLedgerCommand,
    ) -> Result<AccountValueLedgerOutcome, CommerceServiceError> {
        let request_hash = account_value_hold_request_hash(&command)?;
        let transaction_no = account_value_transaction_no(&command);
        let hold_command = SettleAccountHoldCommand::new(
            &command.tenant_id,
            &command.resource_id,
            &command.business_type,
            &transaction_no,
            &command.request_no,
            &command.idempotency_key,
        )?;
        let outcome = match &self.store {
            StoreKind::Postgres(store) => {
                store
                    .settle_account_hold(hold_command, request_hash)
                    .await?
            }
        };
        hold_outcome_to_account_value_outcome(outcome)
    }

    async fn release_account_value_hold(
        &self,
        command: AccountValueLedgerCommand,
    ) -> Result<AccountValueLedgerOutcome, CommerceServiceError> {
        let request_hash = account_value_hold_request_hash(&command)?;
        let hold_command = ReleaseAccountHoldCommand::new(
            &command.tenant_id,
            &command.resource_id,
            &command.request_no,
            &command.idempotency_key,
        )?;
        let outcome = match &self.store {
            StoreKind::Postgres(store) => {
                store
                    .release_account_hold(hold_command, request_hash)
                    .await?
            }
        };
        hold_outcome_to_account_value_outcome(outcome)
    }
}

fn ledger_request_hash(
    request: &PointsRechargeCreditRequest,
    direction: &str,
) -> Result<CommerceRequestHash, CommerceServiceError> {
    let body = serde_json::json!({
        "tenantId": request.tenant_id,
        "organizationId": request.organization_id,
        "ownerUserId": request.owner_user_id,
        "assetType": "points",
        "currencyCode": "POINT",
        "direction": direction,
        "amount": request.points.to_string(),
        "businessType": POINTS_RECHARGE_LEDGER_BUSINESS_TYPE,
        "transactionNo": request.transaction_no,
        "requestNo": request.request_no,
        "idempotencyKey": request.idempotency_key,
    });
    let canonical = serde_json::to_string(&body).map_err(|error| {
        CommerceServiceError::validation(format!("request body is invalid: {error}"))
    })?;
    CommerceRequestHash::new(&sha256_hash(canonical.as_bytes()))
}

fn account_value_transaction_no(command: &AccountValueLedgerCommand) -> String {
    format!("{}:{}", command.business_type, command.resource_id)
}

fn asset_type_for_account_value(asset: AccountValueAssetCode) -> CommerceAccountAssetType {
    match asset {
        AccountValueAssetCode::Cash => CommerceAccountAssetType::Cash,
        AccountValueAssetCode::Points => CommerceAccountAssetType::Points,
        AccountValueAssetCode::TokenBank => CommerceAccountAssetType::TokenBank,
        AccountValueAssetCode::Subscription => {
            unreachable!("subscription coupons are fulfilled by Membership")
        }
    }
}

fn account_value_request_hash(
    command: &AccountValueLedgerCommand,
    transaction_no: &str,
) -> Result<CommerceRequestHash, CommerceServiceError> {
    let body = serde_json::json!({
        "tenantId": command.tenant_id,
        "organizationId": command.organization_id,
        "ownerUserId": command.owner_user_id,
        "assetType": command.asset.as_str(),
        "currencyCode": command.currency_code,
        "direction": command.direction.as_str(),
        "amount": command.amount.as_str(),
        "businessType": command.business_type,
        "transactionNo": transaction_no,
        "requestNo": command.request_no,
        "idempotencyKey": command.idempotency_key,
    });
    let canonical = serde_json::to_string(&body).map_err(|error| {
        CommerceServiceError::validation(format!("request body is invalid: {error}"))
    })?;
    CommerceRequestHash::new(&sha256_hash(canonical.as_bytes()))
}

fn account_value_hold_request_hash(
    command: &AccountValueLedgerCommand,
) -> Result<CommerceRequestHash, CommerceServiceError> {
    let body = serde_json::json!({
        "tenantId": command.tenant_id,
        "organizationId": command.organization_id,
        "ownerUserId": command.owner_user_id,
        "assetType": command.asset.as_str(),
        "currencyCode": command.currency_code,
        "operation": command.operation.as_str(),
        "amount": command.amount.as_str(),
        "businessType": command.business_type,
        "businessNo": command.resource_id,
        "sourceType": "commerce_order_request",
        "sourceId": stable_numeric_source_id(&command.resource_id),
        "requestNo": command.request_no,
        "idempotencyKey": command.idempotency_key,
    });
    let canonical = serde_json::to_string(&body).map_err(|error| {
        CommerceServiceError::validation(format!("request body is invalid: {error}"))
    })?;
    hold_request_hash(&canonical)
}

fn hold_outcome_to_account_value_outcome(
    outcome: HoldMutationOutcome,
) -> Result<AccountValueLedgerOutcome, CommerceServiceError> {
    Ok(AccountValueLedgerOutcome {
        accepted: true,
        replayed: outcome.replayed,
        ledger_entry_id: outcome.ledger_entry.map(|entry| entry.id),
        account_effect_reference_id: Some(outcome.hold.uuid),
    })
}

fn stable_numeric_source_id(value: &str) -> String {
    let digest = sha256_hash(value.trim().as_bytes());
    let prefix = digest.get(..15).unwrap_or(digest.as_str());
    u64::from_str_radix(prefix, 16).unwrap_or(0).to_string()
}
