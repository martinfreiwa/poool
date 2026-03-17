/// Cart domain – handles add/remove/list items in the shopping cart.
pub mod routes;

use crate::auth::routes::AppState;
use axum::{
    routing::{get, post},
    Router,
};

/// Compose all cart-domain routes into a single mountable [`Router`].
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        .route("/cart", get(page_cart))
        .route("/cart/add", post(add_to_cart))
        .route("/cart/remove", post(remove_from_cart))
        .route("/cart/update", post(update_cart_item))
        .route("/api/cart", get(api_cart))
}
