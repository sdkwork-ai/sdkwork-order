//! Refund notify orchestration ports（退款通知框架端口）。
//!
//! The refund notify flow system mirrors the payment notify building block
//! but is an independent pipeline: provider refund notifications arrive at
//! their own URL (`/app/v3/api/orders/refunds/webhooks/{providerCode}`),
//! verify through the shared verification port, ingest through the
//! payment-domain refund ingestion (which advances the `commerce_refund`
//! status machine), and settle the order-side refund state through
//! [`RefundNotifyStatePort`]. Order domain owns the order state; the payment
//! domain owns the refund facts — the two sides only communicate through
//! these ports (high cohesion, low coupling).

use std::future::Future;
use std::pin::Pin;

use sdkwork_contract_service::CommerceServiceError;

use crate::ports::{
    AccountValueLedgerPort, AccountValueRequestExecutionStore, PaymentNotifyAttemptContext,
    PhysicalInventoryReservationPort,
};
use crate::OrderPaymentSettlementAttempt;

/// Refund fact resolved by the payment-domain refund ingestion after the
/// `commerce_refund` status machine applied the provider notification.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefundNotifyContext {
    pub refund_id: String,
    pub refund_no: String,
    pub order_id: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    /// Applied commerce refund status (`succeeded`/`failed`/`canceled`/`processing`).
    pub status: String,
    pub amount: String,
    /// Canonical business type for refund post-processing. Defaults to
    /// `refund`; business flows (e.g. account-value refund hold release,
    /// after-sales linkage) register a `RefundNotifyHandler` for their type.
    pub business_type: String,
}

/// Canonical default business type for refund post-processing.
pub const REFUND_NOTIFY_BUSINESS_REFUND: &str = "refund";

pub type RefundNotifyHandlerFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), CommerceServiceError>> + Send + 'a>>;

/// Business-specific refund post-processing hook. The canonical order
/// `refund_status` update always runs in the pipeline; handlers add
/// business effects (releasing holds, linking after-sales requests) and MUST
/// be idempotent (stable keys per refund).
pub trait RefundNotifyHandler: Send + Sync {
    fn business_type(&self) -> &'static str;

    fn handle<'a>(
        &'a self,
        ctx: &'a RefundNotifyContext,
        attempt: &'a PaymentNotifyAttemptContext,
    ) -> RefundNotifyHandlerFuture<'a>;
}

/// Resolves the handler registered for a refund business type. Unknown types
/// resolve to `None` and the pipeline completes without post-processing.
pub trait RefundNotifyHandlerRegistry: Send + Sync {
    fn resolve(&self, business_type: &str) -> Option<std::sync::Arc<dyn RefundNotifyHandler>>;
}

/// Outcome of persisting a refund notification (idempotent ingest).
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefundNotifyIngestOutcome {
    pub webhook_event_id: String,
    pub replayed: bool,
    pub refund: Option<RefundNotifyContext>,
    /// Original payment attempt context when the out-trade-no resolves, so
    /// the order side can scope the order-state write.
    pub payment_attempt: Option<PaymentNotifyAttemptContext>,
}

pub type RefundNotifyIngestFuture<'a> = Pin<
    Box<dyn Future<Output = Result<RefundNotifyIngestOutcome, CommerceServiceError>> + Send + 'a>,
>;

/// Persists the refund notification idempotently and advances the refund
/// status machine (payment domain implementation).
pub trait RefundNotifyIngestPort: Send + Sync {
    fn ingest<'a>(
        &'a self,
        event: crate::ports::PaymentNotifyEvent,
    ) -> RefundNotifyIngestFuture<'a>;
}

/// Outcome of advancing the owner order refund state.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OwnerOrderRefundStateOutcome {
    pub refund_status: String,
    /// True when the order was already in a terminal refund state and the
    /// write was suppressed (idempotent replay protection).
    pub terminal_preserved: bool,
}

pub type OwnerOrderRefundStateFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<OwnerOrderRefundStateOutcome, CommerceServiceError>> + Send + 'a,
    >,
>;

/// Order-owned persistence boundary for the refund-notify part of the flow.
/// Only the order repository advances `commerce_order.refund_status`.
///
/// The default implementation fails loudly so deployments without the
/// order-owned Postgres store notice that refund-state marking is
/// unconfigured instead of silently acking the notification.
pub trait RefundNotifyStatePort: Send + Sync {
    fn mark_owner_order_refund_status<'a>(
        &'a self,
        _attempt: &'a OrderPaymentSettlementAttempt,
        _refund_status: &'a str,
    ) -> OwnerOrderRefundStateFuture<'a> {
        Box::pin(async move {
            Err(CommerceServiceError::provider_unavailable(
                "order refund state marking is not configured",
            ))
        })
    }
}

