use std::{error::Error, fmt};

use sdkwork_contract_service::CommerceServiceError;

use crate::{
    coupon_recharge_redemption_idempotency_key, fulfill_account_value_order, AccountValueAssetCode,
    AccountValueFulfillmentContext, AccountValueFulfillmentStore, AccountValueLedgerPort,
    AccountValueOrderSubject, CouponRedemptionBenefit, CouponRedemptionPort,
    CouponRedemptionRequest, CouponSubscriptionFulfillmentRequest, FulfillAccountValueOrderCommand,
    FulfillAccountValueOrderOutcome, MembershipPurchaseFulfillmentPort,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CouponFulfilledBenefit {
    TokenBankCredit {
        grant_amount: sdkwork_contract_service::CommerceMoney,
    },
    PointsCredit {
        grant_amount: sdkwork_contract_service::CommerceMoney,
    },
    CashCredit {
        grant_amount: sdkwork_contract_service::CommerceMoney,
        currency_code: String,
    },
    Subscription {
        product_id: String,
        sku_id: String,
        package_id: i64,
        period: String,
        duration_days: i64,
        daily_quota: i64,
        total_quota: i64,
        subscription_id: String,
        starts_at: String,
        expires_at: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CouponFulfillmentOutcome {
    pub order_id: String,
    pub order_no: String,
    pub replayed: bool,
    pub fulfillment_status: String,
    pub benefit: CouponFulfilledBenefit,
}

/// Stage at which a coupon-redemption fulfilment attempt failed.
///
/// The distinction is contractual for the app surface: a rejection that never
/// consumed the coupon keeps its natural HTTP class, while a coupon that was
/// consumed with the benefit still undelivered is a retryable dependency
/// failure. Collapsing both into one generic failure is what made the original
/// incident impossible for a client to reason about.
#[derive(Debug)]
pub enum CouponFulfillmentError {
    /// Failed before the coupon was consumed; nothing was redeemed.
    BeforeRedemption(CommerceServiceError),
    /// The coupon was consumed and the order exists, but the benefit was not
    /// delivered. Retrying the same idempotency key is safe and cannot consume
    /// a second coupon because redemption is keyed by the persisted order id.
    BenefitDelivery(CommerceServiceError),
}

impl CouponFulfillmentError {
    /// Whether this attempt already consumed the coupon.
    pub fn coupon_consumed(&self) -> bool {
        matches!(self, Self::BenefitDelivery(_))
    }

    /// Borrows the underlying commerce error for logging or failure evidence.
    pub fn service_error(&self) -> &CommerceServiceError {
        match self {
            Self::BeforeRedemption(error) | Self::BenefitDelivery(error) => error,
        }
    }

    /// Returns the underlying commerce error for callers that only need the
    /// original classification.
    pub fn into_service_error(self) -> CommerceServiceError {
        match self {
            Self::BeforeRedemption(error) | Self::BenefitDelivery(error) => error,
        }
    }
}

impl fmt::Display for CouponFulfillmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeforeRedemption(error) => {
                write!(formatter, "coupon was not consumed: {}", error.message())
            }
            Self::BenefitDelivery(error) => write!(
                formatter,
                "coupon was consumed but benefit delivery failed: {}",
                error.message()
            ),
        }
    }
}

impl Error for CouponFulfillmentError {}

