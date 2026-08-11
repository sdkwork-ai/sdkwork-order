use sdkwork_contract_service::{CommerceLedgerBusinessType, CommerceMoney, CommerceServiceError};

use crate::{
    physical_inventory_release_idempotency_key, refund_account_hold_idempotency_key,
    refund_payment_execution_idempotency_key, withdrawal_account_hold_idempotency_key,
    withdrawal_payment_execution_idempotency_key, AccountValueAssetCode, AccountValueLedgerCommand,
    AccountValueLedgerOutcome, AccountValueLedgerPort, AccountValueOrderSubject,
    AccountValueRequestExecutionStore, AccountValueRequestReviewAction,
    AccountValueRequestStatusCommand, AccountValueRequestView, PaymentExecutorOutcome,
    PaymentPayoutExecutionRequest, PaymentPayoutExecutorPort, PaymentRefundExecutionRequest,
    PaymentRefundExecutorPort, PhysicalInventoryReservationPort,
    ReleasePhysicalOrderInventoryRequest, ReviewAccountValueRequestCommand,
};

pub async fn execute_account_value_request_review<S, L, R, P, I>(
    store: &S,
    ledger_port: &L,
    refund_executor: &R,
    payout_executor: &P,
    inventory: &I,
    command: ReviewAccountValueRequestCommand,
) -> Result<AccountValueRequestView, CommerceServiceError>
where
    S: AccountValueRequestExecutionStore + ?Sized,
    L: AccountValueLedgerPort + ?Sized,
    R: PaymentRefundExecutorPort + ?Sized,
    P: PaymentPayoutExecutorPort + ?Sized,
    I: PhysicalInventoryReservationPort + ?Sized,
{
    let Some(request) = store
        .load_account_value_request_for_execution(&command)
        .await?
    else {
        return Err(CommerceServiceError::not_found(
            "account value request was not found",
        ));
    };

    if command.action == AccountValueRequestReviewAction::Reject {
        if matches!(request.status.as_str(), "refunded" | "paid_out") {
            return Err(CommerceServiceError::invalid_state(
                "a completed account value request cannot be rejected",
            ));
        }
        if let Some(hold_id) = request.account_effect_reference_id.as_deref() {
            release_account_hold(ledger_port, &command, &request, hold_id).await?;
        }
        return mark_request_status(store, &command, "rejected", None, None).await;
    }

    match command.subject {
        AccountValueOrderSubject::RefundRequest => {
            execute_refund_request(
                store,
                ledger_port,
                refund_executor,
                inventory,
                command,
                request,
            )
            .await
        }
        AccountValueOrderSubject::CashWithdrawal => {
            execute_withdrawal_request(store, ledger_port, payout_executor, command, request).await
        }
        _ => Err(CommerceServiceError::validation(
            "unsupported account value request subject",
        )),
    }
}

