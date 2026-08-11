use std::future::Future;
use std::pin::Pin;

use sdkwork_contract_service::{CommerceLedgerDirection, CommerceMoney, CommerceServiceError};

use crate::{
    AccountValueAssetCode, AccountValueFulfillmentContext, AccountValueOrderSubject,
    AccountValueRequestReviewAction, AccountValueRequestView, FulfillAccountValueOrderCommand,
    FulfillAccountValueOrderOutcome, ReviewAccountValueRequestCommand,
};

pub type AccountValueFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountValueLedgerOperation {
    Credit,
    Debit,
    Reversal,
    Hold,
    HoldSettle,
    HoldRelease,
}

impl AccountValueLedgerOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Credit => "credit",
            Self::Debit => "debit",
            Self::Reversal => "reversal",
            Self::Hold => "hold",
            Self::HoldSettle => "hold_settle",
            Self::HoldRelease => "hold_release",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountValueLedgerCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub asset: AccountValueAssetCode,
    pub amount: CommerceMoney,
    pub currency_code: String,
    pub operation: AccountValueLedgerOperation,
    pub direction: CommerceLedgerDirection,
    pub business_type: String,
    pub resource_id: String,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountValueLedgerOutcome {
    pub accepted: bool,
    pub replayed: bool,
    pub ledger_entry_id: Option<String>,
    pub account_effect_reference_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentRefundExecutionRequest {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub refund_request_id: String,
    pub original_order_id: String,
    pub amount: CommerceMoney,
    pub currency_code: String,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentPayoutExecutionRequest {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub withdrawal_request_id: String,
    pub amount: CommerceMoney,
    pub currency_code: String,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentExecutorOutcome {
    pub accepted: bool,
    pub replayed: bool,
    pub provider_reference_id: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountValueRequestStatusCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub subject: AccountValueOrderSubject,
    pub request_id: String,
    pub action: AccountValueRequestReviewAction,
    pub status: String,
    pub reason_code: Option<String>,
    pub review_comment: Option<String>,
    pub provider_reference_id: Option<String>,
    pub account_effect_reference_id: Option<String>,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CouponRedemptionRequest {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub coupon_code: String,
    pub order_id: String,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CouponSubscriptionPeriod {
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CouponRedemptionBenefit {
    TokenBankCredit {
        grant_amount: CommerceMoney,
    },
    PointsCredit {
        grant_amount: CommerceMoney,
    },
    CashCredit {
        grant_amount: CommerceMoney,
    },
    Subscription {
        product_id: String,
        sku_id: String,
        package_id: i64,
        period: CouponSubscriptionPeriod,
        duration_days: i64,
        daily_quota: i64,
        total_quota: i64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CouponRedemptionOutcome {
    pub accepted: bool,
    pub replayed: bool,
    pub benefit: CouponRedemptionBenefit,
}

impl CouponSubscriptionPeriod {
    pub fn parse(value: &str) -> Result<Self, CommerceServiceError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "day" => Ok(Self::Day),
            "week" => Ok(Self::Week),
            "month" => Ok(Self::Month),
            "quarter" => Ok(Self::Quarter),
            "year" => Ok(Self::Year),
            _ => Err(CommerceServiceError::validation(
                "coupon subscription period is invalid",
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Quarter => "quarter",
            Self::Year => "year",
        }
    }
}

impl CouponRedemptionBenefit {
    pub fn target_asset(&self) -> AccountValueAssetCode {
        match self {
            Self::TokenBankCredit { .. } => AccountValueAssetCode::TokenBank,
            Self::PointsCredit { .. } => AccountValueAssetCode::Points,
            Self::CashCredit { .. } => AccountValueAssetCode::Cash,
            Self::Subscription { .. } => AccountValueAssetCode::Subscription,
        }
    }

    pub fn grant_amount(&self) -> CommerceMoney {
        match self {
            Self::TokenBankCredit { grant_amount }
            | Self::PointsCredit { grant_amount }
            | Self::CashCredit { grant_amount } => grant_amount.clone(),
            Self::Subscription { total_quota, .. } => CommerceMoney::new(&total_quota.to_string())
                .expect("validated subscription quota must be valid commerce money"),
        }
    }
}

pub trait AccountValueLedgerPort: Send + Sync {
    fn apply_account_value_ledger_command<'a>(
        &'a self,
        command: AccountValueLedgerCommand,
    ) -> AccountValueFuture<'a, AccountValueLedgerOutcome>;
}

pub struct NoopAccountValueLedgerPort;

impl AccountValueLedgerPort for NoopAccountValueLedgerPort {
    fn apply_account_value_ledger_command<'a>(
        &'a self,
        _command: AccountValueLedgerCommand,
    ) -> AccountValueFuture<'a, AccountValueLedgerOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::unsupported_capability(
                "account value ledger port is not configured",
            ))
        })
    }
}

pub trait PaymentRefundExecutorPort: Send + Sync {
    fn execute_provider_refund<'a>(
        &'a self,
        request: PaymentRefundExecutionRequest,
    ) -> AccountValueFuture<'a, PaymentExecutorOutcome>;
}