pub async fn redeem_coupon_and_fulfill_order<S, C, L, M>(
    store: &S,
    coupon_port: &C,
    ledger_port: &L,
    membership_port: &M,
    command: FulfillAccountValueOrderCommand,
) -> Result<CouponFulfillmentOutcome, CouponFulfillmentError>
where
    S: AccountValueFulfillmentStore + ?Sized,
    C: CouponRedemptionPort + ?Sized,
    L: AccountValueLedgerPort + ?Sized,
    M: MembershipPurchaseFulfillmentPort + ?Sized,
{
    let Some(context) = store
        .load_account_value_fulfillment_context(&command)
        .await
        .map_err(CouponFulfillmentError::BeforeRedemption)?
    else {
        return Err(CouponFulfillmentError::BeforeRedemption(
            CommerceServiceError::not_found("coupon redemption order was not found"),
        ));
    };
    if context.subject != AccountValueOrderSubject::CouponRecharge {
        return Err(CouponFulfillmentError::BeforeRedemption(
            CommerceServiceError::validation(
                "coupon redemption fulfillment requires a coupon order",
            ),
        ));
    }
    let coupon_code = context
        .coupon_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CouponFulfillmentError::BeforeRedemption(CommerceServiceError::invalid_state(
                "coupon order has no coupon code",
            ))
        })?;
    let expected_benefit = context.coupon_benefit.clone().unwrap_or_else(|| {
        CouponRedemptionBenefit::TokenBankCredit {
            grant_amount: context.grant_amount.clone(),
        }
    });

    let redemption = coupon_port
        .redeem_coupon(CouponRedemptionRequest {
            tenant_id: command.tenant_id.clone(),
            organization_id: command.organization_id.clone(),
            owner_user_id: command.owner_user_id.clone(),
            coupon_code: coupon_code.to_owned(),
            order_id: context.order_id.clone(),
            request_no: context.order_id.clone(),
            idempotency_key: coupon_recharge_redemption_idempotency_key(&context.order_id),
        })
        .await
        .map_err(CouponFulfillmentError::BeforeRedemption)?;
    if !redemption.accepted || redemption.benefit != expected_benefit {
        return Err(CouponFulfillmentError::BeforeRedemption(
            CommerceServiceError::conflict("coupon benefit changed before fulfillment"),
        ));
    }

    match expected_benefit {
        CouponRedemptionBenefit::TokenBankCredit { grant_amount } => {
            let fulfilled = fulfill_coupon_asset_grant(
                store,
                ledger_port,
                command,
                &context,
                AccountValueAssetCode::TokenBank,
                &grant_amount,
            )
            .await
            .map_err(CouponFulfillmentError::BenefitDelivery)?;
            Ok(CouponFulfillmentOutcome {
                order_id: fulfilled.order_id,
                order_no: fulfilled.order_no,
                replayed: fulfilled.replayed || redemption.replayed,
                fulfillment_status: fulfilled.fulfillment_status,
                benefit: CouponFulfilledBenefit::TokenBankCredit { grant_amount },
            })
        }
        CouponRedemptionBenefit::PointsCredit { grant_amount } => {
            let fulfilled = fulfill_coupon_asset_grant(
                store,
                ledger_port,
                command,
                &context,
                AccountValueAssetCode::Points,
                &grant_amount,
            )
            .await
            .map_err(CouponFulfillmentError::BenefitDelivery)?;
            Ok(CouponFulfillmentOutcome {
                order_id: fulfilled.order_id,
                order_no: fulfilled.order_no,
                replayed: fulfilled.replayed || redemption.replayed,
                fulfillment_status: fulfilled.fulfillment_status,
                benefit: CouponFulfilledBenefit::PointsCredit { grant_amount },
            })
        }
        CouponRedemptionBenefit::CashCredit {
            grant_amount,
            currency_code,
        } => {
            let fulfilled = fulfill_coupon_asset_grant(
                store,
                ledger_port,
                command,
                &context,
                AccountValueAssetCode::Cash,
                &grant_amount,
            )
            .await
            .map_err(CouponFulfillmentError::BenefitDelivery)?;
            Ok(CouponFulfillmentOutcome {
                order_id: fulfilled.order_id,
                order_no: fulfilled.order_no,
                replayed: fulfilled.replayed || redemption.replayed,
                fulfillment_status: fulfilled.fulfillment_status,
                benefit: CouponFulfilledBenefit::CashCredit {
                    grant_amount,
                    currency_code,
                },
            })
        }
        CouponRedemptionBenefit::Subscription {
            product_id,
            sku_id,
            package_id,
            period,
            duration_days,
            daily_quota,
            total_quota,
        } => {
            if context.target_asset != AccountValueAssetCode::Subscription {
                return Err(CouponFulfillmentError::BeforeRedemption(
                    CommerceServiceError::conflict(
                        "coupon order subscription snapshot has an invalid target",
                    ),
                ));
            }
            store
                .reserve_account_value_fulfillment(&command, &context)
                .await
                .map_err(CouponFulfillmentError::BenefitDelivery)?;
            let membership = membership_port
                .fulfill_coupon_subscription(CouponSubscriptionFulfillmentRequest {
                    tenant_id: command.tenant_id.clone(),
                    organization_id: command.organization_id.clone(),
                    owner_user_id: command.owner_user_id.clone(),
                    order_id: context.order_id.clone(),
                    product_id: product_id.clone(),
                    sku_id: sku_id.clone(),
                    package_id,
                    period: period.as_str().to_owned(),
                    duration_days,
                    daily_quota,
                    total_quota,
                    request_no: command.request_no.clone(),
                    idempotency_key: format!("coupon-subscription:fulfill:{}", context.order_id),
                })
                .await;
            let membership = match membership {
                Ok(value) if value.accepted => value,
                Ok(_) => {
                    store
                        .release_account_value_fulfillment_reservation(&command, &context)
                        .await
                        .map_err(CouponFulfillmentError::BenefitDelivery)?;
                    return Err(CouponFulfillmentError::BenefitDelivery(
                        CommerceServiceError::invalid_state(
                            "membership rejected coupon subscription fulfillment",
                        ),
                    ));
                }
                Err(error) => {
                    store
                        .release_account_value_fulfillment_reservation(&command, &context)
                        .await
                        .map_err(CouponFulfillmentError::BenefitDelivery)?;
                    return Err(CouponFulfillmentError::BenefitDelivery(error));
                }
            };
            let committed = store
                .commit_account_value_fulfillment(command.clone(), &context)
                .await;
            let committed = match committed {
                Ok(value) => value,
                Err(error) => {
                    store
                        .release_account_value_fulfillment_reservation(&command, &context)
                        .await
                        .map_err(CouponFulfillmentError::BenefitDelivery)?;
                    return Err(CouponFulfillmentError::BenefitDelivery(error));
                }
            };
            Ok(CouponFulfillmentOutcome {
                order_id: committed.order_id,
                order_no: committed.order_no,
                replayed: committed.replayed || redemption.replayed || membership.replayed,
                fulfillment_status: membership.fulfillment_status,
                benefit: CouponFulfilledBenefit::Subscription {
                    product_id,
                    sku_id,
                    package_id,
                    period: period.as_str().to_owned(),
                    duration_days,
                    daily_quota,
                    total_quota,
                    subscription_id: membership.subscription_id,
                    starts_at: membership.starts_at,
                    expires_at: membership.expires_at,
                },
            })
        }
    }
}