async fn execute_refund_request<S, L, R, I>(
    store: &S,
    ledger_port: &L,
    refund_executor: &R,
    inventory: &I,
    command: ReviewAccountValueRequestCommand,
    request: AccountValueRequestView,
) -> Result<AccountValueRequestView, CommerceServiceError>
where
    S: AccountValueRequestExecutionStore + ?Sized,
    L: AccountValueLedgerPort + ?Sized,
    R: PaymentRefundExecutorPort + ?Sized,
    I: PhysicalInventoryReservationPort + ?Sized,
{
    if request.status == "refunded" {
        return Ok(request);
    }
    let original_order_id = request.original_order_id.as_deref().ok_or_else(|| {
        CommerceServiceError::validation("refund request requires original_order_id")
    })?;
    let hold_id = ensure_account_hold(
        store,
        ledger_port,
        &command,
        &request,
        "account_reversal_held",
        &refund_account_hold_idempotency_key(&request.request_id),
    )
    .await?;

    mark_request_status(
        store,
        &command,
        "provider_refund_processing",
        None,
        Some(&hold_id),
    )
    .await?;

    let provider_outcome = match refund_executor
        .execute_provider_refund(PaymentRefundExecutionRequest {
            tenant_id: command.tenant_id.clone(),
            organization_id: command.organization_id.clone(),
            owner_user_id: request.owner_user_id.clone(),
            refund_request_id: request.request_id.clone(),
            original_order_id: original_order_id.to_owned(),
            amount: request.provider_execution_amount(),
            currency_code: request.provider_execution_currency_code().to_owned(),
            request_no: command.request_no.clone(),
            idempotency_key: refund_payment_execution_idempotency_key(&request.request_id),
        })
        .await
    {
        Ok(outcome) => outcome,
        Err(error) if provider_execution_error_is_deterministic(&error) => {
            release_account_hold(ledger_port, &command, &request, &hold_id).await?;
            mark_request_status(
                store,
                &command,
                "provider_refund_failed",
                None,
                Some(&hold_id),
            )
            .await?;
            return Err(error);
        }
        Err(error) => return Err(error),
    };

    match provider_refund_outcome_class(&provider_outcome.status) {
        ProviderRefundOutcomeClass::Succeeded => {
            settle_account_hold(ledger_port, &command, &request, &hold_id).await?;
            let view = mark_final_provider_status(
                store,
                &command,
                &provider_outcome,
                "refunded",
                &hold_id,
            )
            .await?;
            // Keep the owner order in sync with the real refund lifecycle so
            // order views expose `refund_status = refunded`.
            if let Err(error) = store
                .mark_owner_order_refunded(
                    &command.tenant_id,
                    command.organization_id.as_deref(),
                    &request.owner_user_id,
                    original_order_id,
                    "refunded",
                )
                .await
            {
                tracing::warn!(
                    target = "order.refund",
                    order_id = original_order_id,
                    error = ?error,
                    "failed to sync refunded status on owner order"
                );
            }
            // Physical orders refunded before shipment still hold an
            // inventory reservation; release it once the money is back.
            // Idempotent release semantics make this safe for virtual and
            // already-shipped orders (no reservation / consumed rows are
            // skipped), so no order-type lookup is required.
            if let Err(error) = inventory
                .release_physical_order_inventory(ReleasePhysicalOrderInventoryRequest {
                    tenant_id: command.tenant_id.clone(),
                    order_id: original_order_id.to_owned(),
                    reason_code: "refund".to_owned(),
                    request_no: format!("refund-{}", request.request_id),
                    idempotency_key: physical_inventory_release_idempotency_key(original_order_id),
                })
                .await
            {
                tracing::warn!(
                    target = "order.refund",
                    order_id = original_order_id,
                    error = ?error,
                    "failed to release physical inventory after refund"
                );
            }
            Ok(view)
        }
        ProviderRefundOutcomeClass::Processing => {
            mark_final_provider_status(
                store,
                &command,
                &provider_outcome,
                "provider_refund_processing",
                &hold_id,
            )
            .await
        }
        ProviderRefundOutcomeClass::Failed => {
            release_account_hold(ledger_port, &command, &request, &hold_id).await?;
            mark_final_provider_status(
                store,
                &command,
                &provider_outcome,
                "provider_refund_failed",
                &hold_id,
            )
            .await
        }
        ProviderRefundOutcomeClass::Unknown => Err(CommerceServiceError::conflict(
            "payment refund executor returned an unsupported status",
        )),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProviderRefundOutcomeClass {
    Processing,
    Succeeded,
    Failed,
    Unknown,
}

fn provider_execution_error_is_deterministic(error: &CommerceServiceError) -> bool {
    matches!(
        error.code(),
        "unauthenticated"
            | "unauthorized"
            | "not-found"
            | "conflict"
            | "invalid-state"
            | "validation"
            | "unsupported-capability"
    )
}

fn provider_refund_outcome_class(status: &str) -> ProviderRefundOutcomeClass {
    match status.trim().to_ascii_lowercase().as_str() {
        "submitted" | "pending" | "processing" => ProviderRefundOutcomeClass::Processing,
        "succeeded" | "success" | "refunded" => ProviderRefundOutcomeClass::Succeeded,
        "failed" | "canceled" | "cancelled" => ProviderRefundOutcomeClass::Failed,
        _ => ProviderRefundOutcomeClass::Unknown,
    }
}

async fn execute_withdrawal_request<S, L, P>(
    store: &S,
    ledger_port: &L,
    payout_executor: &P,
    command: ReviewAccountValueRequestCommand,
    request: AccountValueRequestView,
) -> Result<AccountValueRequestView, CommerceServiceError>
where
    S: AccountValueRequestExecutionStore + ?Sized,
    L: AccountValueLedgerPort + ?Sized,
    P: PaymentPayoutExecutorPort + ?Sized,
{
    if request.status == "paid_out" {
        return Ok(request);
    }
    let hold_id = ensure_account_hold(
        store,
        ledger_port,
        &command,
        &request,
        "account_cash_held",
        &withdrawal_account_hold_idempotency_key(&request.request_id),
    )
    .await?;

    mark_request_status(
        store,
        &command,
        "provider_payout_processing",
        None,
        Some(&hold_id),
    )
    .await?;

    let provider_outcome = match payout_executor
        .execute_provider_payout(PaymentPayoutExecutionRequest {
            tenant_id: command.tenant_id.clone(),
            organization_id: command.organization_id.clone(),
            withdrawal_request_id: request.request_id.clone(),
            amount: request.provider_execution_amount(),
            currency_code: request.provider_execution_currency_code().to_owned(),
            request_no: command.request_no.clone(),
            idempotency_key: withdrawal_payment_execution_idempotency_key(&request.request_id),
        })
        .await
    {
        Ok(outcome) => outcome,
        Err(error) if provider_execution_error_is_deterministic(&error) => {
            release_account_hold(ledger_port, &command, &request, &hold_id).await?;
            mark_request_status(
                store,
                &command,
                "provider_payout_failed",
                None,
                Some(&hold_id),
            )
            .await?;
            return Err(error);
        }
        Err(error) => return Err(error),
    };

    match provider_refund_outcome_class(&provider_outcome.status) {
        ProviderRefundOutcomeClass::Succeeded => {
            settle_account_hold(ledger_port, &command, &request, &hold_id).await?;
            mark_final_provider_status(store, &command, &provider_outcome, "paid_out", &hold_id)
                .await
        }
        ProviderRefundOutcomeClass::Processing => {
            mark_final_provider_status(
                store,
                &command,
                &provider_outcome,
                "provider_payout_processing",
                &hold_id,
            )
            .await
        }
        ProviderRefundOutcomeClass::Failed => {
            release_account_hold(ledger_port, &command, &request, &hold_id).await?;
            mark_final_provider_status(
                store,
                &command,
                &provider_outcome,
                "provider_payout_failed",
                &hold_id,
            )
            .await
        }
        ProviderRefundOutcomeClass::Unknown => Err(CommerceServiceError::conflict(
            "payment payout executor returned an unsupported status",
        )),
    }
}

async fn ensure_account_hold<S, L>(
    store: &S,
    ledger_port: &L,
    command: &ReviewAccountValueRequestCommand,
    request: &AccountValueRequestView,
    held_status: &str,
    idempotency_key: &str,
) -> Result<String, CommerceServiceError>
where
    S: AccountValueRequestExecutionStore + ?Sized,
    L: AccountValueLedgerPort + ?Sized,
{
    let retry_after_released_hold = matches!(
        request.status.as_str(),
        "provider_refund_failed" | "provider_payout_failed"
    );
    if !retry_after_released_hold {
        if let Some(reference_id) = request.account_effect_reference_id.as_deref() {
            return Ok(reference_id.to_owned());
        }
    }
    let hold_idempotency_key = if retry_after_released_hold {
        format!("{idempotency_key}:retry:{}", command.idempotency_key)
    } else {
        idempotency_key.to_owned()
    };
    let outcome = ledger_port
        .apply_account_value_ledger_command(AccountValueLedgerCommand::hold(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &request.owner_user_id,
            request.target_asset,
            request.amount.clone(),
            &request.currency_code,
            hold_business_type(command.subject, request.target_asset),
            &request.request_id,
            &command.request_no,
            &hold_idempotency_key,
        )?)
        .await?;
    let hold_id = account_effect_reference(&outcome, "account hold")?;
    mark_request_status(store, command, held_status, None, Some(&hold_id)).await?;
    Ok(hold_id)
}

async fn settle_account_hold<L>(
    ledger_port: &L,
    command: &ReviewAccountValueRequestCommand,
    request: &AccountValueRequestView,
    hold_id: &str,
) -> Result<AccountValueLedgerOutcome, CommerceServiceError>
where
    L: AccountValueLedgerPort + ?Sized,
{
    ledger_port
        .apply_account_value_ledger_command(AccountValueLedgerCommand::hold_settle(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &request.owner_user_id,
            request.target_asset,
            request.amount.clone(),
            &request.currency_code,
            settle_business_type(command.subject, request.target_asset),
            hold_id,
            &format!("{}:settle", command.request_no),
            &format!(
                "{}:{hold_id}:settle",
                account_hold_idempotency_key(command.subject, &request.request_id)?
            ),
        )?)
        .await
}

async fn release_account_hold<L>(
    ledger_port: &L,
    command: &ReviewAccountValueRequestCommand,
    request: &AccountValueRequestView,
    hold_id: &str,
) -> Result<AccountValueLedgerOutcome, CommerceServiceError>
where
    L: AccountValueLedgerPort + ?Sized,
{
    ledger_port
        .apply_account_value_ledger_command(AccountValueLedgerCommand::hold_release(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &request.owner_user_id,
            request.target_asset,
            request.amount.clone(),
            &request.currency_code,
            release_business_type(request.target_asset),
            hold_id,
            &format!("{}:release", command.request_no),
            &format!(
                "{}:{hold_id}:release",
                account_hold_idempotency_key(command.subject, &request.request_id)?
            ),
        )?)
        .await
}

async fn mark_final_provider_status<S>(
    store: &S,
    command: &ReviewAccountValueRequestCommand,
    provider_outcome: &PaymentExecutorOutcome,
    status: &str,
    hold_id: &str,
) -> Result<AccountValueRequestView, CommerceServiceError>
where
    S: AccountValueRequestExecutionStore + ?Sized,
{
    mark_request_status(
        store,
        command,
        status,
        provider_outcome.provider_reference_id.as_deref(),
        Some(hold_id),
    )
    .await
}

async fn mark_request_status<S>(
    store: &S,
    command: &ReviewAccountValueRequestCommand,
    status: &str,
    provider_reference_id: Option<&str>,
    account_effect_reference_id: Option<&str>,
) -> Result<AccountValueRequestView, CommerceServiceError>
where
    S: AccountValueRequestExecutionStore + ?Sized,
{
    store
        .mark_account_value_request_status(AccountValueRequestStatusCommand {
            tenant_id: command.tenant_id.clone(),
            organization_id: command.organization_id.clone(),
            subject: command.subject,
            request_id: command.request_id.clone(),
            action: command.action,
            status: status.to_owned(),
            reason_code: command.reason_code.clone(),
            review_comment: command.review_comment.clone(),
            provider_reference_id: provider_reference_id.map(str::to_owned),
            account_effect_reference_id: account_effect_reference_id.map(str::to_owned),
            request_no: command.request_no.clone(),
            idempotency_key: format!("{}:{status}", command.idempotency_key),
        })
        .await
}

fn account_effect_reference(
    outcome: &AccountValueLedgerOutcome,
    effect_name: &str,
) -> Result<String, CommerceServiceError> {
    outcome
        .account_effect_reference_id
        .clone()
        .or_else(|| outcome.ledger_entry_id.clone())
        .ok_or_else(|| {
            CommerceServiceError::invalid_state(format!(
                "{effect_name} command did not return account effect reference id"
            ))
        })
}

fn hold_business_type(
    subject: AccountValueOrderSubject,
    asset: AccountValueAssetCode,
) -> &'static str {
    match (subject, asset) {
        (AccountValueOrderSubject::RefundRequest, AccountValueAssetCode::TokenBank) => {
            CommerceLedgerBusinessType::TOKEN_BANK_HOLD
        }
        (_, AccountValueAssetCode::TokenBank) => CommerceLedgerBusinessType::TOKEN_BANK_HOLD,
        (_, AccountValueAssetCode::Points) => CommerceLedgerBusinessType::POINTS_CLAWBACK,
        (_, AccountValueAssetCode::Subscription) => CommerceLedgerBusinessType::MANUAL_ADJUSTMENT,
        (_, AccountValueAssetCode::Cash) => CommerceLedgerBusinessType::CASH_ADJUSTMENT,
    }
}

fn settle_business_type(
    subject: AccountValueOrderSubject,
    asset: AccountValueAssetCode,
) -> &'static str {
    match subject {
        AccountValueOrderSubject::RefundRequest => subject.compensation_business_type(asset),
        AccountValueOrderSubject::CashWithdrawal => CommerceLedgerBusinessType::CASH_ADJUSTMENT,
        _ => CommerceLedgerBusinessType::MANUAL_ADJUSTMENT,
    }
}

