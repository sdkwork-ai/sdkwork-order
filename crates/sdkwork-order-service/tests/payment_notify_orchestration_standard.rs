//! Standard contract tests for the payment notify orchestration building
//! block: verify → ingest → applied-status dispatch (settle / fail / accept).

use std::sync::{Arc, Mutex};

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    process_payment_notify, AccountPointsCreditFuture, AccountPointsCreditPort,
    AccountValueFulfillmentContext, AccountValueFulfillmentFuture, AccountValueFulfillmentStore,
    AccountValueLedgerCommand, AccountValueLedgerOutcome, AccountValueLedgerPort,
    ConfirmOwnerOrderPaymentOutcome, CouponRedemptionPort, CouponSubscriptionFulfillmentOutcome,
    CouponSubscriptionFulfillmentRequest, FulfillAccountValueOrderCommand,
    FulfillAccountValueOrderOutcome, FulfillPointsRechargeOrderCommand,
    FulfillPointsRechargeOrderOutcome, MarkPointsRechargePaymentSucceededCommand,
    MembershipPurchaseFulfillmentFuture, MembershipPurchaseFulfillmentOutcome,
    MembershipPurchaseFulfillmentPort, MembershipPurchaseFulfillmentRequest,
    MembershipQuotaRechargeFulfillmentOutcome, MembershipQuotaRechargeFulfillmentRequest,
    NoopCouponRedemptionPort, OrderPaymentSettlementAttempt, OwnerOrderPaymentConfirmationFuture,
    OwnerOrderPaymentConfirmationPort, OwnerOrderPaymentStateOutcome, OwnerOrderPaymentStatePort,
    OwnerOrderSettlementPorts, PaymentNotifyAttemptContext, PaymentNotifyEvent,
    PaymentNotifyIngestFuture, PaymentNotifyIngestOutcome, PaymentNotifyIngestPort,
    PaymentNotifyOrderContext, PaymentNotifyOrderContextFuture, PaymentNotifyOrderContextPort,
    PaymentNotifyVerifyFuture, PaymentNotifyVerifyPort, PointsRechargeCreditOutcome,
    PointsRechargeCreditRequest, PointsRechargeFulfillmentContext, PointsRechargeFulfillmentFuture,
    PointsRechargeFulfillmentStore, UnavailablePhysicalGoodsFulfillmentPort,
    PAYMENT_NOTIFY_STATUS_ACCEPTED, PAYMENT_NOTIFY_STATUS_ORDER_FAILURE_MARKED,
    PAYMENT_NOTIFY_STATUS_SETTLED, PAYMENT_NOTIFY_STATUS_UNMAPPED,
};

fn notify_attempt(order_id: &str) -> PaymentNotifyAttemptContext {
    PaymentNotifyAttemptContext {
        payment_attempt_id: format!("attempt-{order_id}"),
        out_trade_no: format!("trade-{order_id}"),
        tenant_id: "tenant-1".to_owned(),
        organization_id: Some("org-1".to_owned()),
        owner_user_id: "user-1".to_owned(),
        order_id: order_id.to_owned(),
    }
}

struct MockNotifyVerifyPort {
    event: PaymentNotifyEvent,
}

impl PaymentNotifyVerifyPort for MockNotifyVerifyPort {
    fn verify_and_normalize<'a>(
        &'a self,
        _provider_code: &'a str,
        _headers: &'a [(String, String)],
        _body: &'a [u8],
    ) -> PaymentNotifyVerifyFuture<'a> {
        let event = self.event.clone();
        Box::pin(async move { Ok(event) })
    }
}

struct MockNotifyIngestPort {
    outcome: PaymentNotifyIngestOutcome,
    received: Mutex<Vec<PaymentNotifyEvent>>,
}

impl MockNotifyIngestPort {
    fn received(&self) -> Vec<PaymentNotifyEvent> {
        self.received.lock().expect("ingest lock").clone()
    }
}

impl PaymentNotifyIngestPort for MockNotifyIngestPort {
    fn ingest<'a>(&'a self, event: PaymentNotifyEvent) -> PaymentNotifyIngestFuture<'a> {
        let outcome = self.outcome.clone();
        self.received.lock().expect("ingest lock").push(event);
        Box::pin(async move { Ok(outcome) })
    }
}

