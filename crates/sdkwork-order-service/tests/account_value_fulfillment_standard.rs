use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use sdkwork_contract_service::{CommerceLedgerBusinessType, CommerceMoney, CommerceServiceError};
use sdkwork_order_service::{
    default_fulfill_account_value_order_command, fulfill_account_value_order,
    redeem_coupon_and_fulfill_account_value_order, settle_owner_order_after_payment_success,
    AccountPointsCreditFuture, AccountPointsCreditPort, AccountValueAssetCode,
    AccountValueFulfillmentContext, AccountValueFulfillmentFuture, AccountValueFulfillmentStore,
    AccountValueFuture, AccountValueLedgerCommand, AccountValueLedgerOutcome,
    AccountValueLedgerPort, AccountValueOrderSubject, ConfirmOwnerOrderPaymentOutcome,
    CouponRedemptionBenefit, CouponRedemptionOutcome, CouponRedemptionPort,
    CouponRedemptionRequest, CouponSubscriptionFulfillmentOutcome,
    CouponSubscriptionFulfillmentRequest, FulfillAccountValueOrderCommand,
    FulfillAccountValueOrderOutcome, FulfillPaidPhysicalOrderRequest,
    FulfillPointsRechargeOrderCommand, FulfillPointsRechargeOrderOutcome,
    MarkPointsRechargePaymentSucceededCommand, MembershipPurchaseFulfillmentFuture,
    MembershipPurchaseFulfillmentOutcome, MembershipPurchaseFulfillmentPort,
    MembershipPurchaseFulfillmentRequest, MembershipPurchaseSettlementSnapshot,
    MembershipQuotaRechargeFulfillmentOutcome, MembershipQuotaRechargeFulfillmentRequest,
    NoopAppTemplatePurchaseFulfillmentPort, NoopCouponRedemptionPort,
    OrderPaymentSettlementAttempt, OwnerOrderPaymentConfirmationFuture,
    OwnerOrderPaymentConfirmationPort, OwnerOrderPaymentStateOutcome, OwnerOrderPaymentStatePort,
    OwnerOrderSettlementPorts, PhysicalGoodsFulfillmentOutcome, PhysicalGoodsFulfillmentPort,
    PhysicalGoodsFuture, PointsRechargeCreditOutcome, PointsRechargeCreditRequest,
    PointsRechargeFulfillmentContext, PointsRechargeFulfillmentFuture,
    PointsRechargeFulfillmentStore, UnavailablePhysicalGoodsFulfillmentPort,
};

#[tokio::test]
async fn fulfill_token_bank_recharge_credits_account_value_ledger_then_commits_order() {
    let store = Arc::new(MockAccountValueFulfillmentStore::default());
    let ledger = Arc::new(MockAccountValueLedgerPort::default());

    store.seed_context(AccountValueFulfillmentContext {
        order_id: "order-token-1".to_owned(),
        order_no: "ORD-TOKEN-1".to_owned(),
        subject: AccountValueOrderSubject::TokenBankRecharge,
        target_asset: AccountValueAssetCode::TokenBank,
        order_status: "pending_payment".to_owned(),
        fulfillment_status: "unfulfilled".to_owned(),
        payment_status: "success".to_owned(),
        payment_attempt_status: "succeeded".to_owned(),
        grant_amount: CommerceMoney::new("30000").expect("grant"),
        asset_unit_code: "TOKEN_BANK".to_owned(),
        coupon_code: None,
        coupon_benefit: None,
    });

    let command = default_fulfill_account_value_order_command(
        AccountValueOrderSubject::TokenBankRecharge,
        "tenant-1",
        Some("org-1"),
        "user-1",
        "order-token-1",
        "req-token-fulfill-1",
    )
    .expect("command");

    let outcome = fulfill_account_value_order(store.as_ref(), ledger.as_ref(), command)
        .await
        .expect("fulfillment");

    assert!(outcome.accepted);
    assert!(!outcome.replayed);
    assert_eq!(outcome.target_asset, AccountValueAssetCode::TokenBank);
    assert_eq!(outcome.amount.as_str(), "30000");
    assert_eq!(store.reserve_calls(), 1);
    assert_eq!(store.commit_calls(), 1);

    let calls = ledger.commands();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].asset, AccountValueAssetCode::TokenBank);
    assert_eq!(calls[0].amount.as_str(), "30000");
    assert_eq!(calls[0].currency_code, "TOKEN_BANK");
    assert_eq!(
        calls[0].business_type,
        CommerceLedgerBusinessType::TOKEN_BANK_PURCHASE_CREDIT
    );
    assert_eq!(calls[0].resource_id, "order-token-1");
    assert_eq!(
        calls[0].idempotency_key,
        "token-bank-recharge:fulfill:order-token-1"
    );
}

