use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_iam_context_service::{AuthLevel, DeploymentMode, Environment, IamAppContext};
use sdkwork_order_repository_sqlx::order_points_recharge_e2e_postgres_pool_from_env;
use sdkwork_order_service::{
    AccountPointsCreditFuture, AccountPointsCreditPort, AccountValueFuture,
    AccountValueLedgerCommand, AccountValueLedgerOperation, AccountValueLedgerOutcome,
    AccountValueLedgerPort, NoopAccountValueLedgerPort, NoopMembershipPurchaseFulfillmentPort,
    PaymentExecutorOutcome, PaymentPayoutExecutionRequest, PaymentPayoutExecutorPort,
    PaymentRefundExecutionRequest, PaymentRefundExecutorPort, PointsRechargeCreditOutcome,
    PointsRechargeCreditRequest, UnavailablePhysicalInventoryReservationPort,
};
use sdkwork_order_service_host::OrderServiceHost;
use sdkwork_database_config::{DatabaseConfig, DatabaseEngine};
use sdkwork_database_sqlx::{DatabasePool, PoolContext};
use sdkwork_routes_order_backend_api::{
    backend_commerce_admin_router_with_postgres_pool,
    backend_order_admin_router_with_postgres_pool, openapi_contract::mount_backend_openapi,
    payment_confirmation_router_with_postgres_pool,
};
use serde_json::Value;
use std::sync::Arc;
use std::sync::Mutex;
use tower::util::ServiceExt;

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

fn build_test_backend_router(pool: sqlx::PgPool) -> Router {
    let credit = Arc::new(NoopAccountPointsCreditPort);
    mount_backend_openapi(
        Router::new()
            .merge(backend_order_admin_router_with_postgres_pool(
                pool.clone(),
                Arc::new(UnavailablePhysicalInventoryReservationPort),
            ))
            .merge(backend_commerce_admin_router_with_postgres_pool(
                pool.clone(),
            ))
            .merge(payment_confirmation_router_with_postgres_pool(
                pool,
                credit,
                Arc::new(NoopAccountValueLedgerPort),
                Arc::new(NoopMembershipPurchaseFulfillmentPort),
            )),
    )
}

async fn test_pool() -> Option<sqlx::PgPool> {
    order_points_recharge_e2e_postgres_pool_from_env().await
}

fn backend_iam_context() -> IamAppContext {
    IamAppContext::new(
        "100001",
        Some("org-1"),
        "900001",
        "session-1",
        "sdkwork-order-backend",
        Environment::Dev,
        DeploymentMode::Saas,
        AuthLevel::Mfa,
        vec!["tenant:100001".to_owned(), "organization:org-1".to_owned()],
        vec![
            "commerce.accountValue.review".to_owned(),
            "commerce.accountValue.read".to_owned(),
        ],
    )
}

#[derive(Default)]
struct RecordingAccountValueLedgerPort {
    commands: Mutex<Vec<AccountValueLedgerCommand>>,
}

impl RecordingAccountValueLedgerPort {
    fn operations(&self) -> Vec<AccountValueLedgerOperation> {
        self.commands
            .lock()
            .expect("ledger commands")
            .iter()
            .map(|command| command.operation)
            .collect()
    }
}

impl AccountValueLedgerPort for RecordingAccountValueLedgerPort {
    fn apply_account_value_ledger_command<'a>(
        &'a self,
        command: AccountValueLedgerCommand,
    ) -> AccountValueFuture<'a, AccountValueLedgerOutcome> {
        Box::pin(async move {
            let operation = command.operation;
            self.commands.lock().expect("ledger commands").push(command);
            Ok(AccountValueLedgerOutcome {
                accepted: true,
                replayed: false,
                ledger_entry_id: Some(format!("ledger-{operation:?}")),
                account_effect_reference_id: match operation {
                    AccountValueLedgerOperation::Hold => Some("hold-refund-1".to_owned()),
                    _ => None,
                },
            })
        })
    }
}

#[derive(Default)]
struct RecordingRefundExecutorPort {
    requests: Mutex<Vec<PaymentRefundExecutionRequest>>,
}

impl RecordingRefundExecutorPort {
    fn requests(&self) -> Vec<PaymentRefundExecutionRequest> {
        self.requests.lock().expect("refund requests").clone()
    }
}

impl PaymentRefundExecutorPort for RecordingRefundExecutorPort {
    fn execute_provider_refund<'a>(
        &'a self,
        request: PaymentRefundExecutionRequest,
    ) -> AccountValueFuture<'a, PaymentExecutorOutcome> {
        Box::pin(async move {
            self.requests.lock().expect("refund requests").push(request);
            Ok(PaymentExecutorOutcome {
                accepted: true,
                replayed: false,
                provider_reference_id: Some("payment-refund-1".to_owned()),
                status: "succeeded".to_owned(),
            })
        })
    }
}

struct FailingPayoutExecutorPort;

impl PaymentPayoutExecutorPort for FailingPayoutExecutorPort {
    fn execute_provider_payout<'a>(
        &'a self,
        _request: PaymentPayoutExecutionRequest,
    ) -> AccountValueFuture<'a, PaymentExecutorOutcome> {
        Box::pin(async move {
            Err(CommerceServiceError::unsupported_capability(
                "provider payout executor is not configured",
            ))
        })
    }
}