struct MockNotifyOrderContextPort {
    context: Option<PaymentNotifyOrderContext>,
}

impl PaymentNotifyOrderContextPort for MockNotifyOrderContextPort {
    fn load_order_settlement_context<'a>(
        &'a self,
        _tenant_id: &'a str,
        _organization_id: Option<&'a str>,
        _order_id: &'a str,
    ) -> PaymentNotifyOrderContextFuture<'a> {
        let context = self.context.clone();
        Box::pin(async move { Ok(context) })
    }
}

#[derive(Default)]
struct MockOwnerOrderPaymentStore {
    confirm_calls: Mutex<u32>,
}

impl MockOwnerOrderPaymentStore {
    fn confirm_calls(&self) -> u32 {
        *self.confirm_calls.lock().expect("confirm lock")
    }
}

impl OwnerOrderPaymentConfirmationPort for MockOwnerOrderPaymentStore {
    fn confirm_owner_order_payment<'a>(
        &'a self,
        attempt: &'a OrderPaymentSettlementAttempt,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, ConfirmOwnerOrderPaymentOutcome> {
        *self.confirm_calls.lock().expect("confirm lock") += 1;
        Box::pin(async move {
            Ok(ConfirmOwnerOrderPaymentOutcome {
                tenant_id: attempt.tenant_id.clone(),
                organization_id: attempt.organization_id.clone(),
                owner_user_id: attempt.owner_user_id.clone(),
                order_id: attempt.order_id.clone(),
                paid_at: "2026-07-08T00:00:00Z".to_owned(),
                replayed: false,
            })
        })
    }
}

#[derive(Default)]
struct MockOwnerOrderPaymentStateStore {
    succeeded_calls: Mutex<u32>,
    failed_calls: Mutex<u32>,
    failed_statuses: Mutex<Vec<String>>,
    /// When set, mark_owner_order_payment_succeeded reports a preserved
    /// terminal order (e.g. cancelled from an earlier failure notify).
    terminal_preserved: bool,
}

impl MockOwnerOrderPaymentStateStore {
    fn succeeded_calls(&self) -> u32 {
        *self.succeeded_calls.lock().expect("succeeded lock")
    }

    fn failed_calls(&self) -> u32 {
        *self.failed_calls.lock().expect("failed lock")
    }

    fn failed_statuses(&self) -> Vec<String> {
        self.failed_statuses.lock().expect("failed lock").clone()
    }
}

impl OwnerOrderPaymentStatePort for MockOwnerOrderPaymentStateStore {
    fn mark_owner_order_payment_succeeded<'a>(
        &'a self,
        _attempt: &'a OrderPaymentSettlementAttempt,
        _paid_at: &'a str,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, OwnerOrderPaymentStateOutcome> {
        *self.succeeded_calls.lock().expect("succeeded lock") += 1;
        let terminal_preserved = self.terminal_preserved;
        Box::pin(async move {
            Ok(OwnerOrderPaymentStateOutcome {
                order_status: if terminal_preserved {
                    "cancelled".to_owned()
                } else {
                    "paid".to_owned()
                },
                terminal_order_preserved: terminal_preserved,
            })
        })
    }

    fn mark_owner_order_payment_failed<'a>(
        &'a self,
        _attempt: &'a OrderPaymentSettlementAttempt,
        failure_status: &'a str,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, OwnerOrderPaymentStateOutcome> {
        *self.failed_calls.lock().expect("failed lock") += 1;
        self.failed_statuses
            .lock()
            .expect("failed lock")
            .push(failure_status.to_owned());
        Box::pin(async move {
            Ok(OwnerOrderPaymentStateOutcome {
                order_status: "cancelled".to_owned(),
                terminal_order_preserved: false,
            })
        })
    }
}

fn settlement_ports<'a>(
    payment_store: &'a MockOwnerOrderPaymentStore,
    order_state_store: &'a MockOwnerOrderPaymentStateStore,
) -> OwnerOrderSettlementPorts<'a> {
    OwnerOrderSettlementPorts {
        payment_store,
        order_state_store,
        recharge_store: &UnsupportedPointsRechargeStore,
        account_value_store: &UnsupportedAccountValueFulfillmentStore,
        credit_port: &UnsupportedAccountPointsCreditPort,
        account_value_ledger_port: &UnsupportedAccountValueLedgerPort,
        coupon_redemption_port: &NoopCouponRedemptionPort,
        membership_port: &UnsupportedMembershipPurchaseFulfillmentPort,
        physical_goods_port: &UnavailablePhysicalGoodsFulfillmentPort,
    }
}