#[tokio::test]
async fn fulfill_account_value_order_replays_without_duplicate_ledger_credit() {
    let store = Arc::new(MockAccountValueFulfillmentStore::default());
    let ledger = Arc::new(MockAccountValueLedgerPort::default());

    store.seed_context(AccountValueFulfillmentContext {
        order_id: "order-token-replayed".to_owned(),
        order_no: "ORD-TOKEN-REPLAYED".to_owned(),
        subject: AccountValueOrderSubject::TokenBankPlanPurchase,
        target_asset: AccountValueAssetCode::TokenBank,
        order_status: "fulfilled".to_owned(),
        fulfillment_status: "fulfilled".to_owned(),
        payment_status: "success".to_owned(),
        payment_attempt_status: "succeeded".to_owned(),
        grant_amount: CommerceMoney::new("50000").expect("grant"),
        asset_unit_code: "TOKEN_BANK".to_owned(),
        coupon_code: None,
        coupon_benefit: None,
    });

    let command = default_fulfill_account_value_order_command(
        AccountValueOrderSubject::TokenBankPlanPurchase,
        "tenant-1",
        None,
        "user-1",
        "order-token-replayed",
        "req-token-fulfill-2",
    )
    .expect("command");

    let outcome = fulfill_account_value_order(store.as_ref(), ledger.as_ref(), command)
        .await
        .expect("fulfillment replay");

    assert!(outcome.replayed);
    assert_eq!(ledger.commands().len(), 0);
    assert_eq!(store.reserve_calls(), 0);
    assert_eq!(store.commit_calls(), 0);
}

#[tokio::test]
async fn settlement_payment_success_dispatches_token_bank_recharge_to_account_value_ledger() {
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::default());
    let account_value_store = Arc::new(MockAccountValueFulfillmentStore::default());
    let account_value_ledger = Arc::new(MockAccountValueLedgerPort::default());
    let points_store = Arc::new(UnsupportedPointsRechargeStore);
    let points_port = Arc::new(UnsupportedAccountPointsCreditPort);
    let membership_port = Arc::new(UnsupportedMembershipPurchaseFulfillmentPort);
    let coupon_port = Arc::new(NoopCouponRedemptionPort);

    account_value_store.seed_context(AccountValueFulfillmentContext {
        order_id: "order-token-settle".to_owned(),
        order_no: "ORD-TOKEN-SETTLE".to_owned(),
        subject: AccountValueOrderSubject::TokenBankRecharge,
        target_asset: AccountValueAssetCode::TokenBank,
        order_status: "pending_payment".to_owned(),
        fulfillment_status: "unfulfilled".to_owned(),
        payment_status: "success".to_owned(),
        payment_attempt_status: "succeeded".to_owned(),
        grant_amount: CommerceMoney::new("120000").expect("grant"),
        asset_unit_code: "TOKEN_BANK".to_owned(),
        coupon_code: None,
        coupon_benefit: None,
    });

    let attempt = OrderPaymentSettlementAttempt {
        tenant_id: "tenant-1".to_owned(),
        organization_id: Some("org-1".to_owned()),
        owner_user_id: "user-1".to_owned(),
        order_id: "order-token-settle".to_owned(),
        payment_attempt_id: None,
        out_trade_no: None,
    };

    let outcome = settle_owner_order_after_payment_success(
        OwnerOrderSettlementPorts {
            payment_store: payment_store.as_ref(),
            order_state_store: order_state_store.as_ref(),
            recharge_store: points_store.as_ref(),
            account_value_store: account_value_store.as_ref(),
            credit_port: points_port.as_ref(),
            account_value_ledger_port: account_value_ledger.as_ref(),
            coupon_redemption_port: coupon_port.as_ref(),
            membership_port: membership_port.as_ref(),
            app_template_port: &NoopAppTemplatePurchaseFulfillmentPort,
            physical_goods_port: &UnavailablePhysicalGoodsFulfillmentPort,
        },
        &attempt,
        Some("token_bank_recharge"),
        None,
        "req-token-settle-1",
    )
    .await
    .expect("token bank settlement");

    assert!(outcome.payment_confirmed);
    assert!(outcome.fulfillment_accepted);
    assert!(!outcome.fulfillment_replayed);
    assert_eq!(outcome.points_credited, 0);
    assert_eq!(outcome.fulfillment_status, "fulfilled");
    assert_eq!(payment_store.confirm_calls(), 1);
    assert_eq!(order_state_store.mark_calls(), 1);
    assert_eq!(account_value_store.reserve_calls(), 1);
    assert_eq!(account_value_store.commit_calls(), 1);

    let commands = account_value_ledger.commands();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].asset, AccountValueAssetCode::TokenBank);
    assert_eq!(commands[0].amount.as_str(), "120000");
    assert_eq!(commands[0].currency_code, "TOKEN_BANK");
}

