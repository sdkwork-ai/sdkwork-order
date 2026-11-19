//! PostgreSQL persistence for the buyer cart.
//!
//! One active cart per `(tenant_id, owner_user_id)`, enforced by the partial
//! unique index `uk_commerce_cart_owner_active`. Lines snapshot the merchandise
//! SKU at add time and are re-hydrated through the catalog for display fields the
//! snapshot does not own.

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_order_service::{
    CartLineView, CartStoreFuture, CartView, CommerceCartStore, CreateCartItemCommand,
    CurrentCartQuery, DeleteCartItemCommand, UpdateCartItemCommand, CART_DEFAULT_CURRENCY_CODE,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::sql_store_error::map_sql_store_error;

/// The authoritative cart store over the shared PostgreSQL pool.
#[derive(Debug, Clone)]
pub struct PostgresCartStore {
    pool: PgPool,
}

struct SkuSnapshot {
    spu_id: Option<String>,
    title: String,
    sku_name: String,
    unit_price_minor: i64,
    price_scale: i16,
    currency_code: String,
    min_order_quantity: i64,
    max_order_quantity: Option<i64>,
}

impl PostgresCartStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn retrieve_current_cart(
        &self,
        query: CurrentCartQuery,
    ) -> Result<CartView, CommerceServiceError> {
        let (cart_id, status, currency_code) = self
            .ensure_active_cart(
                &query.tenant_id,
                query.organization_id.as_deref(),
                &query.owner_user_id,
            )
            .await?;
        self.load_cart(&cart_id, &status, &currency_code).await
    }

    pub async fn create_cart_item(
        &self,
        command: CreateCartItemCommand,
    ) -> Result<CartLineView, CommerceServiceError> {
        let (cart_id, _, _) = self
            .ensure_active_cart(
                &command.tenant_id,
                command.organization_id.as_deref(),
                &command.owner_user_id,
            )
            .await?;
        let snapshot = self
            .load_sku_snapshot(&command.tenant_id, &command.sku_id)
            .await?
            .ok_or_else(|| CommerceServiceError::not_found("cart sku was not found"))?;
        validate_against_sku(command.quantity, &snapshot)?;

        let cart_item_id = Uuid::new_v4().to_string();
        let row = sqlx::query(
            r#"
            INSERT INTO commerce_cart_item
                (id, tenant_id, organization_id, cart_id, sku_id, spu_id, shop_id, quantity,
                 product_title, sku_name, unit_price_minor, price_scale, currency_code)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (cart_id, sku_id) DO UPDATE
            SET quantity = commerce_cart_item.quantity + EXCLUDED.quantity,
                spu_id = COALESCE(EXCLUDED.spu_id, commerce_cart_item.spu_id),
                shop_id = COALESCE(EXCLUDED.shop_id, commerce_cart_item.shop_id),
                product_title = COALESCE(EXCLUDED.product_title, commerce_cart_item.product_title),
                sku_name = COALESCE(EXCLUDED.sku_name, commerce_cart_item.sku_name),
                unit_price_minor = EXCLUDED.unit_price_minor,
                price_scale = EXCLUDED.price_scale,
                currency_code = EXCLUDED.currency_code,
                updated_at = now()
            RETURNING id
            "#,
        )
        .bind(&cart_item_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref().unwrap_or("0"))
        .bind(&cart_id)
        .bind(&command.sku_id)
        .bind(command.spu_id.as_deref().or(snapshot.spu_id.as_deref()))
        .bind(command.shop_id.as_deref())
        .bind(command.quantity)
        .bind(&snapshot.title)
        .bind(&snapshot.sku_name)
        .bind(snapshot.unit_price_minor)
        .bind(snapshot.price_scale)
        .bind(&snapshot.currency_code)
        .fetch_one(self.pool())
        .await
        .map_err(|error| store_error("failed to add a cart line", error))?;
        let cart_item_id: String = row
            .try_get("id")
            .map_err(|error| store_error("failed to read the cart line id", error))?;

        self.load_cart_line(&cart_item_id).await
    }

    pub async fn update_cart_item(
        &self,
        command: UpdateCartItemCommand,
    ) -> Result<CartLineView, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            UPDATE commerce_cart_item i
            SET quantity = $4,
                updated_at = now()
            FROM commerce_cart c
            WHERE i.cart_id = c.id
              AND i.id = $1
              AND c.tenant_id = $2
              AND c.owner_user_id = $3
              AND c.status = 'active'
            RETURNING i.id
            "#,
        )
        .bind(&command.cart_item_id)
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .bind(command.quantity)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to update a cart line", error))?
        .ok_or_else(|| CommerceServiceError::not_found("cart item was not found"))?;
        let cart_item_id: String = row
            .try_get("id")
            .map_err(|error| store_error("failed to read the cart line id", error))?;

        self.load_cart_line(&cart_item_id).await
    }

    pub async fn delete_cart_item(
        &self,
        command: DeleteCartItemCommand,
    ) -> Result<(), CommerceServiceError> {
        sqlx::query(
            r#"
            DELETE FROM commerce_cart_item i
            USING commerce_cart c
            WHERE i.cart_id = c.id
              AND i.id = $1
              AND c.tenant_id = $2
              AND c.owner_user_id = $3
              AND c.status = 'active'
            RETURNING i.id
            "#,
        )
        .bind(&command.cart_item_id)
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to remove a cart line", error))?
        .ok_or_else(|| CommerceServiceError::not_found("cart item was not found"))?;
        Ok(())
    }

    /// Creates the buyer's live cart when it does not exist yet and returns its
    /// identity plus the stored status and currency.
    async fn ensure_active_cart(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
    ) -> Result<(String, String, String), CommerceServiceError> {
        let now_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO commerce_cart
                (id, tenant_id, organization_id, owner_user_id, status, currency_code)
            VALUES
                ($1, $2, $3, $4, 'active', $5)
            ON CONFLICT (tenant_id, owner_user_id) WHERE status = 'active' DO NOTHING
            "#,
        )
        .bind(&now_id)
        .bind(tenant_id)
        .bind(organization_id.unwrap_or("0"))
        .bind(owner_user_id)
        .bind(CART_DEFAULT_CURRENCY_CODE)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to open the buyer cart", error))?;

        let row = sqlx::query(
            r#"
            SELECT id, status, currency_code
            FROM commerce_cart
            WHERE tenant_id = $1
              AND owner_user_id = $2
              AND status = 'active'
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(owner_user_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to read the buyer cart", error))?
        .ok_or_else(|| {
            CommerceServiceError::invalid_state("the buyer cart could not be resolved")
        })?;

        let id: String = row
            .try_get("id")
            .map_err(|error| store_error("failed to read the cart id", error))?;
        let status: String = row
            .try_get("status")
            .map_err(|error| store_error("failed to read the cart status", error))?;
        let currency_code: String = row
            .try_get("currency_code")
            .map_err(|error| store_error("failed to read the cart currency", error))?;
        Ok((id, status, currency_code))
    }

    async fn load_cart(
        &self,
        cart_id: &str,
        status: &str,
        currency_code: &str,
    ) -> Result<CartView, CommerceServiceError> {
        let rows = sqlx::query(
            r#"
            SELECT
                i.id AS item_id,
                i.sku_id,
                i.spu_id,
                i.shop_id,
                i.quantity,
                i.product_title,
                i.sku_name,
                i.image_url,
                i.unit_price_minor,
                i.price_scale,
                i.currency_code,
                s.title AS sku_title,
                s.name AS sku_display_name
            FROM commerce_cart_item i
            LEFT JOIN commerce_product_sku s
                ON s.id::text = i.sku_id
               AND s.tenant_id::text = i.tenant_id
               AND s.deleted_at IS NULL
            WHERE i.cart_id = $1
            ORDER BY i.created_at ASC, i.id ASC
            "#,
        )
        .bind(cart_id)
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to read the cart lines", error))?;

        let items = rows
            .iter()
            .map(map_cart_line_row)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(CartView {
            cart_id: cart_id.to_owned(),
            status: status.to_owned(),
            currency_code: currency_code.to_owned(),
            items,
        })
    }

    async fn load_cart_line(
        &self,
        cart_item_id: &str,
    ) -> Result<CartLineView, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT
                i.id AS item_id,
                i.sku_id,
                i.spu_id,
                i.shop_id,
                i.quantity,
                i.product_title,
                i.sku_name,
                i.image_url,
                i.unit_price_minor,
                i.price_scale,
                i.currency_code,
                s.title AS sku_title,
                s.name AS sku_display_name
            FROM commerce_cart_item i
            LEFT JOIN commerce_product_sku s
                ON s.id::text = i.sku_id
               AND s.tenant_id::text = i.tenant_id
               AND s.deleted_at IS NULL
            WHERE i.id = $1
            "#,
        )
        .bind(cart_item_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to read the cart line", error))?
        .ok_or_else(|| CommerceServiceError::not_found("cart item was not found"))?;

        map_cart_line_row(&row)
    }

    async fn load_sku_snapshot(
        &self,
        tenant_id: &str,
        sku_id: &str,
    ) -> Result<Option<SkuSnapshot>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT
                spu_id::text AS spu_id,
                COALESCE(NULLIF(title, ''), NULLIF(name, ''), '商品') AS title,
                COALESCE(NULLIF(name, ''), NULLIF(title, ''), '') AS sku_name,
                sale_price_minor,
                price_scale,
                currency_code,
                min_order_quantity,
                max_order_quantity
            FROM commerce_product_sku
            WHERE id::text = $1
              AND tenant_id::text = $2
              AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(sku_id)
        .bind(tenant_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to read the cart sku", error))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let title: String = row
            .try_get("title")
            .map_err(|error| store_error("failed to read the sku title", error))?;
        let sku_name: String = row
            .try_get("sku_name")
            .map_err(|error| store_error("failed to read the sku name", error))?;
        let spu_id: Option<String> = row
            .try_get("spu_id")
            .map_err(|error| store_error("failed to read the sku product id", error))?;
        let unit_price_minor: i64 = row
            .try_get("sale_price_minor")
            .map_err(|error| store_error("failed to read the sku price", error))?;
        let price_scale: i16 = row
            .try_get("price_scale")
            .map_err(|error| store_error("failed to read the sku price scale", error))?;
        let currency_code: String = row
            .try_get("currency_code")
            .map_err(|error| store_error("failed to read the sku currency", error))?;
        let min_order_quantity: i64 = row.try_get("min_order_quantity").unwrap_or(1);
        let max_order_quantity: Option<i64> = row.try_get("max_order_quantity").ok().flatten();

        Ok(Some(SkuSnapshot {
            spu_id,
            title,
            sku_name,
            unit_price_minor,
            price_scale,
            currency_code,
            min_order_quantity,
            max_order_quantity,
        }))
    }
}