/// A `commerce_order_refund_request` row resolved behind a PSP refund
/// notification. The linkage is deterministic: the refund execution writes
/// `commerce_refund.idempotency_key =
/// refund_payment_execution_idempotency_key(request_id)`, so the merchant
/// refund number resolves to exactly one refund request (when the refund
/// originates from one).
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefundRequestSettlementContext {
    pub request_id: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub original_order_id: String,
    /// Account asset code (`AccountValueAssetCode::parse` input).
    pub target_asset: String,
    /// Held amount, digit-only smallest-unit string.
    pub amount: String,
    pub currency_code: String,
    pub request_no: String,
    pub status: String,
    /// The account-hold effect id recorded when the execution placed the
    /// hold (`account_effect_reference_id`).
    pub hold_id: Option<String>,
}

/// Order-owned settlement of a refund REQUEST driven by the PSP refund
/// webhook/compensation outcome. Only flips requests that are still
/// in-flight (`provider_refund_processing`); terminal requests return
/// `false` so replays are no-ops.
pub type RefundRequestSettlementFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait RefundRequestSettlementPort: Send + Sync {
    fn load_refund_request_for_settlement<'a>(
        &'a self,
        tenant_id: &'a str,
        refund_no: &'a str,
    ) -> RefundRequestSettlementFuture<'a, Option<RefundRequestSettlementContext>>;

    fn mark_refund_request_settled<'a>(
        &'a self,
        tenant_id: &'a str,
        request_id: &'a str,
        target_status: &'a str,
        provider_reference_id: Option<&'a str>,
    ) -> RefundRequestSettlementFuture<'a, bool>;
}

/// Default refund post-processing for request-backed refunds
/// (`business_type = "refund"`): when the PSP refund notification lands, the
/// refund request that placed the account-value hold is terminalized and the
/// hold is settled (money actually left) or released (refund failed) with
/// the exact idempotency keys the synchronous execution path uses, so a
/// webhook that races the sync path collapses to a no-op. Refunds that do
/// not originate from a refund request complete without post-processing.
pub struct AccountValueRefundNotifyHandler {
    pub settlement: std::sync::Arc<dyn RefundRequestSettlementPort>,
    pub ledger: std::sync::Arc<dyn AccountValueLedgerPort>,
    pub inventory: Option<std::sync::Arc<dyn PhysicalInventoryReservationPort>>,
    pub order_state: Option<std::sync::Arc<dyn AccountValueRequestExecutionStore>>,
}