#[tokio::test]
async fn late_payment_records_payment_but_suppresses_account_value_fulfillment() {
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::terminal("cancelled"));
    let account_value_store = Arc::new(MockAccountValueFulfillmentStore::default());
    let account_value_ledger = Arc::new(MockAccountValueLedgerPort::default());
    let points_store = Arc::new(UnsupportedPointsRechargeStore);
    let points_port = Arc::new(UnsupportedAccountPointsCreditPort);
    let membership_port = Arc::new(UnsupportedMembershipPurchaseFulfillmentPort);
    let coupon_port = Arc::new(NoopCouponRedemptionPort);
    let attempt = OrderPaymentSettlementAttempt {
        tenant_id: "tenant-1".to_owned(),
        organization_id: Some("org-1".to_owned()),
        owner_user_id: "user-1".to_owned(),
        order_id: "order-late-payment".to_owned(),
        payment_attempt_id: None,
        out_trade_no: None,
    };

    let outcome = settle_owner_order_after_payment_success(
        OwnerOrderSettlementPorts {
            payment_store: payment_store.as_ref(),
            order_state_store: order_state_store.as_ref(),
            recharge_store: points_store.as_ref(),
            account_value_store: account_value_store.as_ref(),
            credit_port: points_port.as_ref(),
            account_value_ledger_port: account_value_ledger.as_ref(),
            coupon_redemption_port: coupon_port.as_ref(),
            membership_port: membership_port.as_ref(),
            app_template_port: &NoopAppTemplatePurchaseFulfillmentPort,
            physical_goods_port: &UnavailablePhysicalGoodsFulfillmentPort,
        },
        &attempt,
        Some("token_bank_recharge"),
        None,
        "req-late-payment-1",
    )
    .await
    .expect("late payment settlement");

    assert!(outcome.payment_confirmed);
    assert!(!outcome.fulfillment_accepted);
    assert!(!outcome.fulfillment_replayed);
    assert_eq!(outcome.fulfillment_status, "late_payment_requires_recovery");
    assert_eq!(payment_store.confirm_calls(), 1);
    assert_eq!(order_state_store.mark_calls(), 1);
    assert_eq!(account_value_store.reserve_calls(), 0);
    assert_eq!(account_value_store.commit_calls(), 0);
    assert!(account_value_ledger.commands().is_empty());
}

