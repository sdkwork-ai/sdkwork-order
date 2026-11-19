//! Buyer cart domain rules.
//!
//! The cart is the buyer-owned staging area in front of checkout: a cart line
//! snapshots the merchandise SKU (title, unit price, currency, price scale) at
//! add time so a later catalog price change never rewrites what the buyer already
//! saw. The checkout session stays the authority for what is actually charged;
//! the cart read model exposes the live catalog price next to the snapshot so the
//! storefront can tell the buyer the price moved before checkout.
//!
//! Every query and command carries the resolved subject (`tenant_id`,
//! `organization_id`, `owner_user_id`); nothing here reads an ambient context.

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

/// Status of the single live cart a buyer owns.
pub const CART_STATUS_ACTIVE: &str = "active";

/// Upper bound for one line. Larger quantities are a client defect, not a sale.
pub const CART_ITEM_MAX_QUANTITY: i64 = 999;

/// Upper bound for distinct lines in one cart. Bounds the read cost, the checkout
/// fan-out, and abuse of the add endpoint.
pub const CART_MAX_LINES: i64 = 100;

/// Default currency when the buyer's first line has not named one yet.
pub const CART_DEFAULT_CURRENCY_CODE: &str = "CNY";

/// Idempotency scope for `POST /app/v3/api/cart/items`.
pub const CART_ITEM_CREATE_IDEMPOTENCY_SCOPE: &str = "order.cart.items.create";

/// One hydrated cart line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CartLineView {
    pub cart_item_id: String,
    pub sku_id: String,
    pub spu_id: Option<String>,
    pub quantity: i64,
    pub selected: bool,
    pub title: String,
    pub sku_name: Option<String>,
    pub image_url: Option<String>,
    /// Price captured when the line was added; the authoritative cart amount.
    pub unit_price: CommerceMoney,
    pub price_scale: i16,
    pub currency_code: String,
    /// False when the SKU was unpublished, archived, or deleted after the add.
    pub available: bool,
    /// Live catalog price, when the SKU still resolves.
    pub current_unit_price: Option<CommerceMoney>,
    /// True when the live catalog price differs from the captured snapshot.
    pub price_changed: bool,
}

impl CartLineView {
    /// Line total in minor units. The snapshot price is authoritative, so the
    /// multiplication is the only arithmetic and it is overflow-checked.
    pub fn line_amount_minor(&self) -> Result<i64, CommerceServiceError> {
        let unit_price_minor = self.unit_price.as_str().parse::<i64>().map_err(|_| {
            CommerceServiceError::invalid_state("cart line unit price is not minor units")
        })?;
        unit_price_minor
            .checked_mul(self.quantity)
            .ok_or_else(|| CommerceServiceError::validation("cart line amount is too large"))
    }

    pub fn line_total(&self) -> Result<CommerceMoney, CommerceServiceError> {
        Ok(minor_units_to_money(self.line_amount_minor()?))
    }
}

/// Result of one line mutation plus whether it replayed a recorded idempotent add.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CartItemMutation {
    pub line: CartLineView,
    pub replayed: bool,
}

impl CartItemMutation {
    pub fn created(line: CartLineView) -> Self {
        Self {
            line,
            replayed: false,
        }
    }

    pub fn replayed(line: CartLineView) -> Self {
        Self {
            line,
            replayed: true,
        }
    }
}

/// The buyer's current cart with its hydrated lines.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CartView {
    pub cart_id: String,
    pub status: String,
    pub currency_code: String,
    pub items: Vec<CartLineView>,
}

impl CartView {
    /// Total units across every line, selected or not.
    pub fn total_quantity(&self) -> i64 {
        self.items.iter().map(|item| item.quantity).sum()
    }

    /// Units the buyer actually intends to check out.
    pub fn selected_quantity(&self) -> i64 {
        self.items
            .iter()
            .filter(|item| item.selected)
            .map(|item| item.quantity)
            .sum()
    }

    /// Subtotal of the selected lines: the number the checkout entry point shows.
    pub fn selected_subtotal_minor(&self) -> Result<i64, CommerceServiceError> {
        self.items
            .iter()
            .filter(|item| item.selected)
            .try_fold(0_i64, |total, item| {
                total
                    .checked_add(item.line_amount_minor()?)
                    .ok_or_else(|| CommerceServiceError::validation("cart subtotal is too large"))
            })
    }

    pub fn selected_subtotal(&self) -> Result<CommerceMoney, CommerceServiceError> {
        Ok(minor_units_to_money(self.selected_subtotal_minor()?))
    }