#[tokio::test]
async fn backend_openapi_document_is_served() {
    let Some(pool) = test_pool().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    let app = build_test_backend_router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/backend/v3/api/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn backend_router_mounts_every_openapi_operation_path() {
    let spec: Value = serde_json::from_str(include_str!(
        "../../../apis/backend-api/order/order-backend-api.openapi.json"
    ))
    .unwrap();
    let Some(pool) = test_pool().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    let app = build_test_backend_router(pool);
    let paths = spec["paths"].as_object().unwrap();

    for (template_path, methods) in paths {
        for method_name in methods.as_object().unwrap().keys() {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method_from_openapi(method_name))
                        .uri(concrete_uri(template_path))
                        .header("content-type", "application/json")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_ne!(
                response.status(),
                StatusCode::NOT_FOUND,
                "{method_name} {template_path} is not mounted"
            );
        }
    }
}

#[tokio::test]
async fn approving_refund_request_executes_account_hold_payment_refund_and_hold_settlement() {
    let Some(pool) = test_pool().await else {
        eprintln!("SKIP: SDKWORK_DATABASE_TEST_POSTGRES_URL is not configured");
        return;
    };
    sqlx::query(
        r#"
        INSERT INTO commerce_order_refund_request
            (id, tenant_id, organization_id, request_no, original_order_id, owner_user_id,
             target_asset, amount, currency_code, status, provider_amount, provider_currency_code,
             reason_code, reason_detail, review_comment, provider_reference_id,
             account_effect_reference_id, idempotency_key, created_at, updated_at)
        VALUES
            ('refund-request-1', '100001', 'org-1', 'refund-request-1', 'order-1', '200001',
             'token_bank', '32000', 'TOKEN_BANK', 'requested', '9900', 'CNY',
             'buyer_request', NULL, NULL, NULL, NULL, 'refund-request-idem', '1', '1')
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed refund request");

    let ledger = Arc::new(RecordingAccountValueLedgerPort::default());
    let refunds = Arc::new(RecordingRefundExecutorPort::default());
    let config = DatabaseConfig {
        engine: DatabaseEngine::Postgres,
        url: "postgres://ignored".to_owned(),
        ..Default::default()
    };
    let database_pool = DatabasePool::Postgres(pool.clone(), PoolContext { config });
    let host = Arc::new(
        OrderServiceHost::from_database_pool(database_pool)
            .await
            .expect("test order service host"),
    );
    let app = sdkwork_routes_order_backend_api::routes::build_order_backend_router(host);
    let body = serde_json::json!({
        "reasonCode": "approved",
        "reviewComment": "approved by operator"
    });
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/refund_requests/refund-request-1/approve")
                .header("content-type", "application/json")
                .header("Idempotency-Key", "approve-refund-1")
                .header("Sdkwork-Request-No", "approve-refund-1")
                .extension(backend_iam_context())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        ledger.operations(),
        vec![
            AccountValueLedgerOperation::Hold,
            AccountValueLedgerOperation::HoldSettle
        ]
    );
    let refund_requests = refunds.requests();
    assert_eq!(refund_requests.len(), 1);
    assert_eq!(refund_requests[0].refund_request_id, "refund-request-1");
    assert_eq!(
        refund_requests[0].amount,
        CommerceMoney::new("9900").unwrap()
    );
    assert_eq!(refund_requests[0].currency_code, "CNY");

    let row = sqlx::query(
        r#"
        SELECT status, provider_reference_id, account_effect_reference_id
        FROM commerce_order_refund_request
        WHERE id = 'refund-request-1'
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("refund request row");
    let status: String = sqlx::Row::try_get(&row, "status").unwrap();
    let provider_reference_id: Option<String> =
        sqlx::Row::try_get(&row, "provider_reference_id").unwrap();
    let account_effect_reference_id: Option<String> =
        sqlx::Row::try_get(&row, "account_effect_reference_id").unwrap();
    assert_eq!(status, "refunded");
    assert_eq!(provider_reference_id.as_deref(), Some("payment-refund-1"));
    assert_eq!(
        account_effect_reference_id.as_deref(),
        Some("hold-refund-1")
    );
}

fn method_from_openapi(method_name: &str) -> Method {
    match method_name.to_ascii_lowercase().as_str() {
        "get" => Method::GET,
        "post" => Method::POST,
        "put" => Method::PUT,
        "patch" => Method::PATCH,
        "delete" => Method::DELETE,
        other => panic!("unsupported openapi method: {other}"),
    }
}

fn concrete_uri(template_path: &str) -> String {
    template_path
        .replace("{orderId}", "order-1")
        .replace("{checkoutSessionId}", "session-1")
        .replace("{afterSalesRequestId}", "as-1")
        .replace("{shipmentId}", "shipment-1")
        .replace("{packageId}", "package-1")
        .replace("{planCode}", "plan-1")
        .replace("{refundRequestId}", "refund-1")
        .replace("{withdrawalRequestId}", "withdrawal-1")
        .replace("{fulfillmentId}", "fulfillment-1")
        .replace("{providerCode}", "wechat_pay")
}
