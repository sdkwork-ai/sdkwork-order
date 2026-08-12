//! Order-side payment notify ports over the payment repository and provider
//! adapters.
//!
//! This adapter is the single implementation of the order-service notify
//! ports (`PaymentNotifyVerifyPort`, `PaymentNotifyIngestPort`,
//! `PaymentNotifyOrderContextPort`, `RefundNotifyIngestPort`) shared by every
//! ingest surface:
//!
//! - the HTTP webhook routers (`sdkwork-routes-order-app-api`) verify and
//!   ingest provider notifications through it, and
//! - the payment compensation worker re-enters the same notify processing
//!   framework with synthetic query events through it.
//!
//! Keeping the port implementations in the integration layer (instead of the
//! HTTP adapter layer) lets background and HTTP surfaces share one code path:
//! idempotent ingest, exact-attempt identity, terminal-conflict acks, and
//! rejected-webhook forensics behave identically on both paths.

use std::sync::Arc;

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_order_repository_sqlx::{OrderPaymentSettlementContext, PostgresCommerceOrderStore};
use sdkwork_order_service::{
    PaymentNotifyAttemptContext, PaymentNotifyEvent, PaymentNotifyIngestOutcome,
    PaymentNotifyIngestPort, PaymentNotifyOrderContext, PaymentNotifyOrderContextPort,
    PaymentNotifyVerifyPort, RefundNotifyContext, RefundNotifyIngestFuture,
    RefundNotifyIngestOutcome, RefundNotifyIngestPort,
};
use sdkwork_payment_providers::{
    normalize_provider_code, peek_webhook_routing_fields, provider_registry_for_account,
    PaymentNormalizeWebhookRequest, PaymentProviderRegistry, PaymentVerifyWebhookRequest,
    ProviderCredentialBundle,
};
use sdkwork_payment_repository_sqlx::{
    ingest_provider_refund_webhook_postgres, ingest_provider_webhook_postgres,
    load_active_provider_account_by_merchant_id_postgres, load_active_provider_account_postgres,
    load_webhook_attempt_context_by_out_trade_no_postgres, provider_account_binding,
    record_rejected_provider_webhook_postgres, IngestProviderWebhookCommand,
    PaymentProviderAccountRecord, PaymentWebhookAttemptContext,
};
use sqlx::PgPool;

/// Shared notify ports over a PostgreSQL pool. Constructed by the webhook
/// routers and the compensation worker from the same credentials and
/// deployment registry so every ingest surface resolves provider accounts
/// identically.
#[derive(Clone)]
pub struct StorePaymentNotifyPorts {
    pool: PgPool,
    credentials: ProviderCredentialBundle,
    deployment_registry: Arc<PaymentProviderRegistry>,
}