impl CommerceCartStore for PostgresCartStore {
    fn retrieve_current_cart<'a>(
        &'a self,
        query: CurrentCartQuery,
    ) -> CartStoreFuture<'a, CartView> {
        Box::pin(async move { PostgresCartStore::retrieve_current_cart(self, query).await })
    }

    fn create_cart_item<'a>(
        &'a self,
        command: CreateCartItemCommand,
    ) -> CartStoreFuture<'a, CartLineView> {
        Box::pin(async move { PostgresCartStore::create_cart_item(self, command).await })
    }

    fn update_cart_item<'a>(
        &'a self,
        command: UpdateCartItemCommand,
    ) -> CartStoreFuture<'a, CartLineView> {
        Box::pin(async move { PostgresCartStore::update_cart_item(self, command).await })
    }

    fn delete_cart_item<'a>(&'a self, command: DeleteCartItemCommand) -> CartStoreFuture<'a, ()> {
        Box::pin(async move { PostgresCartStore::delete_cart_item(self, command).await })
    }
}

fn validate_against_sku(quantity: i64, snapshot: &SkuSnapshot) -> Result<(), CommerceServiceError> {
    if quantity < snapshot.min_order_quantity {
        return Err(CommerceServiceError::validation(format!(
            "cart item quantity must be at least {}",
            snapshot.min_order_quantity
        )));
    }
    if let Some(max_order_quantity) = snapshot.max_order_quantity {
        if quantity > max_order_quantity {
            return Err(CommerceServiceError::validation(format!(
                "cart item quantity must not exceed {max_order_quantity}"
            )));
        }
    }
    Ok(())
}

