//! Order-side port for promotion-owned order discounts.
//!
//! The order module owns the amount breakdown and the payment lifecycle; the
//! promotion module owns discount applications, budget reservations and their
//! settlement. This port keeps that boundary explicit: the order asks promotion
//! to apply/settle/release a discount, and reprices its own breakdown from the
//! returned minor-unit amount.

use sdkwork_contract_service::CommerceServiceError;
use std::future::Future;
use std::pin::Pin;

pub type PromotionDiscountFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionDiscountRequest {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
    pub user_coupon_id: String,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionDiscountOutcome {
    /// Incremental discount in integer minor units.
    pub discount_amount_minor: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionDiscountSettlementRequest {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub order_id: String,
}

pub trait PromotionDiscountPort: Send + Sync {
    fn apply_discount<'a>(
        &'a self,
        request: PromotionDiscountRequest,
    ) -> PromotionDiscountFuture<'a, PromotionDiscountOutcome>;

    fn settle_discounts<'a>(
        &'a self,
        request: PromotionDiscountSettlementRequest,
    ) -> PromotionDiscountFuture<'a, usize>;

    fn release_discounts<'a>(
        &'a self,
        request: PromotionDiscountSettlementRequest,
    ) -> PromotionDiscountFuture<'a, usize>;
}

/// Used when the host has no promotion integration. Applying a discount is a
/// hard failure; settle/release are safe no-ops because no application exists.
pub struct NoopPromotionDiscountPort;

impl PromotionDiscountPort for NoopPromotionDiscountPort {
    fn apply_discount<'a>(
        &'a self,
        _request: PromotionDiscountRequest,
    ) -> PromotionDiscountFuture<'a, PromotionDiscountOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::conflict(
                "promotion discount integration is not configured",
            ))
        })
    }

    fn settle_discounts<'a>(
        &'a self,
        _request: PromotionDiscountSettlementRequest,
    ) -> PromotionDiscountFuture<'a, usize> {
        Box::pin(async move { Ok(0) })
    }

    fn release_discounts<'a>(
        &'a self,
        _request: PromotionDiscountSettlementRequest,
    ) -> PromotionDiscountFuture<'a, usize> {
        Box::pin(async move { Ok(0) })
    }
}
