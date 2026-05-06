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
        // HTML pages
        .route("/wallet", get(page_wallet))
        .route("/transactions/:id", get(page_transaction_detail))
        // Form actions
        .route("/wallet/deposit", post(handle_deposit))
        .route("/wallet/withdraw", post(handle_withdraw))
        // JSON API
        .route("/api/wallet/balance", get(api_wallet_balance))
        .route("/api/wallet/transactions", get(api_wallet_transactions))
        .route("/api/wallet/transactions/:id", get(api_transaction_detail))
}
