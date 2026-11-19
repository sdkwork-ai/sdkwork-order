//! Buyer cart domain rules.
//!
//! The cart is the buyer-owned staging area in front of checkout: a cart line
//! snapshots the merchandise SKU (title, unit price, currency) at add time so a
//! later catalog price change never rewrites what the buyer already saw. The
//! checkout session stays the authority for what is actually charged.
//!
//! Every query and command carries the resolved subject (`tenant_id`,
//! `organization_id`, `owner_user_id`); nothing here reads an ambient context.

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

/// Status of the single live cart a buyer owns.
pub const CART_STATUS_ACTIVE: &str = "active";

/// Upper bound for one line. Larger quantities are a client defect, not a sale.
pub const CART_ITEM_MAX_QUANTITY: i64 = 999;

/// Default currency when the caller never named one.
pub const CART_DEFAULT_CURRENCY_CODE: &str = "CNY";

/// One hydrated cart line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CartLineView {
    pub cart_item_id: String,
    pub sku_id: String,
    pub spu_id: Option<String>,
    pub shop_id: Option<String>,
    pub quantity: i64,
    pub title: String,
    pub sku_name: Option<String>,
    pub image_url: Option<String>,
    pub unit_price: CommerceMoney,
    pub price_scale: i16,
    pub currency_code: String,
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

/// The buyer's current cart with its hydrated lines.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CartView {
    pub cart_id: String,
    pub status: String,
    pub currency_code: String,
    pub items: Vec<CartLineView>,
}

impl CartView {
    pub fn total_quantity(&self) -> i64 {
        self.items.iter().map(|item| item.quantity).sum()
    }

    pub fn subtotal_minor(&self) -> Result<i64, CommerceServiceError> {
        self.items.iter().try_fold(0_i64, |total, item| {
            total
                .checked_add(item.line_amount_minor()?)
                .ok_or_else(|| CommerceServiceError::validation("cart subtotal is too large"))
        })
    }

    pub fn subtotal(&self) -> Result<CommerceMoney, CommerceServiceError> {
        Ok(minor_units_to_money(self.subtotal_minor()?))
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateCartItemCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub sku_id: String,
    pub spu_id: Option<String>,
    pub shop_id: Option<String>,
    pub quantity: i64,
}

impl CreateCartItemCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        sku_id: &str,
        spu_id: Option<&str>,
        shop_id: Option<&str>,
        quantity: i64,
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
            spu_id: normalize_optional(spu_id),
            shop_id: normalize_optional(shop_id),
            quantity,
        })
    }
}

/// Replace one cart line quantity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateCartItemCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub cart_item_id: String,
    pub quantity: i64,
}

impl UpdateCartItemCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        cart_item_id: &str,
        quantity: i64,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("cart_item_id", cart_item_id)?;
        validate_quantity(quantity)?;
        Ok(Self {
            tenant_id: tenant_id.to_owned(),
            organization_id: normalize_optional(organization_id),
            owner_user_id: owner_user_id.to_owned(),
            cart_item_id: cart_item_id.trim().to_owned(),
            quantity,
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

    fn line(sku_id: &str, unit_price_minor: i64, quantity: i64) -> CartLineView {
        CartLineView {
            cart_item_id: format!("cart-item-{sku_id}"),
            sku_id: sku_id.to_owned(),
            spu_id: None,
            shop_id: None,
            quantity,
            title: "Sample product".to_owned(),
            sku_name: None,
            image_url: None,
            unit_price: minor_units_to_money(unit_price_minor),
            price_scale: 2,
            currency_code: "CNY".to_owned(),
        }
    }

    #[test]
    fn create_command_rejects_an_empty_sku() {
        let error = CreateCartItemCommand::new("100001", None, "user-1", "  ", None, None, 1)
            .expect_err("empty sku must be rejected");
        assert!(error.message().contains("sku_id"));
    }

    #[test]
    fn create_command_rejects_out_of_range_quantities() {
        assert!(
            CreateCartItemCommand::new("100001", None, "user-1", "sku-1", None, None, 0).is_err()
        );
        assert!(CreateCartItemCommand::new(
            "100001",
            None,
            "user-1",
            "sku-1",
            None,
            None,
            CART_ITEM_MAX_QUANTITY + 1,
        )
        .is_err());
    }

    #[test]
    fn create_command_normalizes_optional_ids() {
        let command = CreateCartItemCommand::new(
            "100001",
            Some("  "),
            "user-1",
            " sku-1 ",
            Some(""),
            None,
            2,
        )
        .expect("command");
        assert_eq!(None, command.organization_id);
        assert_eq!("sku-1", command.sku_id);
        assert_eq!(None, command.spu_id);
    }

    #[test]
    fn cart_totals_multiply_the_snapshot_price() {
        let cart = CartView {
            cart_id: "cart-1".to_owned(),
            status: CART_STATUS_ACTIVE.to_owned(),
            currency_code: CART_DEFAULT_CURRENCY_CODE.to_owned(),
            items: vec![line("sku-1", 1_999, 2), line("sku-2", 500, 3)],
        };

        assert_eq!(5, cart.total_quantity());
        assert_eq!("5498", cart.subtotal().expect("subtotal").as_str());
        assert_eq!(
            "3998",
            cart.items[0].line_total().expect("line total").as_str()
        );
    }
}
