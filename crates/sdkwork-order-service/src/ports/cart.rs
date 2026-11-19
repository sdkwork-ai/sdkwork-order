//! Buyer cart persistence port.
//!
//! Route crates depend on this trait, never on PostgreSQL, so an embedded host
//! can hand the handlers any implementation (the authoritative one lives in
//! `sdkwork-order-repository-sqlx`).

use std::future::Future;
use std::pin::Pin;

use sdkwork_contract_service::CommerceServiceError;

use crate::domain::{
    CartLineView, CartView, CreateCartItemCommand, CurrentCartQuery, DeleteCartItemCommand,
    UpdateCartItemCommand,
};

pub type CartStoreFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommerceCartStore: Send + Sync {
    /// Reads the buyer's live cart, creating the empty cart on first access so
    /// the storefront always renders a stable `id`.
    fn retrieve_current_cart<'a>(
        &'a self,
        query: CurrentCartQuery,
    ) -> CartStoreFuture<'a, CartView>;

    /// Adds a SKU to the cart. A repeated SKU increments the existing line.
    fn create_cart_item<'a>(
        &'a self,
        command: CreateCartItemCommand,
    ) -> CartStoreFuture<'a, CartLineView>;

    /// Replaces one line quantity.
    fn update_cart_item<'a>(
        &'a self,
        command: UpdateCartItemCommand,
    ) -> CartStoreFuture<'a, CartLineView>;

    /// Removes one line.
    fn delete_cart_item<'a>(&'a self, command: DeleteCartItemCommand) -> CartStoreFuture<'a, ()>;
}