pub trait PaymentPayoutExecutorPort: Send + Sync {
    fn execute_provider_payout<'a>(
        &'a self,
        request: PaymentPayoutExecutionRequest,
    ) -> AccountValueFuture<'a, PaymentExecutorOutcome>;
}

pub struct NoopPaymentRefundExecutorPort;

impl PaymentRefundExecutorPort for NoopPaymentRefundExecutorPort {
    fn execute_provider_refund<'a>(
        &'a self,
        _request: PaymentRefundExecutionRequest,
    ) -> AccountValueFuture<'a, PaymentExecutorOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::unsupported_capability(
                "payment refund executor port is not configured",
            ))
        })
    }
}

pub struct NoopPaymentPayoutExecutorPort;

impl PaymentPayoutExecutorPort for NoopPaymentPayoutExecutorPort {
    fn execute_provider_payout<'a>(
        &'a self,
        _request: PaymentPayoutExecutionRequest,
    ) -> AccountValueFuture<'a, PaymentExecutorOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::unsupported_capability(
                "provider payout executor port is not configured",
            ))
        })
    }
}

pub trait CouponRedemptionPort: Send + Sync {
    fn preview_coupon<'a>(
        &'a self,
        request: CouponRedemptionRequest,
    ) -> AccountValueFuture<'a, CouponRedemptionOutcome>;

    fn redeem_coupon<'a>(
        &'a self,
        request: CouponRedemptionRequest,
    ) -> AccountValueFuture<'a, CouponRedemptionOutcome>;
}

pub struct NoopCouponRedemptionPort;

impl CouponRedemptionPort for NoopCouponRedemptionPort {
    fn preview_coupon<'a>(
        &'a self,
        _request: CouponRedemptionRequest,
    ) -> AccountValueFuture<'a, CouponRedemptionOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::unsupported_capability(
                "coupon redemption port is not configured",
            ))
        })
    }

    fn redeem_coupon<'a>(
        &'a self,
        _request: CouponRedemptionRequest,
    ) -> AccountValueFuture<'a, CouponRedemptionOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::unsupported_capability(
                "coupon redemption port is not configured",
            ))
        })
    }
}

pub trait AccountValueRequestExecutionStore: Send + Sync {
    fn load_account_value_request_for_execution<'a>(
        &'a self,
        command: &'a ReviewAccountValueRequestCommand,
    ) -> AccountValueFuture<'a, Option<AccountValueRequestView>>;

    fn mark_account_value_request_status<'a>(
        &'a self,
        command: AccountValueRequestStatusCommand,
    ) -> AccountValueFuture<'a, AccountValueRequestView>;

    /// Syncs the owner order's `refund_status` after a refund settles
    /// successfully, so order views reflect the real refund lifecycle.
    fn mark_owner_order_refunded<'a>(
        &'a self,
        tenant_id: &'a str,
        organization_id: Option<&'a str>,
        owner_user_id: &'a str,
        order_id: &'a str,
        refund_status: &'a str,
    ) -> AccountValueFuture<'a, ()>;
}

pub type AccountValueFulfillmentFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait AccountValueFulfillmentStore: Send + Sync {
    fn load_account_value_fulfillment_context<'a>(
        &'a self,
        command: &'a FulfillAccountValueOrderCommand,
    ) -> AccountValueFulfillmentFuture<'a, Option<AccountValueFulfillmentContext>>;

    fn reserve_account_value_fulfillment<'a>(
        &'a self,
        command: &'a FulfillAccountValueOrderCommand,
        context: &'a AccountValueFulfillmentContext,
    ) -> AccountValueFulfillmentFuture<'a, ()>;

    fn release_account_value_fulfillment_reservation<'a>(
        &'a self,
        command: &'a FulfillAccountValueOrderCommand,
        context: &'a AccountValueFulfillmentContext,
    ) -> AccountValueFulfillmentFuture<'a, ()>;

    fn commit_account_value_fulfillment<'a>(
        &'a self,
        command: FulfillAccountValueOrderCommand,
        context: &'a AccountValueFulfillmentContext,
    ) -> AccountValueFulfillmentFuture<'a, FulfillAccountValueOrderOutcome>;
}

