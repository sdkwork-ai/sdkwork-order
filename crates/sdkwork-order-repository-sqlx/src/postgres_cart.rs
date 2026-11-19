//! PostgreSQL persistence for the buyer cart.
//!
//! # Transaction and concurrency model
//!
//! Every mutation runs inside one transaction that first resolves 鈥?and locks 鈥?//! the buyer's live cart row (`SELECT ... FOR UPDATE`). That single lock is the
//! serialization point for the cart: the line cap, the per-SKU quantity cap, the
//! currency agreement, and the read-after-write all execute while no other writer
//! can touch the same cart, so validation cannot be invalidated between the check
//! and the write. Carts are per buyer, so the lock never couples two buyers.
//!
//! The database also enforces the invariants on its own (`CHECK quantity BETWEEN
//! 1 AND 999`, `unit_price_minor >= 0`, one line per `(cart_id, sku_id)`, cascade
//! delete), so a future writer that bypasses this module still cannot persist a
//! state the totals would mis-parse.
//!
//! # Retry safety
//!
//! `POST /cart/items` is a write command with an optional `Idempotency-Key`. When
//! the header is present the add records the line in `commerce_idempotency_key`
//! inside the same transaction, so a retry replays the recorded line instead of
//! incrementing the quantity twice. A rolled-back attempt leaves no record and is
//! therefore freely retryable.

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_order_service::{
    stable_command_request_hash, CartItemMutation, CartLineView, CartStoreFuture, CartView,
    CommerceCartStore, ConsumeCartItemsCommand, CreateCartItemCommand, CurrentCartQuery,
    DeleteCartItemCommand, UpdateCartItemCommand, CART_DEFAULT_CURRENCY_CODE,
    CART_ITEM_CREATE_IDEMPOTENCY_SCOPE, CART_ITEM_MAX_QUANTITY, CART_MAX_LINES,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::sql_store_error::map_sql_store_error;
use crate::store_clock::now_canonical;

/// The live cart a buyer currently owns, resolved under a row lock.
struct LockedCart {
    cart_id: String,
    status: String,
    currency_code: String,
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
    sellable: bool,
}

/// Persisted replay payload for an idempotent add.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredCartLine {
    id: String,
    sku_id: String,
    spu_id: Option<String>,
    quantity: i64,
    selected: bool,
    title: String,
    sku_name: Option<String>,
    image_url: Option<String>,
    unit_price_minor: String,
    price_scale: i16,
    currency_code: String,
    available: bool,
    current_unit_price_minor: Option<String>,
    available_stock: Option<i64>,
    stock_shortage: bool,
    price_changed: bool,
}

impl StoredCartLine {
    fn from_view(line: &CartLineView) -> Self {
        Self {
            id: line.cart_item_id.clone(),
            sku_id: line.sku_id.clone(),
            spu_id: line.spu_id.clone(),
            quantity: line.quantity,
            selected: line.selected,
            title: line.title.clone(),
            sku_name: line.sku_name.clone(),
            image_url: line.image_url.clone(),
            unit_price_minor: line.unit_price.as_str().to_owned(),
            price_scale: line.price_scale,
            currency_code: line.currency_code.clone(),
            available: line.available,
            current_unit_price_minor: line
                .current_unit_price
                .as_ref()
                .map(|money| money.as_str().to_owned()),
            price_changed: line.price_changed,
            available_stock: line.available_stock,
            stock_shortage: line.stock_shortage,
        }
    }

    fn into_view(self) -> Result<CartLineView, CommerceServiceError> {
        Ok(CartLineView {
            cart_item_id: self.id,
            sku_id: self.sku_id,
            spu_id: self.spu_id,
            quantity: self.quantity,
            selected: self.selected,
            title: self.title,
            sku_name: self.sku_name,
            image_url: self.image_url,
            unit_price: parse_minor_units(&self.unit_price_minor)?,
            price_scale: self.price_scale,
            currency_code: self.currency_code,
            available: self.available,
            current_unit_price: self
                .current_unit_price_minor
                .as_deref()
                .map(parse_minor_units)
                .transpose()?,
            price_changed: self.price_changed,
            available_stock: self.available_stock,
            stock_shortage: self.stock_shortage,
        })
    }
}