#[tokio::test]
async fn ordinary_product_settlement_fails_closed_without_inventory_fulfillment() {
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::default());
    let account_value_store = Arc::new(MockAccountValueFulfillmentStore::default());
    let points_store = Arc::new(UnsupportedPointsRechargeStore);
    let points_port = Arc::new(UnsupportedAccountPointsCreditPort);
    let account_value_ledger = Arc::new(MockAccountValueLedgerPort::default());
    let membership_port = Arc::new(UnsupportedMembershipPurchaseFulfillmentPort);
    let coupon_port = Arc::new(NoopCouponRedemptionPort);
    let attempt = OrderPaymentSettlementAttempt {
        tenant_id: "tenant-1".to_owned(),
        organization_id: Some("org-1".to_owned()),
        owner_user_id: "user-1".to_owned(),
        order_id: "order-product-settle".to_owned(),
        payment_attempt_id: None,
        out_trade_no: None,
    };

    let outcome = settle_owner_order_after_payment_success(
        OwnerOrderSettlementPorts {
            payment_store: payment_store.as_ref(),
            order_state_store: order_state_store.as_ref(),
            recharge_store: points_store.as_ref(),
            account_value_store: account_value_store.as_ref(),
            credit_port: points_port.as_ref(),
            account_value_ledger_port: account_value_ledger.as_ref(),
            coupon_redemption_port: coupon_port.as_ref(),
            membership_port: membership_port.as_ref(),
            app_template_port: &NoopAppTemplatePurchaseFulfillmentPort,
            physical_goods_port: &UnavailablePhysicalGoodsFulfillmentPort,
        },
        &attempt,
        Some("physical_shipment"),
        None,
        "req-product-settle-1",
    )
    .await;

    assert!(outcome.is_err());
    assert_eq!(payment_store.confirm_calls(), 1);
    assert_eq!(order_state_store.mark_calls(), 1);
    assert!(account_value_ledger.commands().is_empty());
}

#[tokio::test]
async fn physical_product_settlement_consumes_inventory_and_opens_shipment_once() {
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::default());
    let account_value_store = Arc::new(MockAccountValueFulfillmentStore::default());
    let points_store = Arc::new(UnsupportedPointsRechargeStore);
    let points_port = Arc::new(UnsupportedAccountPointsCreditPort);
    let account_value_ledger = Arc::new(MockAccountValueLedgerPort::default());
    let membership_port = Arc::new(UnsupportedMembershipPurchaseFulfillmentPort);
    let coupon_port = Arc::new(NoopCouponRedemptionPort);
    let physical_port = Arc::new(CapturingPhysicalGoodsFulfillmentPort::default());
    let attempt = OrderPaymentSettlementAttempt {
        tenant_id: "tenant-1".to_owned(),
        organization_id: Some("buyer-org".to_owned()),
        owner_user_id: "user-1".to_owned(),
        order_id: "order-physical-1".to_owned(),
        payment_attempt_id: None,
        out_trade_no: None,
    };

    let outcome = settle_owner_order_after_payment_success(
        OwnerOrderSettlementPorts {
            payment_store: payment_store.as_ref(),
            order_state_store: order_state_store.as_ref(),
            recharge_store: points_store.as_ref(),
            account_value_store: account_value_store.as_ref(),
            credit_port: points_port.as_ref(),
            account_value_ledger_port: account_value_ledger.as_ref(),
            coupon_redemption_port: coupon_port.as_ref(),
            membership_port: membership_port.as_ref(),
            app_template_port: &NoopAppTemplatePurchaseFulfillmentPort,
            physical_goods_port: physical_port.as_ref(),
        },
        &attempt,
        Some("physical_shipment"),
        None,
        "request-physical-1",
    )
    .await
    .expect("physical fulfillment");

    assert!(outcome.fulfillment_accepted);
    assert_eq!(outcome.fulfillment_status, "awaiting_shipment");
    let requests = physical_port.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].order_id, "order-physical-1");
    assert_eq!(
        requests[0].idempotency_key,
        "physical-goods:fulfill:order-physical-1"
    );
}