impl StorePaymentNotifyPorts {
    pub fn postgres(
        pool: PgPool,
        credentials: ProviderCredentialBundle,
        deployment_registry: Arc<PaymentProviderRegistry>,
    ) -> Self {
        Self {
            pool,
            credentials,
            deployment_registry,
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn credentials(&self) -> &ProviderCredentialBundle {
        &self.credentials
    }

    pub fn deployment_registry(&self) -> &PaymentProviderRegistry {
        self.deployment_registry.as_ref()
    }

    pub fn from_database_pool(pool: &DatabasePool) -> Self {
        // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
        let DatabasePool::Postgres(pool, _) = pool else {
            panic!("payment notify ports require a PostgreSQL database pool");
        };
        Self::postgres(
            pool.clone(),
            ProviderCredentialBundle::from_env(),
            Arc::new(PaymentProviderRegistry::from_credentials(
                ProviderCredentialBundle::from_env(),
            )),
        )
    }
}

impl PaymentNotifyVerifyPort for StorePaymentNotifyPorts {
    fn verify_and_normalize<'a>(
        &'a self,
        provider_code: &'a str,
        headers: &'a [(String, String)],
        body: &'a [u8],
    ) -> sdkwork_order_service::PaymentNotifyVerifyFuture<'a> {
        Box::pin(async move {
            let provider_code = normalize_provider_code(provider_code);
            let resolution = resolve_webhook_provider_account_postgres(
                &self.pool,
                &self.credentials,
                self.deployment_registry.as_ref(),
                &provider_code,
                body,
            )
            .await?;
            let adapter = match resolution.registry.resolve(&provider_code) {
                Some(adapter) => adapter,
                None => {
                    return Err(CommerceServiceError::validation(format!(
                        "payment provider {provider_code} is not configured"
                    )));
                }
            };

            let verify_request = PaymentVerifyWebhookRequest {
                headers: headers.to_vec(),
                body: body.to_vec(),
                metadata: serde_json::json!({ "provider_code": provider_code }),
            };
            match adapter.verify_webhook(verify_request).await {
                Ok(outcome) if outcome.verified => {}
                Ok(_) => {
                    let reason = "webhook signature verification failed";
                    tracing::warn!(target = "order.payment_notify", provider_code, reason);
                    record_rejected_provider_webhook_postgres(
                        &self.pool,
                        &provider_code,
                        body,
                        reason,
                    )
                    .await?;
                    return Err(CommerceServiceError::validation(
                        "webhook signature verification failed",
                    ));
                }
                Err(error) => {
                    tracing::warn!(
                        target = "order.payment_notify",
                        provider_code,
                        error = %format!("{error:?}"),
                        "webhook verification provider error"
                    );
                    record_rejected_provider_webhook_postgres(
                        &self.pool,
                        &provider_code,
                        body,
                        "webhook verification provider error",
                    )
                    .await?;
                    // Sanitized: internal provider error details stay in logs.
                    return Err(CommerceServiceError::validation(
                        "webhook signature verification failed",
                    ));
                }
            }

            let normalize_request = PaymentNormalizeWebhookRequest {
                headers: headers.to_vec(),
                body: body.to_vec(),
                metadata: serde_json::json!({ "provider_code": provider_code }),
            };
            let event = match adapter.normalize_webhook(normalize_request).await {
                Ok(event) => event,
                Err(error) => {
                    tracing::warn!(
                        target = "order.payment_notify",
                        provider_code,
                        error = %format!("{error:?}"),
                        "webhook normalization provider error"
                    );
                    record_rejected_provider_webhook_postgres(
                        &self.pool,
                        &provider_code,
                        body,
                        "webhook normalization provider error",
                    )
                    .await?;
                    return Err(CommerceServiceError::validation("bad request"));
                }
            };

            Ok(PaymentNotifyEvent {
                provider_code: event.provider_code,
                provider_event_id: event.provider_event_id,
                event_type: event.event_type,
                out_trade_no: event.out_trade_no,
                payment_status: event.payment_status,
                payload: event.payload,
                tenant_id: resolution
                    .scope
                    .as_ref()
                    .map(|scope| scope.tenant_id.clone()),
                organization_id: resolution
                    .scope
                    .as_ref()
                    .and_then(|scope| scope.organization_id.clone()),
            })
        })
    }
}

impl PaymentNotifyIngestPort for StorePaymentNotifyPorts {
    fn ingest<'a>(
        &'a self,
        event: PaymentNotifyEvent,
    ) -> sdkwork_order_service::PaymentNotifyIngestFuture<'a> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let outcome = ingest_provider_webhook_postgres(
                &pool,
                IngestProviderWebhookCommand {
                    provider_code: event.provider_code,
                    provider_event_id: event.provider_event_id.unwrap_or_default(),
                    event_type: event.event_type,
                    out_trade_no: event.out_trade_no,
                    payment_status: event.payment_status,
                    payload: event.payload,
                    tenant_id: event.tenant_id,
                    organization_id: event.organization_id,
                },
            )
            .await?;
            Ok(PaymentNotifyIngestOutcome {
                webhook_event_id: outcome.webhook_event_id,
                replayed: outcome.replayed,
                applied_status: outcome.applied_status,
                applied: outcome.applied,
                attempt: outcome
                    .payment_attempt_context
                    .as_ref()
                    .map(payment_notify_attempt_context),
            })
        })
    }
}

impl PaymentNotifyOrderContextPort for StorePaymentNotifyPorts {
    fn load_order_settlement_context<'a>(
        &'a self,
        tenant_id: &'a str,
        organization_id: Option<&'a str>,
        order_id: &'a str,
    ) -> sdkwork_order_service::PaymentNotifyOrderContextFuture<'a> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let store = PostgresCommerceOrderStore::new(pool);
            let context = store
                .load_order_payment_settlement_context(tenant_id, organization_id, order_id)
                .await?;
            Ok(context.map(payment_notify_order_context))
        })
    }
}