    /// True when at least one selected line can no longer be sold.
    pub fn has_unavailable_selection(&self) -> bool {
        self.items
            .iter()
            .any(|item| item.selected && !item.available)
    }
}

/// Subject-scoped lookup of the buyer's live cart.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentCartQuery {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
}

impl CurrentCartQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        Ok(Self {
            tenant_id: tenant_id.to_owned(),
            organization_id: normalize_optional(organization_id),
            owner_user_id: owner_user_id.to_owned(),
        })
    }
}

/// Add one SKU to the buyer's cart. A repeated SKU increments its line.
///
/// `spu_id` and `shop_id` are deliberately absent: both are attribution, and a
/// client-supplied value would let a caller graft a line onto another product or
/// merchant. The product id is derived from the SKU row on the server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateCartItemCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub sku_id: String,
    pub quantity: i64,
    pub idempotency_key: Option<String>,
}

impl CreateCartItemCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        sku_id: &str,
        quantity: i64,
        idempotency_key: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("sku_id", sku_id)?;
        validate_quantity(quantity)?;
        Ok(Self {
            tenant_id: tenant_id.to_owned(),
            organization_id: normalize_optional(organization_id),
            owner_user_id: owner_user_id.to_owned(),
            sku_id: sku_id.trim().to_owned(),
            quantity,
            idempotency_key: normalize_idempotency_key(idempotency_key)?,
        })
    }
}

/// Patch one cart line: replace its quantity, toggle its selection, or both.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateCartItemCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub cart_item_id: String,
    pub quantity: Option<i64>,
    pub selected: Option<bool>,
}

impl UpdateCartItemCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        cart_item_id: &str,
        quantity: Option<i64>,
        selected: Option<bool>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("cart_item_id", cart_item_id)?;
        if quantity.is_none() && selected.is_none() {
            return Err(CommerceServiceError::validation(
                "cart item update requires quantity or selected",
            ));
        }
        if let Some(quantity) = quantity {
            validate_quantity(quantity)?;
        }
        Ok(Self {
            tenant_id: tenant_id.to_owned(),
            organization_id: normalize_optional(organization_id),
            owner_user_id: owner_user_id.to_owned(),
            cart_item_id: cart_item_id.trim().to_owned(),
            quantity,
            selected,
        })
    }
}

/// Remove the cart lines a checkout session just consumed.
///
/// A cart is the staging area in front of checkout, so the lines that became a
/// checkout session must not survive it — otherwise the buyer returns to a cart
/// still holding what they just bought. Consumption is keyed by the line ids the
/// client selected, so a partially selected cart keeps the rest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsumeCartItemsCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub cart_item_ids: Vec<String>,
}

impl ConsumeCartItemsCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        cart_item_ids: &[String],
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;

        let mut normalized: Vec<String> = Vec::new();
        for cart_item_id in cart_item_ids {
            let Some(cart_item_id) = normalize_optional(Some(cart_item_id.as_str())) else {
                continue;
            };
            if !normalized.contains(&cart_item_id) {
                normalized.push(cart_item_id);
            }
        }
        if normalized.is_empty() {
            return Err(CommerceServiceError::validation(
                "at least one cart item id is required",
            ));
        }
        if normalized.len() as i64 > CART_MAX_LINES {
            return Err(CommerceServiceError::validation(format!(
                "cart item ids must not exceed {CART_MAX_LINES}"
            )));
        }

        Ok(Self {
            tenant_id: tenant_id.to_owned(),
            organization_id: normalize_optional(organization_id),
            owner_user_id: owner_user_id.to_owned(),
            cart_item_ids: normalized,
        })
    }
}

/// Remove one cart line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeleteCartItemCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub cart_item_id: String,
}

impl DeleteCartItemCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        cart_item_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("cart_item_id", cart_item_id)?;
        Ok(Self {
            tenant_id: tenant_id.to_owned(),
            organization_id: normalize_optional(organization_id),
            owner_user_id: owner_user_id.to_owned(),
            cart_item_id: cart_item_id.trim().to_owned(),
        })
    }
}

fn validate_quantity(quantity: i64) -> Result<(), CommerceServiceError> {
    if quantity <= 0 {
        return Err(CommerceServiceError::validation(
            "cart item quantity must be greater than zero",
        ));
    }
    if quantity > CART_ITEM_MAX_QUANTITY {
        return Err(CommerceServiceError::validation(format!(
            "cart item quantity must not exceed {CART_ITEM_MAX_QUANTITY}"
        )));
    }
    Ok(())
}