#[tokio::test]
async fn membership_settlement_forwards_the_paid_order_snapshot_exactly_once() {
    let payment_store = Arc::new(MockOwnerOrderPaymentStore::default());
    let order_state_store = Arc::new(MockOwnerOrderPaymentStateStore::default());
    let account_value_store = Arc::new(MockAccountValueFulfillmentStore::default());
    let points_store = Arc::new(UnsupportedPointsRechargeStore);
    let points_port = Arc::new(UnsupportedAccountPointsCreditPort);
    let account_value_ledger = Arc::new(MockAccountValueLedgerPort::default());
    let membership_port = Arc::new(CapturingMembershipPurchaseFulfillmentPort::default());
    let coupon_port = Arc::new(NoopCouponRedemptionPort);
    let attempt = OrderPaymentSettlementAttempt {
        tenant_id: "tenant-1".to_owned(),
        organization_id: Some("0".to_owned()),
        owner_user_id: "user-1".to_owned(),
        order_id: "order-membership-settle".to_owned(),
        payment_attempt_id: None,
        out_trade_no: None,
    };
    let snapshot = MembershipPurchaseSettlementSnapshot {
        action: "upgrade".to_owned(),
        order_no: "MB-UPGRADE-1".to_owned(),
        package_id: 202,
        grant_quantity: None,
    };

    let outcome = settle_owner_order_after_payment_success(
        OwnerOrderSettlementPorts {
            payment_store: payment_store.as_ref(),
            order_state_store: order_state_store.as_ref(),
            recharge_store: points_store.as_ref(),
            account_value_store: account_value_store.as_ref(),
            credit_port: points_port.as_ref(),
            account_value_ledger_port: account_value_ledger.as_ref(),
            coupon_redemption_port: coupon_port.as_ref(),
            membership_port: membership_port.as_ref(),
            app_template_port: &NoopAppTemplatePurchaseFulfillmentPort,
            physical_goods_port: &UnavailablePhysicalGoodsFulfillmentPort,
        },
        &attempt,
        Some("membership"),
        Some(&snapshot),
        "request-membership-settle",
    )
    .await
    .expect("membership settlement");

    assert!(outcome.fulfillment_accepted);
    assert_eq!(outcome.fulfillment_status, "active");
    let requests = membership_port.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].action, "upgrade");
    assert_eq!(requests[0].order_no, "MB-UPGRADE-1");
    assert_eq!(requests[0].package_id, 202);
    assert_eq!(requests[0].paid_at, "2026-07-08T00:00:00Z");
    assert_eq!(
        requests[0].idempotency_key,
        "membership-purchase:fulfill:order-membership-settle"
    );
}

#[tokio::test]
async fn coupon_recharge_consumes_coupon_before_crediting_token_bank() {
    let store = MockAccountValueFulfillmentStore::default();
    store.seed_context(coupon_context("50"));
    let coupon = MockCouponRedemptionPort::new("50");
    let ledger = MockAccountValueLedgerPort::default();
    let command = default_fulfill_account_value_order_command(
        AccountValueOrderSubject::CouponRecharge,
        "tenant-1",
        Some("org-1"),
        "user-1",
        "order-coupon-1",
        "request-coupon-1",
    )
    .expect("coupon fulfillment command");

    let outcome = redeem_coupon_and_fulfill_account_value_order(&store, &coupon, &ledger, command)
        .await
        .expect("coupon recharge fulfillment");

    assert!(outcome.accepted);
    assert_eq!(1, coupon.requests().len());
    assert_eq!("WELCOME", coupon.requests()[0].coupon_code);
    assert_eq!("order-coupon-1", coupon.requests()[0].request_no);
    assert_eq!(1, ledger.commands().len());
    assert_eq!("50", ledger.commands()[0].amount.as_str());
}

#[tokio::test]
async fn coupon_recharge_fails_closed_when_promotion_benefit_changes() {
    let store = MockAccountValueFulfillmentStore::default();
    store.seed_context(coupon_context("50"));
    let coupon = MockCouponRedemptionPort::new("49");
    let ledger = MockAccountValueLedgerPort::default();
    let command = default_fulfill_account_value_order_command(
        AccountValueOrderSubject::CouponRecharge,
        "tenant-1",
        Some("org-1"),
        "user-1",
        "order-coupon-1",
        "request-coupon-1",
    )
    .expect("coupon fulfillment command");

    let error = redeem_coupon_and_fulfill_account_value_order(&store, &coupon, &ledger, command)
        .await
        .expect_err("benefit mismatch must fail");

    assert_eq!("conflict", error.code());
    assert!(ledger.commands().is_empty());
}