fn success_event(provider_code: &str) -> PaymentNotifyEvent {
    PaymentNotifyEvent {
        provider_code: provider_code.to_owned(),
        provider_event_id: Some("evt-1".to_owned()),
        event_type: Some("payment.succeeded".to_owned()),
        out_trade_no: Some("trade-order-1".to_owned()),
        payment_status: Some("success".to_owned()),
        payload: serde_json::json!({}),
        tenant_id: Some("tenant-1".to_owned()),
        organization_id: Some("org-1".to_owned()),
    }
}

#[tokio::test]
async fn succeeded_notify_settles_the_order_through_the_building_block() {
    let verify = MockNotifyVerifyPort {
        event: success_event("alipay"),
    };
    let ingest = MockNotifyIngestPort {
        outcome: PaymentNotifyIngestOutcome {
            webhook_event_id: "webhook-1".to_owned(),
            replayed: false,
            applied_status: Some("succeeded".to_owned()),
            applied: true,
            attempt: Some(notify_attempt("order-1")),
        },
        received: Mutex::new(Vec::new()),
    };
    let context = MockNotifyOrderContextPort {
        context: Some(PaymentNotifyOrderContext {
            subject: "virtual_goods".to_owned(),
            membership_purchase: None,
        }),
    };
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::default());

    let outcome = process_payment_notify(
        &verify,
        &ingest,
        &context,
        settlement_ports(payment_store.as_ref(), order_state_store.as_ref()),
        sdkwork_order_service::default_payment_notify_handler_registry().as_ref(),
        "alipay",
        &[],
        b"{}",
    )
    .await
    .expect("notify processing");

    assert_eq!(PAYMENT_NOTIFY_STATUS_SETTLED, outcome.status);
    assert_eq!("webhook-1", outcome.webhook_event_id);
    let settlement = outcome.settlement.expect("settlement outcome");
    assert!(settlement.payment_confirmed);
    assert_eq!(
        "awaiting_external_fulfillment",
        settlement.fulfillment_status
    );
    assert_eq!(1, payment_store.confirm_calls());
    assert_eq!(1, order_state_store.succeeded_calls());
    assert_eq!(0, order_state_store.failed_calls());
}

#[tokio::test]
async fn failed_notify_marks_the_owner_order_failed_idempotently() {
    let verify = MockNotifyVerifyPort {
        event: success_event("wechat_pay"),
    };
    let ingest = MockNotifyIngestPort {
        outcome: PaymentNotifyIngestOutcome {
            webhook_event_id: "webhook-2".to_owned(),
            replayed: false,
            applied_status: Some("failed".to_owned()),
            applied: true,
            attempt: Some(notify_attempt("order-2")),
        },
        received: Mutex::new(Vec::new()),
    };
    let context = MockNotifyOrderContextPort {
        context: Some(PaymentNotifyOrderContext {
            subject: "product".to_owned(),
            membership_purchase: None,
        }),
    };
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::default());

    let outcome = process_payment_notify(
        &verify,
        &ingest,
        &context,
        settlement_ports(payment_store.as_ref(), order_state_store.as_ref()),
        sdkwork_order_service::default_payment_notify_handler_registry().as_ref(),
        "wechat_pay",
        &[],
        b"{}",
    )
    .await
    .expect("notify processing");

    assert_eq!(PAYMENT_NOTIFY_STATUS_ORDER_FAILURE_MARKED, outcome.status);
    assert!(outcome.order_failure_marked);
    assert_eq!(0, payment_store.confirm_calls());
    assert_eq!(1, order_state_store.failed_calls());
    assert_eq!(
        vec!["failed".to_owned()],
        order_state_store.failed_statuses()
    );
}

