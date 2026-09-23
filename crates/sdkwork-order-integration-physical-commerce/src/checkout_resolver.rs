use std::collections::HashSet;
use std::sync::Arc;

use sdkwork_commerce_money::{Money, MoneyUnit, RoundingMode, UnitCode};
use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_database_id::IdGenerator;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_merchandise_repository_sqlx::PostgresCommerceCatalogStore;
use sdkwork_merchandise_service::{ProductSkuRetrieveQuery, ProductSpuRetrieveQuery, SkuRecord};
use sdkwork_order_service::{
    PhysicalCheckoutResolverPort, PhysicalPurchaseFuture, ResolvePhysicalCheckoutRequest,
    ResolvedPhysicalCheckout, ResolvedPhysicalCheckoutLine,
};
use sdkwork_shop_repository_sqlx::PostgresCommerceShopStore;
use sdkwork_shop_service::{ShopScopeQuery, ShopSummaryView};

enum CatalogStore {
    Postgres(PostgresCommerceCatalogStore),
}

enum ShopStore {
    Postgres(PostgresCommerceShopStore),
}

pub struct PhysicalCheckoutAdapter {
    catalog: CatalogStore,
    shops: ShopStore,
}

impl PhysicalCheckoutAdapter {
    /// Builds the resolver over the merchandise and shop pools.
    ///
    /// `ids` is the caller's process Snowflake identity: the catalog store mints merchandise primary
    /// keys from it, so it is a constructor argument rather than a generator created here.
    pub fn new(
        merchandise_pool: DatabasePool,
        shop_pool: DatabasePool,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
        let DatabasePool::Postgres(pool, _) = merchandise_pool else {
            panic!("physical checkout resolver requires a PostgreSQL merchandise pool");
        };
        let catalog = CatalogStore::Postgres(PostgresCommerceCatalogStore::new(pool, ids));
        let DatabasePool::Postgres(pool, _) = shop_pool else {
            panic!("physical checkout resolver requires a PostgreSQL shop pool");
        };
        let shops = ShopStore::Postgres(PostgresCommerceShopStore::new(pool));
        Self { catalog, shops }
    }

