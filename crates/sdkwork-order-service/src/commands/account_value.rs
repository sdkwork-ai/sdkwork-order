use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

use crate::{
    AccountValueAssetCode, AccountValueOrderSubject, CouponRedemptionBenefit, TokenBankPlanPeriod,
};

/// Server-owned pricing for an account-value recharge.
///
/// The client must never decide how many credits a Token Bank purchase grants
/// or what a Token Bank plan costs. `payable_amount` is `Some` when the seller
/// catalog owns the price (plan/package); the route then replaces the
/// client-supplied amount with it, and `payable_currency_code` replaces the
/// client-supplied currency.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountValueRechargePricing {
    pub grant_amount: CommerceMoney,
    pub payable_amount: Option<CommerceMoney>,
    pub payable_currency_code: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateAccountRechargeOrderCommand {
    pub amount: CommerceMoney,
    pub client_request_no: Option<String>,
    pub currency_code: String,
    pub expire_at: String,
    pub grant_amount: CommerceMoney,
    pub idempotency_key: String,
    pub order_id: String,
    pub order_item_id: String,
    pub order_no: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub package_id: Option<String>,
    pub plan_code: Option<String>,
    pub plan_period: Option<TokenBankPlanPeriod>,
    pub requested_at: String,
    pub subject: AccountValueOrderSubject,
    pub target_asset: AccountValueAssetCode,
    pub tenant_id: String,
    pub out_trade_no: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateCouponRechargeOrderCommand {
    pub amount: CommerceMoney,
    pub coupon_code: String,
    pub benefit: CouponRedemptionBenefit,
    pub currency_code: String,
    pub grant_amount: CommerceMoney,
    pub idempotency_key: String,
    pub order_id: String,
    pub order_item_id: String,
    pub order_no: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub payment_required: bool,
    pub subject: AccountValueOrderSubject,
    pub target_asset: AccountValueAssetCode,
    pub tenant_id: String,
    pub out_trade_no: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateOrderRefundRequestCommand {
    pub amount: CommerceMoney,
    pub currency_code: String,
    pub idempotency_key: String,
    pub original_order_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub provider_amount: Option<CommerceMoney>,
    pub provider_currency_code: Option<String>,
    pub reason_code: Option<String>,
    pub reason_detail: Option<String>,
    pub refund_request_id: String,
    pub request_no: String,
    pub subject: AccountValueOrderSubject,
    pub target_asset: AccountValueAssetCode,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateCashWithdrawalRequestCommand {
    pub amount: CommerceMoney,
    pub asset: AccountValueAssetCode,
    pub currency_code: String,
    pub idempotency_key: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub provider_amount: Option<CommerceMoney>,
    pub provider_currency_code: Option<String>,
    pub payout_account_ref: Option<String>,
    pub payout_method: Option<String>,
    pub reason_code: Option<String>,
    pub request_no: String,
    pub subject: AccountValueOrderSubject,
    pub tenant_id: String,
    pub withdrawal_request_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountValueRequestReviewAction {
    Approve,
    Reject,
    Retry,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpsertAccountValuePackageCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub package_id: Option<String>,
    pub package_code: String,
    pub display_name: String,
    pub target_asset: AccountValueAssetCode,
    pub grant_amount: CommerceMoney,
    pub bonus_amount: CommerceMoney,
    pub price_amount: CommerceMoney,
    pub currency_code: String,
    pub status: String,
    pub sort_weight: i64,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetireAccountValuePackageCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub package_id: String,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpsertTokenBankPlanCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub plan_code: String,
    pub display_name: String,
    pub plan_period: TokenBankPlanPeriod,
    pub grant_amount: CommerceMoney,
    pub bonus_amount: CommerceMoney,
    pub price_amount: CommerceMoney,
    pub currency_code: String,
    pub renewal_policy: String,
    pub status: String,
    pub sort_weight: i64,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetireTokenBankPlanCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub plan_code: String,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewAccountValueRequestCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub subject: AccountValueOrderSubject,
    pub request_id: String,
    pub action: AccountValueRequestReviewAction,
    pub reason_code: Option<String>,
    pub review_comment: Option<String>,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FulfillAccountValueOrderCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
    pub request_no: String,
    pub idempotency_key: String,
}

impl AccountValueRequestReviewAction {
    pub fn parse(value: &str) -> Result<Self, CommerceServiceError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "approve" | "approved" => Ok(Self::Approve),
            "reject" | "rejected" => Ok(Self::Reject),
            "retry" | "processing" => Ok(Self::Retry),
            _ => Err(CommerceServiceError::validation(
                "unsupported account value request review action",
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Reject => "reject",
            Self::Retry => "retry",
        }
    }

    pub fn next_status(self) -> &'static str {
        match self {
            Self::Approve => "approved",
            Self::Reject => "rejected",
            Self::Retry => "processing",
        }
    }
}

impl CreateAccountRechargeOrderCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        subject: AccountValueOrderSubject,
        target_asset: AccountValueAssetCode,
        amount: CommerceMoney,
        currency_code: &str,
        order_id: &str,
        order_item_id: &str,
        order_no: &str,
        out_trade_no: &str,
        requested_at: &str,
        expire_at: &str,
        idempotency_key: &str,
        package_id: Option<&str>,
        plan: Option<(&str, TokenBankPlanPeriod)>,
        client_request_no: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        if !subject.is_recharge_order_subject() {
            return Err(CommerceServiceError::validation(
                "account recharge order subject must be a paid account value subject",
            ));
        }
        subject.validate_target_asset(target_asset)?;
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("currency_code", currency_code)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        crate::validation::require_non_empty("order_item_id", order_item_id)?;
        crate::validation::require_non_empty("order_no", order_no)?;
        crate::validation::require_non_empty("out_trade_no", out_trade_no)?;
        crate::validation::require_non_empty("requested_at", requested_at)?;
        crate::validation::require_non_empty("expire_at", expire_at)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        let (plan_code, plan_period) = match plan {
            Some((plan_code, period)) => {
                crate::validation::require_non_empty("plan_code", plan_code)?;
                (Some(plan_code.trim().to_string()), Some(period))
            }
            None => (None, None),
        };

        if matches!(
            subject,
            AccountValueOrderSubject::TokenBankPlanPurchase
                | AccountValueOrderSubject::TokenBankPlanRenewal
        ) && plan_code.is_none()
        {
            return Err(CommerceServiceError::validation(
                "Token Bank plan order requires plan_code and plan_period",
            ));
        }

        if matches!(subject, AccountValueOrderSubject::AccountRechargePackage)
            && optional_text(package_id).is_none()
        {
            return Err(CommerceServiceError::validation(
                "account recharge package order requires package_id",
            ));
        }

        Ok(Self {
            grant_amount: amount.clone(),
            amount,
            client_request_no: optional_text(client_request_no),
            currency_code: currency_code.trim().to_ascii_uppercase(),
            expire_at: expire_at.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
            order_id: order_id.trim().to_string(),
            order_item_id: order_item_id.trim().to_string(),
            order_no: order_no.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            package_id: optional_text(package_id),
            plan_code,
            plan_period,
            requested_at: requested_at.trim().to_string(),
            subject,
            target_asset,
            tenant_id: tenant_id.trim().to_string(),
            out_trade_no: out_trade_no.trim().to_string(),
        })
    }
}

impl CreateCouponRechargeOrderCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        target_asset: AccountValueAssetCode,
        amount: CommerceMoney,
        currency_code: &str,
        order_id: &str,
        order_item_id: &str,
        order_no: &str,
        out_trade_no: &str,
        coupon_code: &str,
        idempotency_key: &str,
        payment_required: bool,
    ) -> Result<Self, CommerceServiceError> {
        Self::new_with_benefit(
            tenant_id,
            organization_id,
            owner_user_id,
            target_asset,
            amount.clone(),
            currency_code,
            order_id,
            order_item_id,
            order_no,
            out_trade_no,
            coupon_code,
            idempotency_key,
            payment_required,
            CouponRedemptionBenefit::TokenBankCredit {
                grant_amount: amount,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_benefit(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        target_asset: AccountValueAssetCode,
        amount: CommerceMoney,
        currency_code: &str,
        order_id: &str,
        order_item_id: &str,
        order_no: &str,
        out_trade_no: &str,
        coupon_code: &str,
        idempotency_key: &str,
        payment_required: bool,
        benefit: CouponRedemptionBenefit,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("currency_code", currency_code)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        crate::validation::require_non_empty("order_item_id", order_item_id)?;
        crate::validation::require_non_empty("order_no", order_no)?;
        crate::validation::require_non_empty("out_trade_no", out_trade_no)?;
        crate::validation::require_non_empty("coupon_code", coupon_code)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;
        if benefit.target_asset() != target_asset {
            return Err(CommerceServiceError::validation(
                "coupon recharge order benefit target asset must match its order target asset",
            ));
        }

        Ok(Self {
            benefit,
            grant_amount: amount.clone(),
            amount,
            coupon_code: coupon_code.trim().to_string(),
            currency_code: currency_code.trim().to_ascii_uppercase(),
            idempotency_key: idempotency_key.trim().to_string(),
            order_id: order_id.trim().to_string(),
            order_item_id: order_item_id.trim().to_string(),
            order_no: order_no.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            payment_required,
            subject: AccountValueOrderSubject::CouponRecharge,
            target_asset,
            tenant_id: tenant_id.trim().to_string(),
            out_trade_no: out_trade_no.trim().to_string(),
        })
    }
}

impl CreateOrderRefundRequestCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        refund_request_id: &str,
        original_order_id: &str,
        target_asset: AccountValueAssetCode,
        amount: CommerceMoney,
        currency_code: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("refund_request_id", refund_request_id)?;
        crate::validation::require_non_empty("original_order_id", original_order_id)?;
        crate::validation::require_non_empty("currency_code", currency_code)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            amount,
            currency_code: currency_code.trim().to_ascii_uppercase(),
            idempotency_key: idempotency_key.trim().to_string(),
            original_order_id: original_order_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            provider_amount: None,
            provider_currency_code: None,
            reason_code: None,
            reason_detail: None,
            refund_request_id: refund_request_id.trim().to_string(),
            request_no: refund_request_id.trim().to_string(),
            subject: AccountValueOrderSubject::RefundRequest,
            target_asset,
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

impl CreateCashWithdrawalRequestCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        withdrawal_request_id: &str,
        asset: AccountValueAssetCode,
        amount: CommerceMoney,
        currency_code: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        AccountValueOrderSubject::CashWithdrawal.validate_target_asset(asset)?;
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("withdrawal_request_id", withdrawal_request_id)?;
        crate::validation::require_non_empty("currency_code", currency_code)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            amount,
            asset,
            currency_code: currency_code.trim().to_ascii_uppercase(),
            idempotency_key: idempotency_key.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            provider_amount: None,
            provider_currency_code: None,
            payout_account_ref: None,
            payout_method: None,
            reason_code: None,
            request_no: withdrawal_request_id.trim().to_string(),
            subject: AccountValueOrderSubject::CashWithdrawal,
            tenant_id: tenant_id.trim().to_string(),
            withdrawal_request_id: withdrawal_request_id.trim().to_string(),
        })
    }
}

impl UpsertAccountValuePackageCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        package_id: Option<&str>,
        package_code: &str,
        display_name: &str,
        target_asset: AccountValueAssetCode,
        grant_amount: CommerceMoney,
        bonus_amount: CommerceMoney,
        price_amount: CommerceMoney,
        currency_code: &str,
        status: Option<&str>,
        sort_weight: Option<i64>,
        valid_from: Option<&str>,
        valid_to: Option<&str>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("package_code", package_code)?;
        crate::validation::require_non_empty("display_name", display_name)?;
        crate::validation::require_non_empty("currency_code", currency_code)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            package_id: optional_text(package_id),
            package_code: package_code.trim().to_string(),
            display_name: display_name.trim().to_string(),
            target_asset,
            grant_amount,
            bonus_amount,
            price_amount,
            currency_code: currency_code.trim().to_ascii_uppercase(),
            status: normalize_status(status),
            sort_weight: sort_weight.unwrap_or(0),
            valid_from: optional_text(valid_from),
            valid_to: optional_text(valid_to),
            request_no: request_no.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
        })
    }
}

impl RetireAccountValuePackageCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        package_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("package_id", package_id)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            package_id: package_id.trim().to_string(),
            request_no: request_no.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
        })
    }
}

impl UpsertTokenBankPlanCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        plan_code: &str,
        display_name: &str,
        plan_period: TokenBankPlanPeriod,
        grant_amount: CommerceMoney,
        bonus_amount: CommerceMoney,
        price_amount: CommerceMoney,
        currency_code: &str,
        renewal_policy: Option<&str>,
        status: Option<&str>,
        sort_weight: Option<i64>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("plan_code", plan_code)?;
        crate::validation::require_non_empty("display_name", display_name)?;
        crate::validation::require_non_empty("currency_code", currency_code)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            plan_code: plan_code.trim().to_string(),
            display_name: display_name.trim().to_string(),
            plan_period,
            grant_amount,
            bonus_amount,
            price_amount,
            currency_code: currency_code.trim().to_ascii_uppercase(),
            renewal_policy: optional_text(renewal_policy)
                .map(|value| value.to_ascii_lowercase())
                .unwrap_or_else(|| {
                    if plan_period.is_continuous() {
                        "auto_renew".to_string()
                    } else {
                        "manual".to_string()
                    }
                }),
            status: normalize_status(status),
            sort_weight: sort_weight.unwrap_or(0),
            request_no: request_no.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
        })
    }
}

impl RetireTokenBankPlanCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        plan_code: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("plan_code", plan_code)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            plan_code: plan_code.trim().to_string(),
            request_no: request_no.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
        })
    }
}

impl ReviewAccountValueRequestCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        subject: AccountValueOrderSubject,
        request_id: &str,
        action: AccountValueRequestReviewAction,
        reason_code: Option<&str>,
        review_comment: Option<&str>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        if !matches!(
            subject,
            AccountValueOrderSubject::RefundRequest | AccountValueOrderSubject::CashWithdrawal
        ) {
            return Err(CommerceServiceError::validation(
                "account value request review only supports refund_request and cash_withdrawal",
            ));
        }
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("request_id", request_id)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            subject,
            request_id: request_id.trim().to_string(),
            action,
            reason_code: optional_text(reason_code),
            review_comment: optional_text(review_comment),
            request_no: request_no.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
        })
    }

    pub fn next_status(&self) -> &'static str {
        self.action.next_status()
    }
}

impl FulfillAccountValueOrderCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            order_id: order_id.trim().to_string(),
            request_no: request_no.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
        })
    }
}

fn normalize_status(value: Option<&str>) -> String {
    optional_text(value)
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "active".to_string())
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