#[tokio::test]
async fn unmapped_notify_is_accepted_without_order_effects() {
    let verify = MockNotifyVerifyPort {
        event: success_event("stripe"),
    };
    let ingest = MockNotifyIngestPort {
        outcome: PaymentNotifyIngestOutcome {
            webhook_event_id: "webhook-3".to_owned(),
            replayed: false,
            applied_status: None,
            applied: false,
            attempt: None,
        },
        received: Mutex::new(Vec::new()),
    };
    let context = MockNotifyOrderContextPort { context: None };
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::default());

    let outcome = process_payment_notify(
        &verify,
        &ingest,
        &context,
        settlement_ports(payment_store.as_ref(), order_state_store.as_ref()),
        sdkwork_order_service::default_payment_notify_handler_registry().as_ref(),
        "stripe",
        &[],
        b"{}",
    )
    .await
    .expect("notify processing");

    assert_eq!(PAYMENT_NOTIFY_STATUS_UNMAPPED, outcome.status);
    assert_eq!(0, payment_store.confirm_calls());
    assert_eq!(0, order_state_store.succeeded_calls());
    assert_eq!(0, order_state_store.failed_calls());
}

#[tokio::test]
async fn missing_provider_event_id_derives_stable_fallback_identity() {
    let mut event = success_event("alipay");
    event.provider_event_id = None;
    let verify = MockNotifyVerifyPort { event };
    let ingest = MockNotifyIngestPort {
        outcome: PaymentNotifyIngestOutcome {
            webhook_event_id: "webhook-4".to_owned(),
            replayed: false,
            applied_status: None,
            applied: false,
            attempt: None,
        },
        received: Mutex::new(Vec::new()),
    };
    let context = MockNotifyOrderContextPort { context: None };
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::default());

    let _ = process_payment_notify(
        &verify,
        &ingest,
        &context,
        settlement_ports(payment_store.as_ref(), order_state_store.as_ref()),
        sdkwork_order_service::default_payment_notify_handler_registry().as_ref(),
        "alipay",
        &[],
        b"{}",
    )
    .await
    .expect("notify processing");

    let received = ingest.received();
    assert_eq!(1, received.len());
    let derived = received[0]
        .provider_event_id
        .as_deref()
        .expect("derived id");
    // Fallback identity = {provider}:{out_trade_no}:{event_type}:{payload hash}
    assert!(
        derived.starts_with("alipay:trade-order-1:payment"),
        "got {derived}"
    );
    assert_eq!(
        derived.len(),
        "alipay:trade-order-1:payment.succeeded:".len() + 16
    );
}

#[tokio::test]
async fn recovery_notify_after_failure_reaches_late_payment_path() {
    // Sequence: first notify FAILED (order cancelled), then a second SUCCEEDED
    // notify arrives (applied=true via webhook recovery). The orchestrator
    // must re-enter settlement; the order-side late-payment machinery keeps
    // the order cancelled and reports late_payment_requires_recovery.
    let verify = MockNotifyVerifyPort {
        event: success_event("alipay"),
    };
    let ingest = MockNotifyIngestPort {
        outcome: PaymentNotifyIngestOutcome {
            webhook_event_id: "webhook-recv1".to_owned(),
            replayed: false,
            applied_status: Some("succeeded".to_owned()),
            applied: true,
            attempt: Some(notify_attempt("order-recv1")),
        },
        received: Mutex::new(Vec::new()),
    };
    let context = MockNotifyOrderContextPort {
        context: Some(PaymentNotifyOrderContext {
            subject: "product".to_owned(),
            membership_purchase: None,
        }),
    };
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    // Order is already terminal-cancelled from the earlier failure notify.
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore {
        terminal_preserved: true,
        ..Default::default()
    });

    let outcome = process_payment_notify(
        &verify,
        &ingest,
        &context,
        settlement_ports(payment_store.as_ref(), order_state_store.as_ref()),
        sdkwork_order_service::default_payment_notify_handler_registry().as_ref(),
        "alipay",
        &[],
        b"{}",
    )
    .await
    .expect("notify processing");

    assert_eq!(PAYMENT_NOTIFY_STATUS_SETTLED, outcome.status);
    let settlement = outcome.settlement.expect("settlement");
    // Money collected after a terminal failure: fulfillment suppressed,
    // recovery path reported.
    assert_eq!(
        "late_payment_requires_recovery",
        settlement.fulfillment_status
    );
    assert_eq!(1, payment_store.confirm_calls());
    assert_eq!(1, order_state_store.succeeded_calls());
    assert_eq!(0, order_state_store.failed_calls());
}