    async fn resolve(
        &self,
        request: ResolvePhysicalCheckoutRequest,
    ) -> Result<ResolvedPhysicalCheckout, CommerceServiceError> {
        if request.lines.is_empty() {
            return Err(CommerceServiceError::validation(
                "physical checkout requires at least one line",
            ));
        }

        // The registry unit is read once, before any line is priced: every line settles in the
        // checkout currency (`validate_sku` refuses a SKU whose currency differs), and an adapter
        // that has to place a decimal point takes that currency's declared rounding mode from the
        // registry rather than naming one itself.
        let currency_unit = self.currency_unit(&request.currency_code).await?;

        let mut merchant_organization_id: Option<String> = None;
        let mut resolved_lines = Vec::with_capacity(request.lines.len());
        let mut requested_sku_ids = HashSet::with_capacity(request.lines.len());
        for requested in &request.lines {
            if requested.quantity <= 0 {
                return Err(CommerceServiceError::validation(
                    "physical checkout quantity must be greater than zero",
                ));
            }
            if !requested_sku_ids.insert(requested.sku_id.trim()) {
                return Err(CommerceServiceError::validation(
                    "physical checkout contains duplicate SKU lines",
                ));
            }
            let sku = self
                .retrieve_sku(&request.tenant_id, &requested.sku_id)
                .await?
                .ok_or_else(|| CommerceServiceError::not_found("checkout SKU was not found"))?;
            validate_sku(&sku, &request.currency_code, currency_unit.rounding())?;
            // `organization_id` is a Snowflake BIGINT whose platform default is 0 ("no
            // organization"), so "unset" is tested against zero rather than an empty string.
            if sku.organization_id <= 0 {
                return Err(CommerceServiceError::conflict(
                    "physical SKU has no merchant organization",
                ));
            }
            let seller = sku.organization_id.to_string();
            if merchant_organization_id
                .as_deref()
                .is_some_and(|existing| existing != seller.as_str())
            {
                return Err(CommerceServiceError::conflict(
                    "cross-shop checkout is not supported",
                ));
            }
            merchant_organization_id = Some(seller);

            // The retrieve query is still addressed by the decimal string form of the id
            // (API_SPEC §13.6: int64 crosses every boundary as a string).
            let spu_id = sku.spu_id.to_string();
            let spu = self
                .retrieve_spu(&request.tenant_id, &spu_id)
                .await?
                .ok_or_else(|| CommerceServiceError::not_found("checkout product was not found"))?;
            if !spu.status.eq_ignore_ascii_case("active") {
                return Err(CommerceServiceError::conflict(
                    "checkout product is not active",
                ));
            }
            if !matches!(
                spu.product_type.trim().to_ascii_lowercase().as_str(),
                "physical" | "physical_goods" | "physical_shipment"
            ) {
                return Err(CommerceServiceError::conflict(
                    "checkout product is not a physical product",
                ));
            }

            resolved_lines.push((sku, requested.quantity));
        }

        let merchant_organization_id = merchant_organization_id.ok_or_else(|| {
            CommerceServiceError::conflict("physical checkout merchant is unavailable")
        })?;
        let shop = self
            .retrieve_current_shop(&request.tenant_id, &merchant_organization_id)
            .await?
            .ok_or_else(|| CommerceServiceError::not_found("merchant shop was not found"))?;
        validate_shop(&shop, &request.currency_code)?;

        let shop_snapshot_json = serde_json::json!({
            "shopId": shop.shop_id,
            "shopNo": shop.shop_no,
            "shopName": shop.shop_name,
            "merchantOrganizationId": shop.organization_id,
            "storefrontStatus": shop.storefront_status,
            "operationStatus": shop.operation_status,
            "reviewStatus": shop.review_status,
            "currencyCode": shop.default_currency_code,
            "version": shop.version,
        })
        .to_string();

        let lines = resolved_lines
            .into_iter()
            .map(|(sku, quantity)| {
                let title = non_blank(sku.title.as_deref())
                    .or_else(|| non_blank(sku.name.as_deref()))
                    .unwrap_or_default();
                let price_amount = sku_unit_price(&sku, currency_unit.rounding())?;
                let unit_price = CommerceMoney::new(&price_amount).map_err(|error| {
                    CommerceServiceError::validation(format!(
                        "physical SKU price is invalid: {error}"
                    ))
                })?;
                // Ids are published as decimal strings: the snapshot is persisted beside int64
                // columns whose every other boundary is a string (API_SPEC §13.6).
                let sku_snapshot_json = serde_json::json!({
                    "skuId": sku.id.to_string(),
                    "skuNo": sku.sku_no,
                    "productId": sku.spu_id.to_string(),
                    "title": title,
                    "merchantOrganizationId": merchant_organization_id,
                    "shopId": shop.shop_id,
                    "priceAmount": price_amount,
                    "currencyCode": sku.currency_code,
                    "fulfillmentType": sku.fulfillment_type,
                    "inventoryTracking": sku.inventory_tracking,
                    "publishedAt": sku.published_at,
                    "versionAt": sku.updated_at,
                })
                .to_string();
                Ok(ResolvedPhysicalCheckoutLine {
                    sku_id: sku.id.to_string(),
                    product_id: sku.spu_id.to_string(),
                    merchant_organization_id: merchant_organization_id.clone(),
                    shop_id: shop.shop_id.clone(),
                    title,
                    unit_price,
                    currency_code: sku.currency_code.to_ascii_uppercase(),
                    fulfillment_type: sku.fulfillment_type,
                    quantity,
                    inventory_tracking: sku.inventory_tracking,
                    sku_snapshot_json,
                })
            })
            .collect::<Result<Vec<_>, CommerceServiceError>>()?;

        Ok(ResolvedPhysicalCheckout {
            merchant_organization_id,
            shop_id: shop.shop_id,
            shop_snapshot_json,
            shipping_address: request.shipping_address,
            lines,
        })
    }

    async fn retrieve_sku(
        &self,
        tenant_id: &str,
        sku_id: &str,
    ) -> Result<Option<SkuRecord>, CommerceServiceError> {
        let query = ProductSkuRetrieveQuery {
            tenant_id: tenant_id.to_owned(),
            sku_id: sku_id.to_owned(),
        };
        match &self.catalog {
            CatalogStore::Postgres(store) => store.retrieve_sku(&query).await,
        }
    }

    async fn retrieve_spu(
        &self,
        tenant_id: &str,
        spu_id: &str,
    ) -> Result<Option<sdkwork_merchandise_service::SpuRecord>, CommerceServiceError> {
        let query = ProductSpuRetrieveQuery {
            tenant_id: tenant_id.to_owned(),
            spu_id: spu_id.to_owned(),
        };
        match &self.catalog {
            CatalogStore::Postgres(store) => store.retrieve_spu(&query).await,
        }
    }

    /// The registry unit for `code`, read from the same `commerce_currency` row the catalog
    /// validates a price against.
    ///
    /// The catalog owns that row, and it publishes money as an exact count of minor units
    /// (`API_SPEC` section 13.2.1), so an adapter that has to place the decimal point reads the unit
    /// here rather than assembling one from a rounding mode of its own choosing. `sku_unit_price`
    /// keeps the SKU row's snapshotted `price_scale` as the scale — that exponent is what the price
    /// was written at, so a later registry change cannot re-scale a stored price — and takes only
    /// the mode from this unit: a unit assembled from a scale the caller paired with a mode it
    /// picked itself is only *mostly* read from the registry.
    async fn currency_unit(&self, code: &str) -> Result<MoneyUnit, CommerceServiceError> {
        match &self.catalog {
            CatalogStore::Postgres(store) => store.currency_unit(code).await,
        }
    }