fn normalize_idempotency_key(value: Option<&str>) -> Result<Option<String>, CommerceServiceError> {
    let Some(value) = normalize_optional(value) else {
        return Ok(None);
    };
    if value.len() > 128 {
        return Err(CommerceServiceError::validation(
            "Idempotency-Key header length must not exceed 128 characters",
        ));
    }
    Ok(Some(value))
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn minor_units_to_money(amount: i64) -> CommerceMoney {
    CommerceMoney::new(&amount.to_string())
        .expect("computed cart money should be valid minor units")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(sku_id: &str, unit_price_minor: i64, quantity: i64, selected: bool) -> CartLineView {
        CartLineView {
            cart_item_id: format!("cart-item-{sku_id}"),
            sku_id: sku_id.to_owned(),
            spu_id: None,
            quantity,
            selected,
            title: "Sample product".to_owned(),
            sku_name: None,
            image_url: None,
            unit_price: minor_units_to_money(unit_price_minor),
            price_scale: 2,
            currency_code: "CNY".to_owned(),
            available: true,
            current_unit_price: None,
            price_changed: false,
        }
    }

    #[test]
    fn create_command_rejects_an_empty_sku() {
        let error = CreateCartItemCommand::new("100001", None, "user-1", "  ", 1, None)
            .expect_err("empty sku must be rejected");
        assert!(error.message().contains("sku_id"));
    }

    #[test]
    fn create_command_rejects_out_of_range_quantities() {
        assert!(CreateCartItemCommand::new("100001", None, "user-1", "sku-1", 0, None).is_err());
        assert!(CreateCartItemCommand::new(
            "100001",
            None,
            "user-1",
            "sku-1",
            CART_ITEM_MAX_QUANTITY + 1,
            None,
        )
        .is_err());
        assert!(CreateCartItemCommand::new(
            "100001",
            None,
            "user-1",
            "sku-1",
            CART_ITEM_MAX_QUANTITY,
            None,
        )
        .is_ok());
    }

    #[test]
    fn create_command_normalizes_the_sku_and_rejects_an_overlong_idempotency_key() {
        let command = CreateCartItemCommand::new(
            "100001",
            Some("  "),
            "user-1",
            " sku-1 ",
            2,
            Some(" key-1 "),
        )
        .expect("command");
        assert_eq!(None, command.organization_id);
        assert_eq!("sku-1", command.sku_id);
        assert_eq!(Some("key-1".to_owned()), command.idempotency_key);

        let long_key = "k".repeat(129);
        assert!(
            CreateCartItemCommand::new("100001", None, "user-1", "sku-1", 1, Some(&long_key))
                .is_err()
        );
    }

    #[test]
    fn update_command_requires_at_least_one_field() {
        let error = UpdateCartItemCommand::new("100001", None, "user-1", "item-1", None, None)
            .expect_err("empty patch must be rejected");
        assert!(error.message().contains("quantity or selected"));

        assert!(
            UpdateCartItemCommand::new("100001", None, "user-1", "item-1", None, Some(false))
                .is_ok()
        );
    }

    #[test]
    fn cart_totals_count_only_selected_lines() {
        let cart = CartView {
            cart_id: "cart-1".to_owned(),
            status: CART_STATUS_ACTIVE.to_owned(),
            currency_code: CART_DEFAULT_CURRENCY_CODE.to_owned(),
            items: vec![line("sku-1", 1_999, 2, true), line("sku-2", 500, 3, false)],
        };

        assert_eq!(5, cart.total_quantity());
        assert_eq!(2, cart.selected_quantity());
        assert_eq!("3998", cart.selected_subtotal().expect("subtotal").as_str());
        assert_eq!(
            "3998",
            cart.items[0].line_total().expect("line total").as_str()
        );
        assert!(!cart.has_unavailable_selection());
    }

    #[test]
    fn unavailable_selection_is_visible_on_the_read_model() {
        let mut unavailable = line("sku-1", 1_999, 1, true);
        unavailable.available = false;
        let cart = CartView {
            cart_id: "cart-1".to_owned(),
            status: CART_STATUS_ACTIVE.to_owned(),
            currency_code: CART_DEFAULT_CURRENCY_CODE.to_owned(),
            items: vec![unavailable],
        };
        assert!(cart.has_unavailable_selection());
    }
}