#[tokio::test]
async fn terminal_conflict_ack_does_not_re_enter_settlement() {
    // A stale failed notification on an already-succeeded payment: ingest
    // reports applied=false (current status preserved) and the orchestrator
    // must ack WITHOUT re-running settlement.
    let verify = MockNotifyVerifyPort {
        event: success_event("alipay"),
    };
    let ingest = MockNotifyIngestPort {
        outcome: PaymentNotifyIngestOutcome {
            webhook_event_id: "webhook-tc1".to_owned(),
            replayed: false,
            applied_status: Some("succeeded".to_owned()),
            applied: false,
            attempt: Some(notify_attempt("order-tc1")),
        },
        received: Mutex::new(Vec::new()),
    };
    let context = MockNotifyOrderContextPort {
        context: Some(PaymentNotifyOrderContext {
            subject: "virtual_goods".to_owned(),
            membership_purchase: None,
        }),
    };
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::default());

    let outcome = process_payment_notify(
        &verify,
        &ingest,
        &context,
        settlement_ports(payment_store.as_ref(), order_state_store.as_ref()),
        sdkwork_order_service::default_payment_notify_handler_registry().as_ref(),
        "alipay",
        &[],
        b"{}",
    )
    .await
    .expect("notify processing");

    assert_eq!(PAYMENT_NOTIFY_STATUS_ACCEPTED, outcome.status);
    assert!(outcome.settlement.is_none());
    assert!(!outcome.order_failure_marked);
    assert_eq!(0, payment_store.confirm_calls());
    assert_eq!(0, order_state_store.succeeded_calls());
    assert_eq!(0, order_state_store.failed_calls());
}

#[tokio::test]
async fn redelivered_notify_replays_and_resettles_without_double_effects() {
    // At-least-once contract: a provider redelivery of the same event id is
    // ingested as a replay and re-enters settlement; every step is idempotent.
    let verify = MockNotifyVerifyPort {
        event: success_event("alipay"),
    };
    let ingest = MockNotifyIngestPort {
        outcome: PaymentNotifyIngestOutcome {
            webhook_event_id: "webhook-1".to_owned(),
            replayed: true,
            applied_status: Some("succeeded".to_owned()),
            applied: true,
            attempt: Some(notify_attempt("order-1")),
        },
        received: Mutex::new(Vec::new()),
    };
    let context = MockNotifyOrderContextPort {
        context: Some(PaymentNotifyOrderContext {
            subject: "virtual_goods".to_owned(),
            membership_purchase: None,
        }),
    };
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::default());

    let outcome = process_payment_notify(
        &verify,
        &ingest,
        &context,
        settlement_ports(payment_store.as_ref(), order_state_store.as_ref()),
        sdkwork_order_service::default_payment_notify_handler_registry().as_ref(),
        "alipay",
        &[],
        b"{}",
    )
    .await
    .expect("notify processing");

    assert!(outcome.replayed);
    assert_eq!(PAYMENT_NOTIFY_STATUS_SETTLED, outcome.status);
    assert_eq!(1, payment_store.confirm_calls());
    assert_eq!(1, order_state_store.succeeded_calls());
}

#[tokio::test]
async fn failed_notify_marks_the_owner_order_failed_through_the_pipeline() {
    // A payment-failure webhook must reach the order side: the ingest port
    // returns the attempt context for failure statuses too, so the order is
    // cancelled instead of acked without effect.
    let verify = MockNotifyVerifyPort {
        event: success_event("alipay"),
    };
    let ingest = MockNotifyIngestPort {
        outcome: PaymentNotifyIngestOutcome {
            webhook_event_id: "webhook-f1".to_owned(),
            replayed: false,
            applied_status: Some("failed".to_owned()),
            applied: true,
            attempt: Some(notify_attempt("order-f1")),
        },
        received: Mutex::new(Vec::new()),
    };
    let context = MockNotifyOrderContextPort {
        context: Some(PaymentNotifyOrderContext {
            subject: "product".to_owned(),
            membership_purchase: None,
        }),
    };
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::default());

    let outcome = process_payment_notify(
        &verify,
        &ingest,
        &context,
        settlement_ports(payment_store.as_ref(), order_state_store.as_ref()),
        sdkwork_order_service::default_payment_notify_handler_registry().as_ref(),
        "alipay",
        &[],
        b"{}",
    )
    .await
    .expect("notify processing");

    assert_eq!(PAYMENT_NOTIFY_STATUS_ORDER_FAILURE_MARKED, outcome.status);
    assert!(outcome.order_failure_marked);
    assert_eq!(0, payment_store.confirm_calls());
    assert_eq!(1, order_state_store.failed_calls());
    assert_eq!(
        vec!["failed".to_owned()],
        order_state_store.failed_statuses()
    );
}