/// The authoritative cart store over the shared PostgreSQL pool.
#[derive(Debug, Clone)]
pub struct PostgresCartStore {
    pool: PgPool,
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
        let mut tx = self.begin().await?;
        let cart = lock_active_cart(
            &mut tx,
            &query.tenant_id,
            query.organization_id.as_deref(),
            &query.owner_user_id,
        )
        .await?;
        let items = load_cart_lines(&mut tx, &cart.cart_id).await?;
        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit the cart read", error))?;

        Ok(CartView {
            cart_id: cart.cart_id,
            status: cart.status,
            currency_code: cart.currency_code,
            items,
        })
    }

    pub async fn create_cart_item(
        &self,
        command: CreateCartItemCommand,
    ) -> Result<CartItemMutation, CommerceServiceError> {
        let mut tx = self.begin().await?;

        if let Some(idempotency_key) = command.idempotency_key.as_deref() {
            if let Some(line) = resume_idempotent_add(&mut tx, &command, idempotency_key).await? {
                tx.commit()
                    .await
                    .map_err(|error| store_error("failed to commit the cart replay", error))?;
                return Ok(CartItemMutation::replayed(line));
            }
        }

        let cart = lock_active_cart(
            &mut tx,
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
        )
        .await?;
        let sku = load_sku_snapshot(&mut tx, &command.tenant_id, &command.sku_id)
            .await?
            .ok_or_else(|| CommerceServiceError::not_found("cart sku was not found"))?;
        if !sku.sellable {
            return Err(CommerceServiceError::validation(
                "cart sku is not available for sale",
            ));
        }

        align_cart_currency(&mut tx, &cart, &sku).await?;

        let existing_quantity =
            load_line_quantity_for_sku(&mut tx, &cart.cart_id, &command.sku_id).await?;
        let resulting_quantity = match existing_quantity {
            Some(quantity) => quantity.checked_add(command.quantity).ok_or_else(|| {
                CommerceServiceError::validation("cart item quantity is too large")
            })?,
            None => {
                let line_count = count_cart_lines(&mut tx, &cart.cart_id).await?;
                if line_count >= CART_MAX_LINES {
                    return Err(CommerceServiceError::conflict(format!(
                        "cart line limit of {CART_MAX_LINES} was reached"
                    )));
                }
                command.quantity
            }
        };
        validate_resulting_quantity(resulting_quantity, &sku)?;

        let cart_item_id = upsert_cart_line(&mut tx, &cart.cart_id, &command, &sku).await?;
        touch_cart(&mut tx, &cart.cart_id).await?;
        let line = load_cart_line(&mut tx, &cart_item_id).await?;

        if let Some(idempotency_key) = command.idempotency_key.as_deref() {
            complete_idempotent_add(&mut tx, &command, idempotency_key, &line).await?;
        }

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit the cart line", error))?;
        Ok(CartItemMutation::created(line))
    }

    pub async fn update_cart_item(
        &self,
        command: UpdateCartItemCommand,
    ) -> Result<CartItemMutation, CommerceServiceError> {
        let mut tx = self.begin().await?;
        let cart = lock_active_cart(
            &mut tx,
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
        )
        .await?;

        let current = load_cart_line(&mut tx, &command.cart_item_id).await?;
        if !cart_item_belongs_to(&mut tx, &cart.cart_id, &command.cart_item_id).await? {
            return Err(CommerceServiceError::not_found("cart item was not found"));
        }

        if let Some(quantity) = command.quantity {
            let sku = load_sku_snapshot(&mut tx, &command.tenant_id, &current.sku_id).await?;
            let sellable = sku.as_ref().map(|sku| sku.sellable).unwrap_or(false);
            if !sellable {
                return Err(CommerceServiceError::validation(
                    "cart sku is not available for sale",
                ));
            }
            if let Some(sku) = sku.as_ref() {
                validate_resulting_quantity(quantity, sku)?;
            }
        }

        update_cart_line(
            &mut tx,
            &command.cart_item_id,
            command.quantity,
            command.selected,
        )
        .await?;
        touch_cart(&mut tx, &cart.cart_id).await?;
        let line = load_cart_line(&mut tx, &command.cart_item_id).await?;

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit the cart line update", error))?;
        Ok(CartItemMutation::created(line))
    }

    pub async fn delete_cart_item(
        &self,
        command: DeleteCartItemCommand,
    ) -> Result<(), CommerceServiceError> {
        let mut tx = self.begin().await?;
        let cart = lock_active_cart(
            &mut tx,
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
        )
        .await?;

        let deleted = sqlx::query(
            r#"
            DELETE FROM commerce_cart_item
            WHERE id = $1 AND cart_id = $2
            RETURNING id
            "#,
        )
        .bind(&command.cart_item_id)
        .bind(&cart.cart_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to remove a cart line", error))?;
        if deleted.is_none() {
            return Err(CommerceServiceError::not_found("cart item was not found"));
        }

        touch_cart(&mut tx, &cart.cart_id).await?;
        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit the cart line removal", error))?;
        Ok(())
    }

    /// Removes the lines a checkout session consumed in one statement, under the
    /// same cart lock as every other mutation.
    pub async fn consume_cart_items(
        &self,
        command: ConsumeCartItemsCommand,
    ) -> Result<u32, CommerceServiceError> {
        let mut tx = self.begin().await?;
        let cart = lock_active_cart(
            &mut tx,
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
        )
        .await?;

        let removed = sqlx::query(
            r#"
            DELETE FROM commerce_cart_item
            WHERE cart_id = $1 AND id = ANY($2)
            RETURNING id
            "#,
        )
        .bind(&cart.cart_id)
        .bind(&command.cart_item_ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(|error| store_error("failed to consume the cart lines", error))?;
        if !removed.is_empty() {
            touch_cart(&mut tx, &cart.cart_id).await?;
        }

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit the cart consumption", error))?;
        Ok(removed.len() as u32)
    }

    /// Moves idle live carts to `expired` so an abandoned cart cannot hold the
    /// buyer's single active slot forever.
    ///
    /// Bounded by `limit` and ordered oldest-first, so one sweep is a small,
    /// predictable statement and a backlog drains across ticks instead of
    /// locking a large row set. Idempotent: an already expired cart is not a
    /// candidate again.
    pub async fn expire_idle_carts(
        &self,
        idle_days: i32,
        limit: i64,
    ) -> Result<u32, CommerceServiceError> {
        let idle_days = idle_days.max(1);
        let limit = limit.clamp(1, 5_000);

        let rows = sqlx::query(
            r#"
            UPDATE commerce_cart
            SET status = 'expired', updated_at = now()
            WHERE id IN (
                SELECT id
                FROM commerce_cart
                WHERE status = 'active'
                  AND updated_at < now() - make_interval(days => $1)
                ORDER BY updated_at ASC
                LIMIT $2
            )
            RETURNING id
            "#,
        )
        .bind(idle_days)
        .bind(limit)
        .fetch_all(self.pool())
        .await
        .map_err(|error| store_error("failed to expire idle carts", error))?;

        Ok(rows.len() as u32)
    }

    async fn begin(&self) -> Result<Transaction<'_, Postgres>, CommerceServiceError> {
        self.pool()
            .begin()
            .await
            .map_err(|error| store_error("failed to begin a cart transaction", error))
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
    ) -> CartStoreFuture<'a, CartItemMutation> {
        Box::pin(async move { PostgresCartStore::create_cart_item(self, command).await })
    }

    fn update_cart_item<'a>(
        &'a self,
        command: UpdateCartItemCommand,
    ) -> CartStoreFuture<'a, CartItemMutation> {
        Box::pin(async move { PostgresCartStore::update_cart_item(self, command).await })
    }

    fn delete_cart_item<'a>(&'a self, command: DeleteCartItemCommand) -> CartStoreFuture<'a, ()> {
        Box::pin(async move { PostgresCartStore::delete_cart_item(self, command).await })
    }

    fn consume_cart_items<'a>(
        &'a self,
        command: ConsumeCartItemsCommand,
    ) -> CartStoreFuture<'a, u32> {
        Box::pin(async move { PostgresCartStore::consume_cart_items(self, command).await })
    }
}

