//! Standard contract tests for the refund notify orchestration building
//! block: verify → ingest → refund status dispatch (refunded / refund_failed
//! / accepted), with terminal-safe idempotency on the order side.

use std::sync::{Arc, Mutex};

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{
    process_refund_notify, OwnerOrderRefundStateFuture, OwnerOrderRefundStateOutcome,
    PaymentNotifyAttemptContext, PaymentNotifyEvent, PaymentNotifyVerifyFuture,
    PaymentNotifyVerifyPort, RefundNotifyContext, RefundNotifyIngestFuture,
    RefundNotifyIngestOutcome, RefundNotifyIngestPort, RefundNotifyStatePort,
    REFUND_NOTIFY_STATUS_ACCEPTED, REFUND_NOTIFY_STATUS_REFUNDED,
    REFUND_NOTIFY_STATUS_REFUND_FAILED,
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

fn refund_context(order_id: &str, status: &str) -> RefundNotifyContext {
    RefundNotifyContext {
        refund_id: format!("refund-{order_id}"),
        refund_no: format!("RF-{order_id}"),
        order_id: order_id.to_owned(),
        tenant_id: "tenant-1".to_owned(),
        organization_id: Some("org-1".to_owned()),
        status: status.to_owned(),
        amount: "100".to_owned(),
        business_type: sdkwork_order_service::REFUND_NOTIFY_BUSINESS_REFUND.to_owned(),
    }
}

fn refund_event(provider_code: &str) -> PaymentNotifyEvent {
    PaymentNotifyEvent {
        provider_code: provider_code.to_owned(),
        provider_event_id: Some("refund-evt-1".to_owned()),
        event_type: Some("REFUND.SUCCESS".to_owned()),
        out_trade_no: Some("trade-order-1".to_owned()),
        payment_status: None,
        payload: serde_json::json!({}),
        tenant_id: Some("tenant-1".to_owned()),
        organization_id: Some("org-1".to_owned()),
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

struct MockRefundIngestPort {
    outcome: RefundNotifyIngestOutcome,
}

impl RefundNotifyIngestPort for MockRefundIngestPort {
    fn ingest<'a>(&'a self, _event: PaymentNotifyEvent) -> RefundNotifyIngestFuture<'a> {
        let outcome = self.outcome.clone();
        Box::pin(async move { Ok(outcome) })
    }
}

#[derive(Default)]
struct MockRefundStatePort {
    calls: Mutex<u32>,
    statuses: Mutex<Vec<String>>,
    terminal: bool,
}

impl MockRefundStatePort {
    fn calls(&self) -> u32 {
        *self.calls.lock().expect("state lock")
    }

    fn statuses(&self) -> Vec<String> {
        self.statuses.lock().expect("state lock").clone()
    }
}

impl RefundNotifyStatePort for MockRefundStatePort {
    fn mark_owner_order_refund_status<'a>(
        &'a self,
        _attempt: &'a sdkwork_order_service::OrderPaymentSettlementAttempt,
        refund_status: &'a str,
    ) -> OwnerOrderRefundStateFuture<'a> {
        *self.calls.lock().expect("state lock") += 1;
        self.statuses
            .lock()
            .expect("state lock")
            .push(refund_status.to_owned());
        let terminal = self.terminal;
        let refund_status = refund_status.to_owned();
        Box::pin(async move {
            Ok(OwnerOrderRefundStateOutcome {
                refund_status,
                terminal_preserved: terminal,
            })
        })
    }
}

async fn run(
    verify: &MockNotifyVerifyPort,
    ingest: &MockRefundIngestPort,
    state: &MockRefundStatePort,
) -> Result<sdkwork_order_service::RefundNotifyProcessingOutcome, CommerceServiceError> {
    process_refund_notify(verify, ingest, state, "wechat_pay", &[], b"{}").await
}

#[tokio::test]
async fn refund_succeeded_marks_the_order_refunded() {
    let verify = MockNotifyVerifyPort {
        event: refund_event("wechat_pay"),
    };
    let ingest = MockRefundIngestPort {
        outcome: RefundNotifyIngestOutcome {
            webhook_event_id: "webhook-r1".to_owned(),
            replayed: false,
            refund: Some(refund_context("order-1", "succeeded")),
            payment_attempt: Some(notify_attempt("order-1")),
        },
    };
    let state = MockRefundStatePort::default();

    let outcome = run(&verify, &ingest, &state).await.expect("refund notify");

    assert_eq!(REFUND_NOTIFY_STATUS_REFUNDED, outcome.status);
    assert!(outcome.order_refund_marked);
    assert_eq!(1, state.calls());
    assert_eq!(vec!["refunded".to_owned()], state.statuses());
}