fn coupon_context(grant_amount: &str) -> AccountValueFulfillmentContext {
    AccountValueFulfillmentContext {
        order_id: "order-coupon-1".to_owned(),
        order_no: "CP1001".to_owned(),
        subject: AccountValueOrderSubject::CouponRecharge,
        target_asset: AccountValueAssetCode::TokenBank,
        order_status: "paid".to_owned(),
        fulfillment_status: "unfulfilled".to_owned(),
        payment_status: "succeeded".to_owned(),
        payment_attempt_status: "succeeded".to_owned(),
        grant_amount: CommerceMoney::new(grant_amount).expect("coupon grant"),
        asset_unit_code: "TOKEN_BANK".to_owned(),
        coupon_code: Some("WELCOME".to_owned()),
        coupon_benefit: Some(CouponRedemptionBenefit::TokenBankCredit {
            grant_amount: CommerceMoney::new(grant_amount).expect("coupon benefit grant"),
        }),
    }
}

struct MockCouponRedemptionPort {
    grant_amount: CommerceMoney,
    requests: Mutex<Vec<CouponRedemptionRequest>>,
}

impl MockCouponRedemptionPort {
    fn new(grant_amount: &str) -> Self {
        Self {
            grant_amount: CommerceMoney::new(grant_amount).expect("coupon grant"),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn requests(&self) -> Vec<CouponRedemptionRequest> {
        self.requests.lock().expect("coupon requests lock").clone()
    }
}

impl CouponRedemptionPort for MockCouponRedemptionPort {
    fn preview_coupon<'a>(
        &'a self,
        request: CouponRedemptionRequest,
    ) -> AccountValueFuture<'a, CouponRedemptionOutcome> {
        self.redeem_coupon(request)
    }

    fn redeem_coupon<'a>(
        &'a self,
        request: CouponRedemptionRequest,
    ) -> AccountValueFuture<'a, CouponRedemptionOutcome> {
        self.requests
            .lock()
            .expect("coupon requests lock")
            .push(request);
        let grant_amount = self.grant_amount.clone();
        Box::pin(async move {
            Ok(CouponRedemptionOutcome {
                accepted: true,
                replayed: false,
                benefit: CouponRedemptionBenefit::TokenBankCredit { grant_amount },
            })
        })
    }
}

#[derive(Default)]
struct MockAccountValueFulfillmentStore {
    contexts: Mutex<HashMap<String, AccountValueFulfillmentContext>>,
    reserve_calls: Mutex<u32>,
    commit_calls: Mutex<u32>,
    release_calls: Mutex<u32>,
}

impl MockAccountValueFulfillmentStore {
    fn seed_context(&self, context: AccountValueFulfillmentContext) {
        self.contexts
            .lock()
            .expect("contexts lock")
            .insert(context.order_id.clone(), context);
    }

    fn reserve_calls(&self) -> u32 {
        *self.reserve_calls.lock().expect("reserve lock")
    }

    fn commit_calls(&self) -> u32 {
        *self.commit_calls.lock().expect("commit lock")
    }
}

#[derive(Default)]
struct MockOwnerOrderPaymentStore {
    confirm_calls: Mutex<u32>,
}

#[derive(Default)]
struct MockOwnerOrderPaymentStateStore {
    mark_calls: Mutex<u32>,
    order_status: Option<String>,
}

impl MockOwnerOrderPaymentStateStore {
    fn terminal(order_status: &str) -> Self {
        Self {
            mark_calls: Mutex::new(0),
            order_status: Some(order_status.to_owned()),
        }
    }

    fn mark_calls(&self) -> u32 {
        *self.mark_calls.lock().expect("mark lock")
    }
}