/// Resolves the buyer's live cart and holds a row lock on it for the rest of the
/// transaction.
async fn lock_active_cart(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
) -> Result<LockedCart, CommerceServiceError> {
    sqlx::query(
        r#"
        INSERT INTO commerce_cart
            (id, tenant_id, organization_id, owner_user_id, status, currency_code)
        VALUES
            ($1, $2, $3, $4, 'active', $5)
        ON CONFLICT (tenant_id, owner_user_id) WHERE status = 'active' DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(tenant_id)
    .bind(organization_id.unwrap_or("0"))
    .bind(owner_user_id)
    .bind(CART_DEFAULT_CURRENCY_CODE)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to open the buyer cart", error))?;

    let row = sqlx::query(
        r#"
        SELECT id, status, currency_code
        FROM commerce_cart
        WHERE tenant_id = $1
          AND owner_user_id = $2
          AND status = 'active'
        FOR UPDATE
        "#,
    )
    .bind(tenant_id)
    .bind(owner_user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to lock the buyer cart", error))?
    .ok_or_else(|| CommerceServiceError::invalid_state("the buyer cart could not be resolved"))?;

    Ok(LockedCart {
        cart_id: string_cell(&row, "id")?,
        status: string_cell(&row, "status")?,
        currency_code: string_cell(&row, "currency_code")?,
    })
}

/// Keeps one currency per cart. The first line fixes it; a later line in another
/// currency is a conflict rather than a silently mixed total.
async fn align_cart_currency(
    tx: &mut Transaction<'_, Postgres>,
    cart: &LockedCart,
    sku: &SkuSnapshot,
) -> Result<(), CommerceServiceError> {
    if cart.currency_code.eq_ignore_ascii_case(&sku.currency_code) {
        return Ok(());
    }
    let line_count = count_cart_lines(tx, &cart.cart_id).await?;
    if line_count > 0 {
        return Err(CommerceServiceError::conflict(format!(
            "cart currency {} does not match the sku currency {}",
            cart.currency_code, sku.currency_code
        )));
    }

    sqlx::query(
        r#"
        UPDATE commerce_cart
        SET currency_code = $2, updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(&cart.cart_id)
    .bind(&sku.currency_code)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to align the cart currency", error))?;
    Ok(())
}

async fn count_cart_lines(
    tx: &mut Transaction<'_, Postgres>,
    cart_id: &str,
) -> Result<i64, CommerceServiceError> {
    let row =
        sqlx::query("SELECT count(*) AS line_count FROM commerce_cart_item WHERE cart_id = $1")
            .bind(cart_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|error| store_error("failed to count the cart lines", error))?;
    row.try_get("line_count")
        .map_err(|error| store_error("failed to read the cart line count", error))
}

async fn load_line_quantity_for_sku(
    tx: &mut Transaction<'_, Postgres>,
    cart_id: &str,
    sku_id: &str,
) -> Result<Option<i64>, CommerceServiceError> {
    let row = sqlx::query(
        "SELECT quantity FROM commerce_cart_item WHERE cart_id = $1 AND sku_id = $2 LIMIT 1",
    )
    .bind(cart_id)
    .bind(sku_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to read the existing cart line", error))?;
    match row {
        Some(row) => row
            .try_get("quantity")
            .map(Some)
            .map_err(|error| store_error("failed to read the cart line quantity", error)),
        None => Ok(None),
    }
}

async fn cart_item_belongs_to(
    tx: &mut Transaction<'_, Postgres>,
    cart_id: &str,
    cart_item_id: &str,
) -> Result<bool, CommerceServiceError> {
    let row =
        sqlx::query("SELECT 1 AS present FROM commerce_cart_item WHERE id = $1 AND cart_id = $2")
            .bind(cart_item_id)
            .bind(cart_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|error| store_error("failed to verify the cart line ownership", error))?;
    Ok(row.is_some())
}

async fn upsert_cart_line(
    tx: &mut Transaction<'_, Postgres>,
    cart_id: &str,
    command: &CreateCartItemCommand,
    sku: &SkuSnapshot,
) -> Result<String, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        INSERT INTO commerce_cart_item
            (id, tenant_id, organization_id, cart_id, sku_id, spu_id, quantity, selected,
             product_title, sku_name, unit_price_minor, price_scale, currency_code)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7, TRUE, $8, $9, $10, $11, $12)
        ON CONFLICT (cart_id, sku_id) DO UPDATE
        SET quantity = commerce_cart_item.quantity + EXCLUDED.quantity,
            spu_id = COALESCE(EXCLUDED.spu_id, commerce_cart_item.spu_id),
            product_title = EXCLUDED.product_title,
            sku_name = EXCLUDED.sku_name,
            unit_price_minor = EXCLUDED.unit_price_minor,
            price_scale = EXCLUDED.price_scale,
            currency_code = EXCLUDED.currency_code,
            updated_at = now()
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&command.tenant_id)
    .bind(command.organization_id.as_deref().unwrap_or("0"))
    .bind(cart_id)
    .bind(&command.sku_id)
    .bind(sku.spu_id.as_deref())
    .bind(command.quantity)
    .bind(&sku.title)
    .bind(&sku.sku_name)
    .bind(sku.unit_price_minor)
    .bind(sku.price_scale)
    .bind(&sku.currency_code)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| store_error("failed to add a cart line", error))?;

    string_cell(&row, "id")
}