#[test]
fn refund_event_type_detection_covers_provider_envelopes() {
    use sdkwork_order_service::is_refund_event_type;
    assert!(is_refund_event_type(Some("REFUND.SUCCESS")));
    assert!(is_refund_event_type(Some("REFUND.CLOSED")));
    assert!(is_refund_event_type(Some("charge.refunded")));
    assert!(is_refund_event_type(Some("refund")));
    assert!(!is_refund_event_type(Some("TRANSACTION.SUCCESS")));
    assert!(!is_refund_event_type(Some("payment.succeeded")));
    assert!(!is_refund_event_type(None));
    assert!(!is_refund_event_type(Some("")));
}

#[tokio::test]
async fn missing_order_context_fails_closed_with_not_found() {
    let verify = MockNotifyVerifyPort {
        event: success_event("alipay"),
    };
    let ingest = MockNotifyIngestPort {
        outcome: PaymentNotifyIngestOutcome {
            webhook_event_id: "webhook-5".to_owned(),
            replayed: false,
            applied_status: Some("succeeded".to_owned()),
            applied: true,
            attempt: Some(notify_attempt("order-5")),
        },
        received: Mutex::new(Vec::new()),
    };
    let context = MockNotifyOrderContextPort { context: None };
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::default());

    let error = process_payment_notify(
        &verify,
        &ingest,
        &context,
        settlement_ports(payment_store.as_ref(), order_state_store.as_ref()),
        sdkwork_order_service::default_payment_notify_handler_registry().as_ref(),
        "alipay",
        &[],
        b"{}",
    )
    .await
    .expect_err("missing order context must fail closed");

    assert!(format!("{error:?}").contains("order was not found"));
    assert_eq!(0, payment_store.confirm_calls());
}

// ---------------------------------------------------------------------------
// Unsupported port stubs: the orchestration tests never dispatch fulfillment
// that touches these stores, so they must fail loudly if accidentally called.
// ---------------------------------------------------------------------------

struct UnsupportedPointsRechargeStore;

impl PointsRechargeFulfillmentStore for UnsupportedPointsRechargeStore {
    fn load_points_recharge_fulfillment_context<'a>(
        &'a self,
        _command: &'a FulfillPointsRechargeOrderCommand,
    ) -> PointsRechargeFulfillmentFuture<'a, Option<PointsRechargeFulfillmentContext>> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "points recharge store should not be called in orchestration tests",
            ))
        })
    }

    fn reserve_points_recharge_fulfillment<'a>(
        &'a self,
        _command: &'a FulfillPointsRechargeOrderCommand,
        _context: &'a PointsRechargeFulfillmentContext,
    ) -> PointsRechargeFulfillmentFuture<'a, ()> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "points recharge reservation should not be called in orchestration tests",
            ))
        })
    }

    fn release_points_recharge_fulfillment_reservation<'a>(
        &'a self,
        _command: &'a FulfillPointsRechargeOrderCommand,
        _context: &'a PointsRechargeFulfillmentContext,
    ) -> PointsRechargeFulfillmentFuture<'a, ()> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "points recharge release should not be called in orchestration tests",
            ))
        })
    }

    fn commit_points_recharge_fulfillment<'a>(
        &'a self,
        _command: FulfillPointsRechargeOrderCommand,
        _context: &'a PointsRechargeFulfillmentContext,
    ) -> PointsRechargeFulfillmentFuture<'a, FulfillPointsRechargeOrderOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "points recharge commit should not be called in orchestration tests",
            ))
        })
    }

    fn rollback_points_recharge_fulfillment<'a>(
        &'a self,
        _command: &'a FulfillPointsRechargeOrderCommand,
        _context: &'a PointsRechargeFulfillmentContext,
    ) -> PointsRechargeFulfillmentFuture<'a, ()> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "points recharge rollback should not be called in orchestration tests",
            ))
        })
    }

    fn mark_points_recharge_payment_succeeded<'a>(
        &'a self,
        _command: MarkPointsRechargePaymentSucceededCommand,
    ) -> PointsRechargeFulfillmentFuture<'a, ()> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "points recharge payment success should not be called in orchestration tests",
            ))
        })
    }
}