impl OwnerOrderPaymentStatePort for MockOwnerOrderPaymentStateStore {
    fn mark_owner_order_payment_succeeded<'a>(
        &'a self,
        _attempt: &'a OrderPaymentSettlementAttempt,
        _paid_at: &'a str,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, OwnerOrderPaymentStateOutcome> {
        *self.mark_calls.lock().expect("mark lock") += 1;
        let order_status = self
            .order_status
            .clone()
            .unwrap_or_else(|| "paid".to_owned());
        let terminal_order_preserved = self.order_status.is_some();
        Box::pin(async move {
            Ok(OwnerOrderPaymentStateOutcome {
                order_status,
                terminal_order_preserved,
            })
        })
    }
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

struct UnsupportedPointsRechargeStore;

impl PointsRechargeFulfillmentStore for UnsupportedPointsRechargeStore {
    fn load_points_recharge_fulfillment_context<'a>(
        &'a self,
        _command: &'a FulfillPointsRechargeOrderCommand,
    ) -> PointsRechargeFulfillmentFuture<'a, Option<PointsRechargeFulfillmentContext>> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "points recharge store should not be called for token_bank_recharge",
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
                "points recharge reservation should not be called for token_bank_recharge",
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
                "points recharge release should not be called for token_bank_recharge",
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
                "points recharge commit should not be called for token_bank_recharge",
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
                "points recharge rollback should not be called for token_bank_recharge",
            ))
        })
    }

    fn mark_points_recharge_payment_succeeded<'a>(
        &'a self,
        _command: MarkPointsRechargePaymentSucceededCommand,
    ) -> PointsRechargeFulfillmentFuture<'a, ()> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "points recharge payment success should not be called for token_bank_recharge",
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
                "points credit port should not be called for token_bank_recharge",
            ))
        })
    }

    fn reverse_points_recharge_credit<'a>(
        &'a self,
        _request: PointsRechargeCreditRequest,
    ) -> AccountPointsCreditFuture<'a, PointsRechargeCreditOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "points reverse port should not be called for token_bank_recharge",
            ))
        })
    }
}

struct UnsupportedMembershipPurchaseFulfillmentPort;

#[derive(Default)]
struct CapturingPhysicalGoodsFulfillmentPort {
    requests: Mutex<Vec<FulfillPaidPhysicalOrderRequest>>,
}

impl CapturingPhysicalGoodsFulfillmentPort {
    fn requests(&self) -> Vec<FulfillPaidPhysicalOrderRequest> {
        self.requests
            .lock()
            .expect("physical requests lock")
            .clone()
    }
}

impl PhysicalGoodsFulfillmentPort for CapturingPhysicalGoodsFulfillmentPort {
    fn fulfill_paid_physical_order<'a>(
        &'a self,
        request: FulfillPaidPhysicalOrderRequest,
    ) -> PhysicalGoodsFuture<'a, PhysicalGoodsFulfillmentOutcome> {
        self.requests
            .lock()
            .expect("physical requests lock")
            .push(request);
        Box::pin(async {
            Ok(PhysicalGoodsFulfillmentOutcome {
                accepted: true,
                replayed: false,
                fulfillment_status: "awaiting_shipment".to_owned(),
            })
        })
    }
}

#[derive(Default)]
struct CapturingMembershipPurchaseFulfillmentPort {
    requests: Mutex<Vec<MembershipPurchaseFulfillmentRequest>>,
}

impl CapturingMembershipPurchaseFulfillmentPort {
    fn requests(&self) -> Vec<MembershipPurchaseFulfillmentRequest> {
        self.requests.lock().expect("requests lock").clone()
    }
}

impl MembershipPurchaseFulfillmentPort for CapturingMembershipPurchaseFulfillmentPort {
    fn fulfill_membership_purchase<'a>(
        &'a self,
        request: MembershipPurchaseFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, MembershipPurchaseFulfillmentOutcome> {
        self.requests.lock().expect("requests lock").push(request);
        Box::pin(async {
            Ok(MembershipPurchaseFulfillmentOutcome {
                accepted: true,
                replayed: false,
                fulfillment_status: "active".to_owned(),
            })
        })
    }