async fn update_cart_line(
    tx: &mut Transaction<'_, Postgres>,
    cart_item_id: &str,
    quantity: Option<i64>,
    selected: Option<bool>,
) -> Result<(), CommerceServiceError> {
    let updated = sqlx::query(
        r#"
        UPDATE commerce_cart_item
        SET quantity = COALESCE($2, quantity),
            selected = COALESCE($3, selected),
            updated_at = now()
        WHERE id = $1
        RETURNING id
        "#,
    )
    .bind(cart_item_id)
    .bind(quantity)
    .bind(selected)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to update the cart line", error))?;
    if updated.is_none() {
        return Err(CommerceServiceError::not_found("cart item was not found"));
    }
    Ok(())
}

async fn touch_cart(
    tx: &mut Transaction<'_, Postgres>,
    cart_id: &str,
) -> Result<(), CommerceServiceError> {
    sqlx::query("UPDATE commerce_cart SET updated_at = now() WHERE id = $1")
        .bind(cart_id)
        .execute(&mut **tx)
        .await
        .map_err(|error| store_error("failed to touch the buyer cart", error))?;
    Ok(())
}

async fn load_sku_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    sku_id: &str,
) -> Result<Option<SkuSnapshot>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT
            spu_id::text AS spu_id,
            COALESCE(NULLIF(title, ''), NULLIF(name, ''), '鍟嗗搧') AS title,
            COALESCE(NULLIF(name, ''), NULLIF(title, ''), '') AS sku_name,
            sale_price_minor,
            price_scale,
            currency_code,
            min_order_quantity,
            max_order_quantity,
            (status = 'active' AND sales_status = 'active') AS sellable
        FROM commerce_product_sku
        WHERE id::text = $1
          AND tenant_id::text = $2
          AND deleted_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to read the cart sku", error))?;

    let Some(row) = row else {
        return Ok(None);
    };

    Ok(Some(SkuSnapshot {
        spu_id: row.try_get("spu_id").ok().flatten(),
        title: string_cell(&row, "title")?,
        sku_name: string_cell(&row, "sku_name")?,
        unit_price_minor: row
            .try_get("sale_price_minor")
            .map_err(|error| store_error("failed to read the sku price", error))?,
        price_scale: row
            .try_get("price_scale")
            .map_err(|error| store_error("failed to read the sku price scale", error))?,
        currency_code: string_cell(&row, "currency_code")?,
        min_order_quantity: row.try_get("min_order_quantity").unwrap_or(1),
        max_order_quantity: row.try_get("max_order_quantity").ok().flatten(),
        sellable: row.try_get("sellable").unwrap_or(false),
    }))
}