struct UnsupportedAccountPointsCreditPort;

impl AccountPointsCreditPort for UnsupportedAccountPointsCreditPort {
    fn credit_points_recharge<'a>(
        &'a self,
        _request: PointsRechargeCreditRequest,
    ) -> AccountPointsCreditFuture<'a, PointsRechargeCreditOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "points credit port should not be called in orchestration tests",
            ))
        })
    }

    fn reverse_points_recharge_credit<'a>(
        &'a self,
        _request: PointsRechargeCreditRequest,
    ) -> AccountPointsCreditFuture<'a, PointsRechargeCreditOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "points reverse port should not be called in orchestration tests",
            ))
        })
    }
}

struct UnsupportedAccountValueFulfillmentStore;

impl AccountValueFulfillmentStore for UnsupportedAccountValueFulfillmentStore {
    fn load_account_value_fulfillment_context<'a>(
        &'a self,
        _command: &'a FulfillAccountValueOrderCommand,
    ) -> AccountValueFulfillmentFuture<'a, Option<AccountValueFulfillmentContext>> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "account value store should not be called in orchestration tests",
            ))
        })
    }

    fn reserve_account_value_fulfillment<'a>(
        &'a self,
        _command: &'a FulfillAccountValueOrderCommand,
        _context: &'a AccountValueFulfillmentContext,
    ) -> AccountValueFulfillmentFuture<'a, ()> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "account value reservation should not be called in orchestration tests",
            ))
        })
    }

    fn release_account_value_fulfillment_reservation<'a>(
        &'a self,
        _command: &'a FulfillAccountValueOrderCommand,
        _context: &'a AccountValueFulfillmentContext,
    ) -> AccountValueFulfillmentFuture<'a, ()> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "account value release should not be called in orchestration tests",
            ))
        })
    }

    fn commit_account_value_fulfillment<'a>(
        &'a self,
        _command: FulfillAccountValueOrderCommand,
        _context: &'a AccountValueFulfillmentContext,
    ) -> AccountValueFulfillmentFuture<'a, FulfillAccountValueOrderOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "account value commit should not be called in orchestration tests",
            ))
        })
    }
}

struct UnsupportedAccountValueLedgerPort;

impl AccountValueLedgerPort for UnsupportedAccountValueLedgerPort {
    fn apply_account_value_ledger_command<'a>(
        &'a self,
        _command: AccountValueLedgerCommand,
    ) -> AccountValueFulfillmentFuture<'a, AccountValueLedgerOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "account value ledger should not be called in orchestration tests",
            ))
        })
    }
}

struct UnsupportedMembershipPurchaseFulfillmentPort;

impl MembershipPurchaseFulfillmentPort for UnsupportedMembershipPurchaseFulfillmentPort {
    fn fulfill_membership_purchase<'a>(
        &'a self,
        _request: MembershipPurchaseFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, MembershipPurchaseFulfillmentOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "membership fulfillment should not be called in orchestration tests",
            ))
        })
    }

    fn fulfill_coupon_subscription<'a>(
        &'a self,
        _request: CouponSubscriptionFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, CouponSubscriptionFulfillmentOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "membership coupon subscription should not be called in orchestration tests",
            ))
        })
    }

    fn fulfill_membership_quota_recharge<'a>(
        &'a self,
        _request: MembershipQuotaRechargeFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, MembershipQuotaRechargeFulfillmentOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "membership quota recharge should not be called in orchestration tests",
            ))
        })
    }
}