fn map_cart_line_row(row: &sqlx::postgres::PgRow) -> Result<CartLineView, CommerceServiceError> {
    let product_title: Option<String> = row.try_get("product_title").ok().flatten();
    let sku_title: Option<String> = row.try_get("sku_title").ok().flatten();
    let sku_display_name: Option<String> = row.try_get("sku_display_name").ok().flatten();
    let stored_sku_name: Option<String> = row.try_get("sku_name").ok().flatten();

    let title = product_title
        .or(sku_title)
        .or_else(|| sku_display_name.clone())
        .unwrap_or_else(|| "商品".to_owned());
    let sku_name = stored_sku_name.or(sku_display_name);

    let unit_price_minor: i64 = row
        .try_get("unit_price_minor")
        .map_err(|error| store_error("failed to read the cart line price", error))?;
    let price_scale: i16 = row
        .try_get("price_scale")
        .map_err(|error| store_error("failed to read the cart line price scale", error))?;

    Ok(CartLineView {
        cart_item_id: string_cell(row, "item_id")?,
        sku_id: string_cell(row, "sku_id")?,
        spu_id: row.try_get("spu_id").ok().flatten(),
        shop_id: row.try_get("shop_id").ok().flatten(),
        quantity: row
            .try_get("quantity")
            .map_err(|error| store_error("failed to read the cart line quantity", error))?,
        title,
        sku_name,
        image_url: row.try_get("image_url").ok().flatten(),
        unit_price: CommerceMoney::new(&unit_price_minor.to_string()).map_err(|_| {
            CommerceServiceError::invalid_state("cart line unit price is not minor units")
        })?,
        price_scale,
        currency_code: string_cell(row, "currency_code")?,
    })
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Result<String, CommerceServiceError> {
    row.try_get(column)
        .map_err(|error| store_error(&format!("failed to read the cart column {column}"), error))
}

fn store_error(context: &str, error: sqlx::Error) -> CommerceServiceError {
    map_sql_store_error(context, error)
}