async fn load_cart_lines(
    tx: &mut Transaction<'_, Postgres>,
    cart_id: &str,
) -> Result<Vec<CartLineView>, CommerceServiceError> {
    let rows = sqlx::query(CART_LINE_LIST_SQL)
        .bind(cart_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|error| store_error("failed to read the cart lines", error))?;

    rows.iter().map(map_cart_line_row).collect()
}

async fn load_cart_line(
    tx: &mut Transaction<'_, Postgres>,
    cart_item_id: &str,
) -> Result<CartLineView, CommerceServiceError> {
    let row = sqlx::query(CART_LINE_SINGLE_SQL)
        .bind(cart_item_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| store_error("failed to read the cart line", error))?
        .ok_or_else(|| CommerceServiceError::not_found("cart item was not found"))?;

    map_cart_line_row(&row)
}

const CART_LINE_LIST_SQL: &str = r#"
        SELECT
            i.id AS item_id,
            i.sku_id,
            i.spu_id,
            i.quantity,
            i.selected,
            i.product_title,
            i.sku_name,
            i.image_url,
            i.unit_price_minor,
            i.price_scale,
            i.currency_code,
            (s.id IS NOT NULL) AS sku_found,
            COALESCE(s.status = 'active' AND s.sales_status = 'active', FALSE) AS sellable,
            s.sale_price_minor AS current_unit_price_minor,
            stock.available AS available_stock
        FROM commerce_cart_item i
        LEFT JOIN commerce_product_sku s
            ON s.id::text = i.sku_id
           AND s.tenant_id::text = i.tenant_id
           AND s.deleted_at IS NULL
        LEFT JOIN LATERAL (
            SELECT SUM(st.available_quantity) AS available
            FROM commerce_inventory_stock st
            WHERE st.tenant_id = i.tenant_id
              AND st.sku_id = i.sku_id
              AND st.status = 'active'
        ) stock ON TRUE
        WHERE i.cart_id = $1
        ORDER BY i.created_at ASC, i.id ASC