#[tokio::test]
async fn refund_failed_marks_the_order_refund_failed() {
    let verify = MockNotifyVerifyPort {
        event: refund_event("wechat_pay"),
    };
    let ingest = MockRefundIngestPort {
        outcome: RefundNotifyIngestOutcome {
            webhook_event_id: "webhook-r2".to_owned(),
            replayed: false,
            refund: Some(refund_context("order-2", "failed")),
            payment_attempt: Some(notify_attempt("order-2")),
        },
    };
    let state = MockRefundStatePort::default();

    let outcome = run(&verify, &ingest, &state).await.expect("refund notify");

    assert_eq!(REFUND_NOTIFY_STATUS_REFUND_FAILED, outcome.status);
    assert!(outcome.order_refund_marked);
    assert_eq!(vec!["refund_failed".to_owned()], state.statuses());
}

#[tokio::test]
async fn unmatched_refund_notify_is_accepted_without_order_effects() {
    let verify = MockNotifyVerifyPort {
        event: refund_event("stripe"),
    };
    let ingest = MockRefundIngestPort {
        outcome: RefundNotifyIngestOutcome {
            webhook_event_id: "webhook-r3".to_owned(),
            replayed: false,
            refund: None,
            payment_attempt: Some(notify_attempt("order-3")),
        },
    };
    let state = MockRefundStatePort::default();

    let outcome = run(&verify, &ingest, &state).await.expect("refund notify");

    assert_eq!(REFUND_NOTIFY_STATUS_ACCEPTED, outcome.status);
    assert!(!outcome.order_refund_marked);
    assert_eq!(0, state.calls());
}

#[tokio::test]
async fn refund_without_original_attempt_is_accepted_without_order_effects() {
    let verify = MockNotifyVerifyPort {
        event: refund_event("alipay"),
    };
    let ingest = MockRefundIngestPort {
        outcome: RefundNotifyIngestOutcome {
            webhook_event_id: "webhook-r4".to_owned(),
            replayed: false,
            refund: Some(refund_context("order-4", "succeeded")),
            payment_attempt: None,
        },
    };
    let state = MockRefundStatePort::default();

    let outcome = run(&verify, &ingest, &state).await.expect("refund notify");

    assert_eq!(REFUND_NOTIFY_STATUS_ACCEPTED, outcome.status);
    assert!(!outcome.order_refund_marked);
    assert_eq!(0, state.calls());
}

#[tokio::test]
async fn terminal_refunded_order_is_preserved_on_late_failure_notify() {
    let verify = MockNotifyVerifyPort {
        event: refund_event("wechat_pay"),
    };
    let ingest = MockRefundIngestPort {
        outcome: RefundNotifyIngestOutcome {
            webhook_event_id: "webhook-r5".to_owned(),
            replayed: true,
            refund: Some(refund_context("order-5", "failed")),
            payment_attempt: Some(notify_attempt("order-5")),
        },
    };
    let state = MockRefundStatePort {
        terminal: true,
        ..Default::default()
    };

    let outcome = run(&verify, &ingest, &state).await.expect("refund notify");

    // The state port reported the terminal preservation; the outcome
    // reflects that no order write happened.
    assert!(!outcome.order_refund_marked);
    assert_eq!(1, state.calls());
    assert_eq!(vec!["refund_failed".to_owned()], state.statuses());
}

#[tokio::test]
async fn missing_provider_event_id_derives_stable_fallback_identity() {
    let mut event = refund_event("alipay");
    event.provider_event_id = None;
    let verify = MockNotifyVerifyPort { event };
    let ingest = MockRefundIngestPort {
        outcome: RefundNotifyIngestOutcome {
            webhook_event_id: "webhook-r6".to_owned(),
            replayed: false,
            refund: None,
            payment_attempt: None,
        },
    };
    let state = MockRefundStatePort::default();

    // The derived identity flows into the ingest port; with no refund match
    // the outcome is accepted. The derivation itself is covered by the
    // payment notify tests; here we just verify the flow stays consistent.
    let outcome = run(&verify, &ingest, &state).await.expect("refund notify");
    assert_eq!(REFUND_NOTIFY_STATUS_ACCEPTED, outcome.status);
    assert_eq!(0, state.calls());
}