impl RefundNotifyIngestPort for StorePaymentNotifyPorts {
    fn ingest<'a>(&'a self, event: PaymentNotifyEvent) -> RefundNotifyIngestFuture<'a> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let outcome = ingest_provider_refund_webhook_postgres(
                &pool,
                IngestProviderWebhookCommand {
                    provider_code: event.provider_code,
                    provider_event_id: event.provider_event_id.unwrap_or_default(),
                    event_type: event.event_type,
                    out_trade_no: event.out_trade_no,
                    payment_status: event.payment_status,
                    payload: event.payload,
                    tenant_id: event.tenant_id,
                    organization_id: event.organization_id,
                },
            )
            .await?;
            Ok(RefundNotifyIngestOutcome {
                webhook_event_id: outcome.webhook_event_id,
                replayed: outcome.replayed,
                refund: outcome.refund.as_ref().map(|refund| RefundNotifyContext {
                    refund_id: refund.refund_id.clone(),
                    refund_no: refund.refund_no.clone(),
                    order_id: refund.order_id.clone(),
                    tenant_id: refund.tenant_id.clone(),
                    organization_id: refund.organization_id.clone(),
                    status: refund.status.clone(),
                    amount: refund.amount.clone(),
                    business_type: sdkwork_order_service::REFUND_NOTIFY_BUSINESS_REFUND.to_owned(),
                }),
                payment_attempt: outcome.payment_attempt_context.as_ref().map(|attempt| {
                    PaymentNotifyAttemptContext {
                        payment_attempt_id: attempt.payment_attempt_id.clone(),
                        out_trade_no: attempt.out_trade_no.clone(),
                        tenant_id: attempt.tenant_id.clone(),
                        organization_id: attempt.organization_id.clone(),
                        owner_user_id: attempt.owner_user_id.clone(),
                        order_id: attempt.order_id.clone(),
                    }
                }),
            })
        })
    }
}

fn payment_notify_attempt_context(
    attempt: &PaymentWebhookAttemptContext,
) -> PaymentNotifyAttemptContext {
    PaymentNotifyAttemptContext {
        payment_attempt_id: attempt.payment_attempt_id.clone(),
        out_trade_no: attempt.out_trade_no.clone(),
        tenant_id: attempt.tenant_id.clone(),
        organization_id: attempt.organization_id.clone(),
        owner_user_id: attempt.owner_user_id.clone(),
        order_id: attempt.order_id.clone(),
    }
}

fn payment_notify_order_context(
    context: OrderPaymentSettlementContext,
) -> PaymentNotifyOrderContext {
    PaymentNotifyOrderContext {
        subject: context.subject,
        membership_purchase: context.membership_purchase,
    }
}

struct WebhookProviderScope {
    tenant_id: String,
    organization_id: Option<String>,
}

struct WebhookProviderResolution {
    registry: PaymentProviderRegistry,
    scope: Option<WebhookProviderScope>,
}

async fn resolve_webhook_provider_account_postgres(
    pool: &PgPool,
    credentials: &ProviderCredentialBundle,
    deployment_registry: &PaymentProviderRegistry,
    provider_code: &str,
    body: &[u8],
) -> Result<WebhookProviderResolution, CommerceServiceError> {
    let peek = peek_webhook_routing_fields(provider_code, body);
    let attempt_context = if let Some(out_trade_no) = peek.out_trade_no.as_deref() {
        load_webhook_attempt_context_by_out_trade_no_postgres(pool, provider_code, out_trade_no)
            .await?
    } else {
        None
    };
    let fallback_scope = attempt_context
        .as_ref()
        .map(|context| WebhookProviderScope {
            tenant_id: context.tenant_id.clone(),
            organization_id: context.organization_id.clone(),
        });
    let account = if let Some(context) = attempt_context.as_ref() {
        load_active_provider_account_postgres(
            pool,
            &context.tenant_id,
            context.organization_id.as_deref(),
            &context.provider_code,
        )
        .await?
    } else if let Some(merchant_id) = peek.merchant_id.as_deref() {
        load_active_provider_account_by_merchant_id_postgres(pool, provider_code, merchant_id)
            .await?
    } else {
        None
    };
    Ok(webhook_provider_resolution(
        deployment_registry,
        credentials,
        account,
        fallback_scope,
    ))
}

fn webhook_provider_resolution(
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    account: Option<PaymentProviderAccountRecord>,
    fallback_scope: Option<WebhookProviderScope>,
) -> WebhookProviderResolution {
    let scope = account
        .as_ref()
        .map(|record| WebhookProviderScope {
            tenant_id: record.tenant_id.clone(),
            organization_id: record.organization_id.clone(),
        })
        .or(fallback_scope);
    let registry = account.as_ref().map_or_else(
        || deployment_registry.clone(),
        |record| provider_registry_for_account(credentials, Some(provider_account_binding(record))),
    );
    WebhookProviderResolution { registry, scope }
}