"#;

const CART_LINE_SINGLE_SQL: &str = r#"
        SELECT
            i.id AS item_id,
            i.sku_id,
            i.spu_id,
            i.quantity,
            i.selected,
            i.product_title,
            i.sku_name,
            i.image_url,
            i.unit_price_minor,
            i.price_scale,
            i.currency_code,
            (s.id IS NOT NULL) AS sku_found,
            COALESCE(s.status = 'active' AND s.sales_status = 'active', FALSE) AS sellable,
            s.sale_price_minor AS current_unit_price_minor,
            stock.available AS available_stock
        FROM commerce_cart_item i
        LEFT JOIN commerce_product_sku s
            ON s.id::text = i.sku_id
           AND s.tenant_id::text = i.tenant_id
           AND s.deleted_at IS NULL
        LEFT JOIN LATERAL (
            SELECT SUM(st.available_quantity) AS available
            FROM commerce_inventory_stock st
            WHERE st.tenant_id = i.tenant_id
              AND st.sku_id = i.sku_id
              AND st.status = 'active'
        ) stock ON TRUE
        WHERE i.id = $1
"#;

async fn resume_idempotent_add(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateCartItemCommand,
    idempotency_key: &str,
) -> Result<Option<CartLineView>, CommerceServiceError> {
    let request_hash = cart_item_request_hash(command);
    let claimed = sqlx::query(
        r#"
        INSERT INTO commerce_idempotency_key
            (id, tenant_id, organization_id, scope, idempotency_key, request_hash,
             status, locked_until, expires_at, created_at, updated_at)
        VALUES
            ($1, $2, $3, $4, $5, $6, 'locked', $7, $7, $7, $7)
        ON CONFLICT (tenant_id, scope, idempotency_key) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&command.tenant_id)
    .bind(command.organization_id.as_deref().unwrap_or("0"))
    .bind(CART_ITEM_CREATE_IDEMPOTENCY_SCOPE)
    .bind(idempotency_key)
    .bind(&request_hash)
    .bind(now_canonical())
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to claim the cart idempotency key", error))?;

    if claimed.rows_affected() == 1 {
        return Ok(None);
    }

    let row = sqlx::query(
        r#"
        SELECT request_hash, response_json, status
        FROM commerce_idempotency_key
        WHERE tenant_id = $1 AND scope = $2 AND idempotency_key = $3
        FOR UPDATE
        "#,
    )
    .bind(&command.tenant_id)
    .bind(CART_ITEM_CREATE_IDEMPOTENCY_SCOPE)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to read the cart idempotency record", error))?
    .ok_or_else(|| {
        CommerceServiceError::invalid_state("the cart idempotency record could not be resolved")
    })?;

    let stored_hash = string_cell(&row, "request_hash")?;
    if stored_hash != request_hash {
        return Err(CommerceServiceError::conflict(
            "Idempotency-Key was reused with a different cart request",
        ));
    }
    let status = string_cell(&row, "status")?;
    if status != "completed" {
        return Err(CommerceServiceError::conflict(
            "a cart request with this Idempotency-Key is still in progress",
        ));
    }
    let response_json: Option<String> = row.try_get("response_json").ok().flatten();
    let response_json = response_json.ok_or_else(|| {
        CommerceServiceError::invalid_state("the cart idempotency record has no response")
    })?;
    let stored: StoredCartLine = serde_json::from_str(&response_json).map_err(|_| {
        CommerceServiceError::invalid_state("the cart idempotency response is not readable")
    })?;

    Ok(Some(stored.into_view()?))
}