impl RefundNotifyHandler for AccountValueRefundNotifyHandler {
    fn business_type(&self) -> &'static str {
        REFUND_NOTIFY_BUSINESS_REFUND
    }

    fn handle<'a>(
        &'a self,
        ctx: &'a RefundNotifyContext,
        _attempt: &'a PaymentNotifyAttemptContext,
    ) -> RefundNotifyHandlerFuture<'a> {
        Box::pin(async move {
            if !matches!(ctx.status.as_str(), "succeeded" | "failed") {
                // Processing/unknown states carry no money movement yet.
                return Ok(());
            }
            let Some(request) = self
                .settlement
                .load_refund_request_for_settlement(&ctx.tenant_id, &ctx.refund_no)
                .await?
            else {
                return Ok(());
            };
            const IN_FLIGHT: &str = "provider_refund_processing";
            if request.status != IN_FLIGHT {
                // Terminal (or not yet held): the synchronous execution path
                // already owns the outcome.
                return Ok(());
            }
            let Some(hold_id) = request.hold_id.as_deref() else {
                tracing::warn!(
                    target = "order.refund_notify",
                    refund_no = %ctx.refund_no,
                    request_id = %request.request_id,
                    "refund request has no account hold reference; skipping hold settlement"
                );
                return Ok(());
            };
            let asset = crate::AccountValueAssetCode::parse(&request.target_asset)?;
            let amount = sdkwork_contract_service::CommerceMoney::new(&request.amount)
                .map_err(CommerceServiceError::storage)?;
            let terminal_status = if ctx.status == "succeeded" {
                self.ledger
                    .apply_account_value_ledger_command(
                        crate::AccountValueLedgerCommand::hold_settle(
                            &request.tenant_id,
                            request.organization_id.as_deref(),
                            &request.owner_user_id,
                            asset,
                            amount,
                            &request.currency_code,
                            crate::AccountValueOrderSubject::RefundRequest
                                .compensation_business_type(asset),
                            hold_id,
                            &format!("{}:settle", request.request_no),
                            &format!(
                                "{}:{hold_id}:settle",
                                crate::refund_account_hold_idempotency_key(&request.request_id)
                            ),
                        )?,
                    )
                    .await?;
                "refunded"
            } else {
                self.ledger
                    .apply_account_value_ledger_command(
                        crate::AccountValueLedgerCommand::hold_release(
                            &request.tenant_id,
                            request.organization_id.as_deref(),
                            &request.owner_user_id,
                            asset,
                            amount,
                            &request.currency_code,
                            webhook_release_business_type(asset),
                            hold_id,
                            &format!("{}:release", request.request_no),
                            &format!(
                                "{}:{hold_id}:release",
                                crate::refund_account_hold_idempotency_key(&request.request_id)
                            ),
                        )?,
                    )
                    .await?;
                "provider_refund_failed"
            };
            let settled = self
                .settlement
                .mark_refund_request_settled(
                    &request.tenant_id,
                    &request.request_id,
                    terminal_status,
                    Some(&ctx.refund_no),
                )
                .await?;
            if !settled {
                // Lost a race with the synchronous path; its outcome stands.
                return Ok(());
            }
            if let Some(order_state) = self.order_state.as_ref() {
                if let Err(error) = order_state
                    .mark_owner_order_refunded(
                        &request.tenant_id,
                        request.organization_id.as_deref(),
                        &request.owner_user_id,
                        &request.original_order_id,
                        terminal_status,
                    )
                    .await
                {
                    tracing::warn!(
                        target = "order.refund_notify",
                        order_id = %request.original_order_id,
                        error = ?error,
                        "failed to sync owner order refund status after webhook settlement"
                    );
                }
            }
            if ctx.status == "succeeded" {
                if let Some(inventory) = self.inventory.as_ref() {
                    if let Err(error) = inventory
                        .release_physical_order_inventory(
                            crate::ReleasePhysicalOrderInventoryRequest {
                                tenant_id: request.tenant_id.clone(),
                                order_id: request.original_order_id.clone(),
                                reason_code: "refund".to_owned(),
                                request_no: format!("refund-{}", request.request_id),
                                idempotency_key: crate::physical_inventory_release_idempotency_key(
                                    &request.original_order_id,
                                ),
                            },
                        )
                        .await
                    {
                        tracing::warn!(
                            target = "order.refund_notify",
                            order_id = %request.original_order_id,
                            error = ?error,
                            "failed to release physical inventory after webhook refund settlement"
                        );
                    }
                }
            }
            Ok(())
        })
    }
}

fn webhook_release_business_type(asset: crate::AccountValueAssetCode) -> &'static str {
    use crate::AccountValueAssetCode as Asset;
    match asset {
        Asset::TokenBank => {
            sdkwork_contract_service::CommerceLedgerBusinessType::TOKEN_BANK_HOLD_RELEASE
        }
        Asset::Points => sdkwork_contract_service::CommerceLedgerBusinessType::POINTS_CLAWBACK,
        Asset::Subscription => {
            sdkwork_contract_service::CommerceLedgerBusinessType::MANUAL_ADJUSTMENT
        }
        Asset::Cash => sdkwork_contract_service::CommerceLedgerBusinessType::CASH_ADJUSTMENT,
    }
}

/// Builds the refund notify handler registry carrying the default
/// request-settlement handler. Deployments pass their ledger port (the same
/// one the synchronous execution path uses), optionally the physical
/// inventory reservation port for post-refund stock release, and optionally
/// the execution store for owner-order refund-status sync.
pub fn refund_notify_handler_registry_with(
    settlement: std::sync::Arc<dyn RefundRequestSettlementPort>,
    ledger: std::sync::Arc<dyn AccountValueLedgerPort>,
    inventory: Option<std::sync::Arc<dyn PhysicalInventoryReservationPort>>,
    order_state: Option<std::sync::Arc<dyn AccountValueRequestExecutionStore>>,
) -> std::sync::Arc<dyn RefundNotifyHandlerRegistry> {
    std::sync::Arc::new(SingleRefundHandlerRegistry {
        handler: std::sync::Arc::new(AccountValueRefundNotifyHandler {
            settlement,
            ledger,
            inventory,
            order_state,
        }),
    })
}

/// Single-handler registry for the default refund post-processing mount.
struct SingleRefundHandlerRegistry {
    handler: std::sync::Arc<dyn RefundNotifyHandler>,
}

impl RefundNotifyHandlerRegistry for SingleRefundHandlerRegistry {
    fn resolve(&self, business_type: &str) -> Option<std::sync::Arc<dyn RefundNotifyHandler>> {
        (business_type == REFUND_NOTIFY_BUSINESS_REFUND).then(|| self.handler.clone())
    }
}