/// 发放型资产券（Token Bank 额度 / 积分 / 现金）通用履约：校验订单快照与权益一致后入账。
async fn fulfill_coupon_asset_grant<S, L>(
    store: &S,
    ledger_port: &L,
    command: FulfillAccountValueOrderCommand,
    context: &AccountValueFulfillmentContext,
    expected_asset: AccountValueAssetCode,
    grant_amount: &sdkwork_contract_service::CommerceMoney,
) -> Result<FulfillAccountValueOrderOutcome, CommerceServiceError>
where
    S: AccountValueFulfillmentStore + ?Sized,
    L: AccountValueLedgerPort + ?Sized,
{
    if context.target_asset != expected_asset || context.grant_amount != *grant_amount {
        return Err(CommerceServiceError::conflict(
            "coupon order asset snapshot does not match its benefit",
        ));
    }
    fulfill_account_value_order(store, ledger_port, command).await
}

pub async fn redeem_coupon_and_fulfill_account_value_order<S, C, L>(
    store: &S,
    coupon_port: &C,
    ledger_port: &L,
    command: FulfillAccountValueOrderCommand,
) -> Result<FulfillAccountValueOrderOutcome, CommerceServiceError>
where
    S: AccountValueFulfillmentStore + ?Sized,
    C: CouponRedemptionPort + ?Sized,
    L: AccountValueLedgerPort + ?Sized,
{
    let Some(context) = store
        .load_account_value_fulfillment_context(&command)
        .await?
    else {
        return Err(CommerceServiceError::not_found(
            "coupon recharge order was not found",
        ));
    };

    if context.subject != AccountValueOrderSubject::CouponRecharge {
        return Err(CommerceServiceError::validation(
            "coupon recharge fulfillment requires a coupon order",
        ));
    }
    if !matches!(
        context.target_asset,
        AccountValueAssetCode::TokenBank
            | AccountValueAssetCode::Points
            | AccountValueAssetCode::Cash
    ) {
        return Err(CommerceServiceError::validation(
            "coupon recharge fulfillment requires an asset grant coupon order",
        ));
    }
    if context.already_fulfilled() {
        return Ok(FulfillAccountValueOrderOutcome::replayed(&context));
    }

    let coupon_code = context
        .coupon_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CommerceServiceError::invalid_state("coupon recharge order has no coupon code")
        })?;
    let redemption = coupon_port
        .redeem_coupon(CouponRedemptionRequest {
            tenant_id: command.tenant_id.clone(),
            organization_id: command.organization_id.clone(),
            owner_user_id: command.owner_user_id.clone(),
            coupon_code: coupon_code.to_owned(),
            order_id: context.order_id.clone(),
            request_no: context.order_id.clone(),
            idempotency_key: coupon_recharge_redemption_idempotency_key(&context.order_id),
        })
        .await?;

    let benefit_matches = redemption.benefit.target_asset() == context.target_asset
        && redemption.benefit.grant_amount() == context.grant_amount;
    if !redemption.accepted || !benefit_matches {
        return Err(CommerceServiceError::conflict(
            "coupon benefit changed before asset fulfillment",
        ));
    }

    fulfill_account_value_order(store, ledger_port, command).await
}