async fn complete_idempotent_add(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateCartItemCommand,
    idempotency_key: &str,
    line: &CartLineView,
) -> Result<(), CommerceServiceError> {
    let response_json = serde_json::to_string(&StoredCartLine::from_view(line)).map_err(|_| {
        CommerceServiceError::invalid_state("the cart replay payload is not writable")
    })?;
    let now = now_canonical();

    sqlx::query(
        r#"
        UPDATE commerce_idempotency_key
        SET status = 'completed',
            response_json = $4,
            locked_until = NULL,
            expires_at = CAST($5 AS TIMESTAMPTZ),
            updated_at = CAST($5 AS TIMESTAMPTZ)
        WHERE tenant_id = $1 AND scope = $2 AND idempotency_key = $3
        "#,
    )
    .bind(&command.tenant_id)
    .bind(CART_ITEM_CREATE_IDEMPOTENCY_SCOPE)
    .bind(idempotency_key)
    .bind(response_json)
    .bind(&now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to record the cart idempotent response", error))?;
    Ok(())
}

fn cart_item_request_hash(command: &CreateCartItemCommand) -> String {
    stable_command_request_hash(
        CART_ITEM_CREATE_IDEMPOTENCY_SCOPE,
        &[
            &command.tenant_id,
            &command.owner_user_id,
            &command.sku_id,
            &command.quantity.to_string(),
        ],
    )
}

fn validate_resulting_quantity(
    quantity: i64,
    sku: &SkuSnapshot,
) -> Result<(), CommerceServiceError> {
    if quantity > CART_ITEM_MAX_QUANTITY {
        return Err(CommerceServiceError::validation(format!(
            "cart item quantity must not exceed {CART_ITEM_MAX_QUANTITY}"
        )));
    }
    if quantity < sku.min_order_quantity {
        return Err(CommerceServiceError::validation(format!(
            "cart item quantity must be at least {}",
            sku.min_order_quantity
        )));
    }
    if let Some(max_order_quantity) = sku.max_order_quantity {
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
    let sku_name: Option<String> = row.try_get("sku_name").ok().flatten();
    let sku_found: bool = row.try_get("sku_found").unwrap_or(false);
    let sellable: bool = row.try_get("sellable").unwrap_or(false);
    let unit_price_minor: i64 = row
        .try_get("unit_price_minor")
        .map_err(|error| store_error("failed to read the cart line price", error))?;
    let price_scale: i16 = row
        .try_get("price_scale")
        .map_err(|error| store_error("failed to read the cart line price scale", error))?;
    let available_stock: Option<i64> = row.try_get("available_stock").ok().flatten();
    let current_unit_price_minor: Option<i64> =
        row.try_get("current_unit_price_minor").ok().flatten();

    Ok(CartLineView {
        cart_item_id: string_cell(row, "item_id")?,
        sku_id: string_cell(row, "sku_id")?,
        spu_id: row.try_get("spu_id").ok().flatten(),
        quantity: row
            .try_get("quantity")
            .map_err(|error| store_error("failed to read the cart line quantity", error))?,
        selected: row.try_get("selected").unwrap_or(true),
        title: product_title
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "鍟嗗搧".to_owned()),
        sku_name,
        image_url: row.try_get("image_url").ok().flatten(),
        unit_price: parse_minor_units(&unit_price_minor.to_string())?,
        price_scale,
        currency_code: string_cell(row, "currency_code")?,
        available: sku_found && sellable,
        current_unit_price: current_unit_price_minor
            .map(|minor| parse_minor_units(&minor.to_string()))
            .transpose()?,
        available_stock,
        stock_shortage: available_stock
            .map(|stock| stock < row.try_get::<i64, _>("quantity").unwrap_or(0))
            .unwrap_or(false),
        price_changed: current_unit_price_minor
            .map(|minor| minor != unit_price_minor)
            .unwrap_or(false),
    })
}

fn parse_minor_units(value: &str) -> Result<CommerceMoney, CommerceServiceError> {
    CommerceMoney::new(value)
        .map_err(|_| CommerceServiceError::invalid_state("cart line price is not minor units"))
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Result<String, CommerceServiceError> {
    row.try_get(column)
        .map_err(|error| store_error(&format!("failed to read the cart column {column}"), error))
}

fn store_error(context: &str, error: sqlx::Error) -> CommerceServiceError {
    map_sql_store_error(context, error)
}