    async fn retrieve_current_shop(
        &self,
        tenant_id: &str,
        organization_id: &str,
    ) -> Result<Option<ShopSummaryView>, CommerceServiceError> {
        let scope = ShopScopeQuery::new(tenant_id, Some(organization_id))?;
        match &self.shops {
            ShopStore::Postgres(store) => store.retrieve_current_shop(scope).await,
        }
    }
}

impl PhysicalCheckoutResolverPort for PhysicalCheckoutAdapter {
    fn resolve_physical_checkout<'a>(
        &'a self,
        request: ResolvePhysicalCheckoutRequest,
    ) -> PhysicalPurchaseFuture<'a, ResolvedPhysicalCheckout> {
        Box::pin(async move { self.resolve(request).await })
    }
}

fn validate_sku(
    sku: &SkuRecord,
    currency_code: &str,
    rounding: RoundingMode,
) -> Result<(), CommerceServiceError> {
    if !sku.status.eq_ignore_ascii_case("active") {
        return Err(CommerceServiceError::conflict("checkout SKU is not active"));
    }
    if !matches!(
        sku.fulfillment_type.trim().to_ascii_lowercase().as_str(),
        "physical" | "physical_shipment"
    ) {
        return Err(CommerceServiceError::conflict(
            "checkout SKU is not physically shippable",
        ));
    }
    if matches!(
        sku.inventory_tracking.trim().to_ascii_lowercase().as_str(),
        "none" | "disabled" | "untracked"
    ) {
        return Err(CommerceServiceError::conflict(
            "physical SKU must enable inventory tracking",
        ));
    }
    if !sku.currency_code.eq_ignore_ascii_case(currency_code) {
        return Err(CommerceServiceError::conflict(
            "checkout SKU currency does not match checkout currency",
        ));
    }
    let price_amount = sku_unit_price(sku, rounding)?;
    CommerceMoney::new(&price_amount).map_err(|error| {
        CommerceServiceError::validation(format!("physical SKU price is invalid: {error}"))
    })?;
    Ok(())
}

/// The first of `values` that carries something other than whitespace.
///
/// The catalog models `title` and `name` as nullable columns, and a row may carry either, both, or
/// neither: the caller picks by content rather than by presence.
fn non_blank(value: Option<&str>) -> Option<String> {
    value
        .filter(|candidate| !candidate.trim().is_empty())
        .map(str::to_owned)
}

/// The SKU's unit price as a major-denomination decimal string.
///
/// The catalog publishes money as exact minor units plus the currency exponent snapshotted at write
/// time, so the decimal point is placed from `price_scale`. Reconstructing the amount from a
/// hardcoded `/ 100` is the defect that snapshot exists to make impossible, and `f64` is never used
/// for money.
///
/// `rounding` is the currency registry's declared mode, supplied by the caller from
/// `PhysicalCheckoutAdapter::currency_unit` rather than named here: the exponent is the SKU row's
/// fact, but the mode its currency rounds in is the registry's. Placing a point performs no
/// conversion, so the mode leaves this amount unchanged today; naming one locally would still be
/// the one place a bad `rounding_mode` token stopped being reported.
fn sku_unit_price(
    sku: &SkuRecord,
    rounding: RoundingMode,
) -> Result<String, CommerceServiceError> {
    let scale = u8::try_from(sku.price_scale).map_err(|_| {
        CommerceServiceError::validation(format!(
            "physical SKU price scale {} is out of range",
            sku.price_scale
        ))
    })?;
    let code = UnitCode::new(&sku.currency_code).map_err(|error| {
        CommerceServiceError::validation(format!("physical SKU currency is invalid: {error}"))
    })?;
    let unit = MoneyUnit::custom(code, scale, rounding).map_err(|error| {
        CommerceServiceError::validation(format!("physical SKU price unit is invalid: {error}"))
    })?;
    Ok(Money::from_minor(i128::from(sku.sale_price_minor), unit).to_major_string())
}

fn validate_shop(shop: &ShopSummaryView, currency_code: &str) -> Result<(), CommerceServiceError> {
    if !shop.operation_status.eq_ignore_ascii_case("active") {
        return Err(CommerceServiceError::conflict(
            "merchant shop is not operational",
        ));
    }
    if !matches!(
        shop.storefront_status.trim().to_ascii_lowercase().as_str(),
        "active" | "open" | "published"
    ) {
        return Err(CommerceServiceError::conflict(
            "merchant storefront is not open",
        ));
    }
    if !matches!(
        shop.review_status.trim().to_ascii_lowercase().as_str(),
        "approved" | "passed" | "active"
    ) {
        return Err(CommerceServiceError::conflict(
            "merchant shop has not passed review",
        ));
    }
    if !shop
        .default_currency_code
        .eq_ignore_ascii_case(currency_code)
    {
        return Err(CommerceServiceError::conflict(
            "merchant shop currency does not match checkout currency",
        ));
    }
    Ok(())
}