impl AccountValueLedgerCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn credit(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        asset: AccountValueAssetCode,
        amount: CommerceMoney,
        currency_code: &str,
        business_type: &str,
        resource_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Self::new(
            tenant_id,
            organization_id,
            owner_user_id,
            asset,
            amount,
            currency_code,
            AccountValueLedgerOperation::Credit,
            CommerceLedgerDirection::Credit,
            business_type,
            resource_id,
            request_no,
            idempotency_key,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn debit(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        asset: AccountValueAssetCode,
        amount: CommerceMoney,
        currency_code: &str,
        business_type: &str,
        resource_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Self::new(
            tenant_id,
            organization_id,
            owner_user_id,
            asset,
            amount,
            currency_code,
            AccountValueLedgerOperation::Debit,
            CommerceLedgerDirection::Debit,
            business_type,
            resource_id,
            request_no,
            idempotency_key,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reversal(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        asset: AccountValueAssetCode,
        amount: CommerceMoney,
        currency_code: &str,
        business_type: &str,
        resource_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Self::new(
            tenant_id,
            organization_id,
            owner_user_id,
            asset,
            amount,
            currency_code,
            AccountValueLedgerOperation::Reversal,
            CommerceLedgerDirection::Debit,
            business_type,
            resource_id,
            request_no,
            idempotency_key,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn hold(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        asset: AccountValueAssetCode,
        amount: CommerceMoney,
        currency_code: &str,
        business_type: &str,
        resource_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Self::new(
            tenant_id,
            organization_id,
            owner_user_id,
            asset,
            amount,
            currency_code,
            AccountValueLedgerOperation::Hold,
            CommerceLedgerDirection::Debit,
            business_type,
            resource_id,
            request_no,
            idempotency_key,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn hold_settle(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        asset: AccountValueAssetCode,
        amount: CommerceMoney,
        currency_code: &str,
        business_type: &str,
        hold_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Self::new(
            tenant_id,
            organization_id,
            owner_user_id,
            asset,
            amount,
            currency_code,
            AccountValueLedgerOperation::HoldSettle,
            CommerceLedgerDirection::Debit,
            business_type,
            hold_id,
            request_no,
            idempotency_key,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn hold_release(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        asset: AccountValueAssetCode,
        amount: CommerceMoney,
        currency_code: &str,
        business_type: &str,
        hold_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Self::new(
            tenant_id,
            organization_id,
            owner_user_id,
            asset,
            amount,
            currency_code,
            AccountValueLedgerOperation::HoldRelease,
            CommerceLedgerDirection::Credit,
            business_type,
            hold_id,
            request_no,
            idempotency_key,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        asset: AccountValueAssetCode,
        amount: CommerceMoney,
        currency_code: &str,
        operation: AccountValueLedgerOperation,
        direction: CommerceLedgerDirection,
        business_type: &str,
        resource_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("currency_code", currency_code)?;
        sdkwork_contract_service::CommerceLedgerBusinessType::validate(business_type)?;
        crate::validation::require_non_empty("resource_id", resource_id)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            asset,
            amount,
            currency_code: currency_code.trim().to_ascii_uppercase(),
            operation,
            direction,
            business_type: business_type.trim().to_string(),
            resource_id: resource_id.trim().to_string(),
            request_no: request_no.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
        })
    }
}

pub fn token_bank_recharge_fulfillment_idempotency_key(order_id: &str) -> String {
    format!("token-bank-recharge:fulfill:{order_id}")
}

pub fn token_bank_plan_purchase_idempotency_key(order_id: &str) -> String {
    format!("token-bank-plan:purchase:{order_id}")
}

pub fn token_bank_plan_renewal_idempotency_key(order_id: &str) -> String {
    format!("token-bank-plan:renewal:{order_id}")
}

pub fn account_package_fulfillment_idempotency_key(order_id: &str) -> String {
    format!("account-package:fulfill:{order_id}")
}

pub fn coupon_recharge_fulfillment_idempotency_key(order_id: &str) -> String {
    format!("coupon-recharge:fulfill:{order_id}")
}

pub fn coupon_recharge_redemption_idempotency_key(order_id: &str) -> String {
    format!("coupon-recharge:redeem:{order_id}")
}

pub fn refund_account_hold_idempotency_key(refund_request_id: &str) -> String {
    format!("refund-request:account-hold:{refund_request_id}")
}

pub fn refund_payment_execution_idempotency_key(refund_request_id: &str) -> String {
    format!("refund-request:payment-refund:{refund_request_id}")
}

pub fn withdrawal_account_hold_idempotency_key(withdrawal_request_id: &str) -> String {
    format!("withdrawal:account-hold:{withdrawal_request_id}")
}

pub fn withdrawal_payment_execution_idempotency_key(withdrawal_request_id: &str) -> String {
    format!("withdrawal:payment-payout:{withdrawal_request_id}")
}

pub const ACCOUNT_VALUE_LEDGER_PORT: &str = "account.value.ledger";
pub const PAYMENT_REFUND_EXECUTOR_PORT: &str = "payment.refund.executor";
pub const PAYMENT_PAYOUT_EXECUTOR_PORT: &str = "payment.payout.executor";
pub const COUPON_REDEMPTION_PORT: &str = "coupon.redemption";

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