    fn fulfill_coupon_subscription<'a>(
        &'a self,
        _request: CouponSubscriptionFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, CouponSubscriptionFulfillmentOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "coupon subscription should not be called for membership purchase",
            ))
        })
    }

    fn fulfill_membership_quota_recharge<'a>(
        &'a self,
        _request: MembershipQuotaRechargeFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, MembershipQuotaRechargeFulfillmentOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "quota recharge should not be called for membership purchase",
            ))
        })
    }
}

impl MembershipPurchaseFulfillmentPort for UnsupportedMembershipPurchaseFulfillmentPort {
    fn fulfill_membership_purchase<'a>(
        &'a self,
        _request: MembershipPurchaseFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, MembershipPurchaseFulfillmentOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "membership port should not be called for token_bank_recharge",
            ))
        })
    }

    fn fulfill_coupon_subscription<'a>(
        &'a self,
        _request: CouponSubscriptionFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, CouponSubscriptionFulfillmentOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "membership coupon port should not be called for token_bank_recharge",
            ))
        })
    }

    fn fulfill_membership_quota_recharge<'a>(
        &'a self,
        _request: MembershipQuotaRechargeFulfillmentRequest,
    ) -> MembershipPurchaseFulfillmentFuture<'a, MembershipQuotaRechargeFulfillmentOutcome> {
        Box::pin(async {
            Err(CommerceServiceError::unsupported_capability(
                "membership quota recharge port should not be called for token_bank_recharge",
            ))
        })
    }
}

impl AccountValueFulfillmentStore for MockAccountValueFulfillmentStore {
    fn load_account_value_fulfillment_context<'a>(
        &'a self,
        command: &'a FulfillAccountValueOrderCommand,
    ) -> AccountValueFulfillmentFuture<'a, Option<AccountValueFulfillmentContext>> {
        let context = self
            .contexts
            .lock()
            .expect("contexts lock")
            .get(&command.order_id)
            .cloned();
        Box::pin(async move { Ok(context) })
    }

    fn reserve_account_value_fulfillment<'a>(
        &'a self,
        _command: &'a FulfillAccountValueOrderCommand,
        _context: &'a AccountValueFulfillmentContext,
    ) -> AccountValueFulfillmentFuture<'a, ()> {
        *self.reserve_calls.lock().expect("reserve lock") += 1;
        Box::pin(async { Ok(()) })
    }

    fn release_account_value_fulfillment_reservation<'a>(
        &'a self,
        _command: &'a FulfillAccountValueOrderCommand,
        _context: &'a AccountValueFulfillmentContext,
    ) -> AccountValueFulfillmentFuture<'a, ()> {
        *self.release_calls.lock().expect("release lock") += 1;
        Box::pin(async { Ok(()) })
    }

    fn commit_account_value_fulfillment<'a>(
        &'a self,
        _command: FulfillAccountValueOrderCommand,
        context: &'a AccountValueFulfillmentContext,
    ) -> AccountValueFulfillmentFuture<'a, FulfillAccountValueOrderOutcome> {
        *self.commit_calls.lock().expect("commit lock") += 1;
        Box::pin(async move { Ok(FulfillAccountValueOrderOutcome::fulfilled(context)) })
    }
}

#[derive(Default)]
struct MockAccountValueLedgerPort {
    commands: Mutex<Vec<AccountValueLedgerCommand>>,
}

impl MockAccountValueLedgerPort {
    fn commands(&self) -> Vec<AccountValueLedgerCommand> {
        self.commands.lock().expect("commands lock").clone()
    }
}

impl AccountValueLedgerPort for MockAccountValueLedgerPort {
    fn apply_account_value_ledger_command<'a>(
        &'a self,
        command: AccountValueLedgerCommand,
    ) -> AccountValueFulfillmentFuture<'a, AccountValueLedgerOutcome> {
        self.commands.lock().expect("commands lock").push(command);
        Box::pin(async {
            Ok(AccountValueLedgerOutcome {
                accepted: true,
                replayed: false,
                ledger_entry_id: Some("ledger-1".to_owned()),
                account_effect_reference_id: None,
            })
        })
    }
}