fn release_business_type(asset: AccountValueAssetCode) -> &'static str {
    match asset {
        AccountValueAssetCode::TokenBank => CommerceLedgerBusinessType::TOKEN_BANK_HOLD_RELEASE,
        AccountValueAssetCode::Points => CommerceLedgerBusinessType::POINTS_CLAWBACK,
        AccountValueAssetCode::Subscription => CommerceLedgerBusinessType::MANUAL_ADJUSTMENT,
        AccountValueAssetCode::Cash => CommerceLedgerBusinessType::CASH_ADJUSTMENT,
    }
}

fn account_hold_idempotency_key(
    subject: AccountValueOrderSubject,
    request_id: &str,
) -> Result<String, CommerceServiceError> {
    match subject {
        AccountValueOrderSubject::RefundRequest => {
            Ok(refund_account_hold_idempotency_key(request_id))
        }
        AccountValueOrderSubject::CashWithdrawal => {
            Ok(withdrawal_account_hold_idempotency_key(request_id))
        }
        _ => Err(CommerceServiceError::validation(
            "unsupported account value request subject",
        )),
    }
}

#[allow(dead_code)]
fn require_positive_amount(
    amount: &CommerceMoney,
    field_name: &str,
) -> Result<(), CommerceServiceError> {
    if amount.as_str() == "0" {
        return Err(CommerceServiceError::validation(format!(
            "{field_name} must be greater than zero"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod refund_outcome_tests {
    use super::{provider_refund_outcome_class, ProviderRefundOutcomeClass};

    #[test]
    fn provider_refund_processing_does_not_mean_funds_were_refunded() {
        assert_eq!(
            provider_refund_outcome_class("processing"),
            ProviderRefundOutcomeClass::Processing
        );
        assert_eq!(
            provider_refund_outcome_class("succeeded"),
            ProviderRefundOutcomeClass::Succeeded
        );
        assert_eq!(
            provider_refund_outcome_class("unexpected"),
            ProviderRefundOutcomeClass::Unknown
        );
    }
}
