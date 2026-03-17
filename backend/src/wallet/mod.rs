/// Wallet domain – handles user fiat balances and transactions (deposits/withdrawals)
pub mod models;
pub mod routes;

use crate::auth::routes::AppState;
use axum::{
    routing::{get, post},
    Router,
};

/// Compose all wallet-domain routes into a single mountable [`Router`].
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        // HTML page
        .route("/wallet", get(page_wallet))
        // Form actions
        .route("/wallet/deposit", post(handle_deposit))
        .route("/wallet/withdraw", post(handle_withdraw))
        // JSON API
        .route("/api/wallet/balance", get(api_wallet_balance))
        .route("/api/wallet/transactions", get(api_wallet_transactions))
}
