use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_repository_sqlx::{
    OrderPaymentSettlementContext, PostgresCommerceOrderStore, PostgresCommerceRechargeStore,
};
use sdkwork_order_service::{
    settle_owner_order_after_payment_success, AccountPointsCreditPort, AccountValueLedgerPort,
    CouponRedemptionPort, MembershipPurchaseFulfillmentPort, NoopCouponRedemptionPort,
    OwnerOrderPaymentReconciliationPort, OwnerOrderSettlementPorts, PhysicalGoodsFulfillmentPort,
    ReconcileOwnerOrderPaymentRequest, UnavailableOwnerOrderPaymentReconciliationPort,
    UnavailablePhysicalGoodsFulfillmentPort,
};
use sdkwork_payment_repository_sqlx::PostgresCommerceOwnerOrderPaymentStore;
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::api_response::{
    forbidden, map_service_error, not_found, success_created_item, unauthorized, validation,
};
use crate::backend_command_headers::resolve_required_backend_write_command_headers;

use crate::subject::{backend_operator_scope_from_iam, BackendOperatorScope};

mod permissions {
    pub const CONFIRM: &str = "commerce.orders.fulfill";
}

#[derive(Clone)]
enum PaymentConfirmationStoreKind {
    Postgres {
        payments: Arc<PostgresCommerceOwnerOrderPaymentStore>,
        recharge: Arc<PostgresCommerceRechargeStore>,
        orders: Arc<PostgresCommerceOrderStore>,
    },
}

