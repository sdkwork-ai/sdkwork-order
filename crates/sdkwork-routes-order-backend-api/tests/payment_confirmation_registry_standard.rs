//! Parity between the two payment-success entry points.
//!
//! `specs/PAYMENT_CALLBACK_NOTIFY_SPEC.md` §5.4 requires the backend operator
//! confirmation and the PSP webhook intake to share ONE
//! `PaymentNotifyHandlerRegistry`: a business handler registered for the
//! provider callback must never be silently bypassed when an operator
//! re-confirms the same payment, and vice versa.
//!
//! These are compile-level + source-level gates. The behavioural coverage for
//! settlement itself lives in `sdkwork-order-service`'s notify orchestration
//! tests (which exercise the registry dispatch directly), because the routes
//! crate cannot fabricate an IAM request context without a live app surface.

use std::sync::Arc;

use axum::Router;
use sdkwork_order_service::{
    default_payment_notify_handler_registry, AccountPointsCreditFuture, AccountPointsCreditPort,
    NoopAccountValueLedgerPort, NoopAppTemplatePurchaseFulfillmentPort, NoopCouponRedemptionPort,
    NoopMembershipPurchaseFulfillmentPort, OwnerOrderPaymentReconciliationPort,
    PaymentNotifyHandlerRegistry, PointsRechargeCreditOutcome, PointsRechargeCreditRequest,
    UnavailableOwnerOrderPaymentReconciliationPort, UnavailablePhysicalGoodsFulfillmentPort,
    UnavailableVirtualGoodsFulfillmentPort,
};
use sdkwork_routes_order_backend_api::payment_confirmation_router_with_postgres_pool_and_ports;

/// The confirmation mount only needs the credit port for the points-recharge
/// handler, which these seam tests never exercise.
struct NoopAccountPointsCreditPort;

impl AccountPointsCreditPort for NoopAccountPointsCreditPort {
    fn credit_points_recharge<'a>(
        &'a self,
        _request: PointsRechargeCreditRequest,
    ) -> AccountPointsCreditFuture<'a, PointsRechargeCreditOutcome> {
        Box::pin(async move {
            Ok(PointsRechargeCreditOutcome {
                accepted: true,
                replayed: false,
            })
        })
    }

    fn reverse_points_recharge_credit<'a>(
        &'a self,
        _request: PointsRechargeCreditRequest,
    ) -> AccountPointsCreditFuture<'a, PointsRechargeCreditOutcome> {
        Box::pin(async move {
            Ok(PointsRechargeCreditOutcome {
                accepted: true,
                replayed: false,
            })
        })
    }
}

fn lazy_pool() -> sqlx::PgPool {
    sqlx::PgPool::connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .expect("lazy pool")
}

/// The registry seam exists on the confirmation mount, so a deployment can pass
/// the exact registry instance its webhook mount uses.
#[tokio::test]
async fn confirmation_mount_accepts_the_shared_handler_registry() {
    let registry: Arc<dyn PaymentNotifyHandlerRegistry> = default_payment_notify_handler_registry();
    let reconciliation: Arc<dyn OwnerOrderPaymentReconciliationPort> =
        Arc::new(UnavailableOwnerOrderPaymentReconciliationPort);

    let router: Router = payment_confirmation_router_with_postgres_pool_and_ports(
        lazy_pool(),
        Arc::new(NoopAccountPointsCreditPort),
        Arc::new(NoopAccountValueLedgerPort),
        Arc::new(NoopCouponRedemptionPort),
        Arc::new(NoopMembershipPurchaseFulfillmentPort),
        Arc::new(NoopAppTemplatePurchaseFulfillmentPort),
        reconciliation,
        Arc::new(UnavailablePhysicalGoodsFulfillmentPort),
        Arc::new(UnavailableVirtualGoodsFulfillmentPort),
        Some(registry),
    );
    let _: Router = router;
}

/// The confirmation mount carries every paid-order fulfillment family, including
/// the virtual-goods port. A mount that only knew about physical goods would
/// defer (or fail) every virtual order confirmed by an operator.
#[tokio::test]
async fn confirmation_mount_takes_the_virtual_goods_port() {
    let _build = |virtual_goods: Arc<dyn sdkwork_order_service::VirtualGoodsFulfillmentPort>| {
        payment_confirmation_router_with_postgres_pool_and_ports(
            lazy_pool(),
            Arc::new(NoopAccountPointsCreditPort),
            Arc::new(NoopAccountValueLedgerPort),
            Arc::new(NoopCouponRedemptionPort),
            Arc::new(NoopMembershipPurchaseFulfillmentPort),
            Arc::new(NoopAppTemplatePurchaseFulfillmentPort),
            Arc::new(UnavailableOwnerOrderPaymentReconciliationPort),
            Arc::new(UnavailablePhysicalGoodsFulfillmentPort),
            virtual_goods,
            None,
        )
    };
}

/// Source-level invariant: the confirmation path must go through the
/// registry-aware settlement entry point. Reverting to the registry-less
/// variant would silently honour only the built-in handlers.
#[test]
fn confirmation_settlement_always_uses_the_registry_entry_point() {
    let source = include_str!("../src/payment_confirmation_router.rs");

    assert!(
        source.contains("settle_owner_order_after_payment_success_with_registry("),
        "payment_confirmation_router must settle through the registry-aware entry point"
    );
    assert!(
        !source.contains("settle_owner_order_after_payment_success("),
        "payment_confirmation_router must not fall back to the registry-less settlement entry point"
    );
    assert!(
        source.contains("payment_notify_handler_registry"),
        "payment_confirmation_router must carry the injected notify handler registry"
    );
    // The confirmation path must never read a tenant/organization from the
    // request body: scope comes from the access-token context only.
    assert!(
        !source.contains("tenant_id: body.") && !source.contains("organization_id: body."),
        "confirmation scope must come from the access-token context, not the request body"
    );
}