#[derive(Clone)]
struct PaymentConfirmationState {
    store: PaymentConfirmationStoreKind,
    credit_port: Arc<dyn AccountPointsCreditPort>,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
    membership_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
    reconciliation_port: Arc<dyn OwnerOrderPaymentReconciliationPort>,
    physical_goods_port: Arc<dyn PhysicalGoodsFulfillmentPort>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmOrderPaymentRequest {
    request_no: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmOrderPaymentResponse {
    payment_confirmed: bool,
    payment_replayed: bool,
    fulfillment_accepted: bool,
    fulfillment_replayed: bool,
    order_id: String,
    points_credited: i64,
    fulfillment_status: String,
}

pub fn payment_confirmation_router_with_postgres_pool(
    pool: PgPool,
    credit_port: Arc<dyn AccountPointsCreditPort>,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    membership_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
) -> Router {
    payment_confirmation_router_with_postgres_pool_and_coupon(
        pool,
        credit_port,
        account_value_ledger_port,
        Arc::new(NoopCouponRedemptionPort),
        membership_port,
    )
}

pub fn payment_confirmation_router_with_postgres_pool_and_coupon(
    pool: PgPool,
    credit_port: Arc<dyn AccountPointsCreditPort>,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
    membership_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
) -> Router {
    payment_confirmation_router_with_postgres_pool_and_integrations(
        pool,
        credit_port,
        account_value_ledger_port,
        coupon_redemption_port,
        membership_port,
        Arc::new(UnavailableOwnerOrderPaymentReconciliationPort),
        Arc::new(UnavailablePhysicalGoodsFulfillmentPort),
    )
}

pub fn payment_confirmation_router_with_postgres_pool_and_integrations(
    pool: PgPool,
    credit_port: Arc<dyn AccountPointsCreditPort>,
    account_value_ledger_port: Arc<dyn AccountValueLedgerPort>,
    coupon_redemption_port: Arc<dyn CouponRedemptionPort>,
    membership_port: Arc<dyn MembershipPurchaseFulfillmentPort>,
    reconciliation_port: Arc<dyn OwnerOrderPaymentReconciliationPort>,
    physical_goods_port: Arc<dyn PhysicalGoodsFulfillmentPort>,
) -> Router {
    build_payment_confirmation_router(PaymentConfirmationState {
        store: PaymentConfirmationStoreKind::Postgres {
            payments: Arc::new(PostgresCommerceOwnerOrderPaymentStore::new(pool.clone())),
            recharge: Arc::new(PostgresCommerceRechargeStore::new(pool.clone())),
            orders: Arc::new(PostgresCommerceOrderStore::new(pool)),
        },
        credit_port,
        account_value_ledger_port,
        coupon_redemption_port,
        membership_port,
        reconciliation_port,
        physical_goods_port,
    })
}

fn build_payment_confirmation_router(state: PaymentConfirmationState) -> Router {
    Router::new()
        .route(
            "/backend/v3/api/orders/{orderId}/payment_confirmations",
            post(confirm_order_payment),
        )
        .with_state(state)
}

async fn confirm_order_payment(
    State(state): State<PaymentConfirmationState>,
    Extension(runtime_context): Extension<IamAppContext>,
    request_context: Extension<WebRequestContext>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    Json(body): Json<ConfirmOrderPaymentRequest>,
) -> Response {
    let ctx = Some(&request_context.0);
    let subject = match require_confirmation_subject(runtime_context, ctx) {
        Ok(subject) => subject,
        Err(response) => return *response,
    };

    if body.request_no.trim().is_empty() {
        return validation(ctx, "request_no is required");
    }

    let write_headers =
        match resolve_required_backend_write_command_headers(ctx, &headers, |idempotency_key| {
            format!("pay-confirm-{order_id}-{idempotency_key}")
        }) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let settlement_request_no =
        payment_reconciliation_request_no(&order_id, &write_headers.idempotency_key);

    let credit_port = state.credit_port.clone();
    let account_value_ledger_port = state.account_value_ledger_port.clone();
    let coupon_redemption_port = state.coupon_redemption_port.clone();
    let membership_port = state.membership_port.clone();
    let reconciliation_port = state.reconciliation_port.clone();
    let physical_goods_port = state.physical_goods_port.clone();
    match state.store {
        PaymentConfirmationStoreKind::Postgres {
            ref payments,
            ref recharge,
            ref orders,
        } => {
            confirm_order_payment_inner(
                ctx,
                &subject,
                &order_id,
                &settlement_request_no,
                orders.as_ref(),
                reconciliation_port.as_ref(),
                OwnerOrderSettlementPorts {
                    payment_store: payments.as_ref(),
                    order_state_store: orders.as_ref(),
                    recharge_store: recharge.as_ref(),
                    account_value_store: recharge.as_ref(),
                    credit_port: credit_port.as_ref(),
                    account_value_ledger_port: account_value_ledger_port.as_ref(),
                    coupon_redemption_port: coupon_redemption_port.as_ref(),
                    membership_port: membership_port.as_ref(),
                    physical_goods_port: physical_goods_port.as_ref(),
                },
            )
            .await
        }
    }
}

fn payment_reconciliation_request_no(order_id: &str, idempotency_key: &str) -> String {
    format!("pay-confirm-{order_id}-{idempotency_key}")
}

async fn confirm_order_payment_inner(
    ctx: Option<&WebRequestContext>,
    subject: &BackendOperatorScope,
    order_id: &str,
    request_no: &str,
    order_context_loader: &dyn OrderSettlementContextLoader,
    reconciliation_port: &dyn OwnerOrderPaymentReconciliationPort,
    settlement_ports: OwnerOrderSettlementPorts<'_>,
) -> Response {
    let order_context = match order_context_loader
        .load_order_payment_settlement_context(
            &subject.tenant_id,
            subject.organization_id.as_deref(),
            order_id,
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found(ctx, "order was not found"),
        Err(error) => return map_service_error(ctx, error),
    };

    let reconciliation = match reconciliation_port
        .reconcile_owner_order_payment(ReconcileOwnerOrderPaymentRequest {
            tenant_id: subject.tenant_id.clone(),
            organization_id: subject.organization_id.clone(),
            owner_user_id: order_context.owner_user_id,
            order_id: order_id.to_owned(),
        })
        .await
    {
        Ok(outcome) => outcome,
        Err(error) => return map_service_error(ctx, error),
    };

    tracing::info!(
        target = "order.payment.reconciliation",
        order_id,
        provider = %reconciliation.provider_code,
        provider_status = %reconciliation.provider_status,
        replayed = reconciliation.replayed,
        "payment provider reconciliation confirmed success"
    );

    let settlement_outcome = match settle_owner_order_after_payment_success(
        settlement_ports,
        &reconciliation.attempt,
        Some(order_context.subject.as_str()),
        order_context.membership_purchase.as_ref(),
        request_no,
    )
    .await
    {
        Ok(outcome) => outcome,
        Err(error) => return map_service_error(ctx, error),
    };

    success_created_item(
        ctx,
        ConfirmOrderPaymentResponse {
            payment_confirmed: settlement_outcome.payment_confirmed,
            payment_replayed: settlement_outcome.payment_replayed,
            fulfillment_accepted: settlement_outcome.fulfillment_accepted,
            fulfillment_replayed: settlement_outcome.fulfillment_replayed,
            order_id: settlement_outcome.order_id,
            points_credited: settlement_outcome.points_credited,
            fulfillment_status: settlement_outcome.fulfillment_status,
        },
    )
}

fn require_confirmation_subject(
    context: IamAppContext,
    web_context: Option<&WebRequestContext>,
) -> Result<BackendOperatorScope, Box<Response>> {
    if !context.can_access_backend_api() {
        return Err(Box::new(forbidden(
            web_context,
            "backend api access requires an organization-scoped session",
        )));
    }
    if !context.has_permission(permissions::CONFIRM) {
        return Err(Box::new(forbidden(
            web_context,
            format!("missing required permission: {}", permissions::CONFIRM),
        )));
    }
    match backend_operator_scope_from_iam(&context) {
        Ok(subject) => Ok(subject),
        Err(message) => Err(Box::new(unauthorized(web_context, message))),
    }
}

pub(crate) trait OrderSettlementContextLoader: Send + Sync {
    fn load_order_payment_settlement_context<'a>(
        &'a self,
        tenant_id: &'a str,
        organization_id: Option<&'a str>,
        order_id: &'a str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Option<OrderPaymentSettlementContext>, CommerceServiceError>,
                > + Send
                + 'a,
        >,
    >;
}

impl OrderSettlementContextLoader for PostgresCommerceOrderStore {
    fn load_order_payment_settlement_context<'a>(
        &'a self,
        tenant_id: &'a str,
        organization_id: Option<&'a str>,
        order_id: &'a str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Option<OrderPaymentSettlementContext>, CommerceServiceError>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            self.load_order_payment_settlement_context(tenant_id, organization_id, order_id)
                .await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::payment_reconciliation_request_no;

    #[test]
    fn reconciliation_settlement_identity_is_derived_from_the_required_idempotency_key() {
        assert_eq!(
            payment_reconciliation_request_no("order-1", "idem-key-123"),
            payment_reconciliation_request_no("order-1", "idem-key-123")
        );
        assert_ne!(
            payment_reconciliation_request_no("order-1", "idem-key-123"),
            payment_reconciliation_request_no("order-1", "idem-key-456")
        );
    }
}
