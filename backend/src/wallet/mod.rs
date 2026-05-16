/// Wallet domain – handles user fiat balances and transactions (deposits/withdrawals)
pub mod models;
pub mod reconciliation;
pub mod routes;
pub mod safety;

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
        .route("/wallet/deposit/:id/submit", post(handle_deposit_submit))
        .route("/api/wallet/deposit/init", post(api_deposit_init))
        .route("/api/wallet/step-up/verify", post(api_step_up_verify))
        .route("/wallet/withdraw", post(handle_withdraw))
        .route(
            "/api/wallet/withdrawals/:id/cancel",
            post(api_cancel_withdrawal),
        )
        // JSON API
        .route("/api/wallet/balance", get(api_wallet_balance))
        .route("/api/wallet/transactions", get(api_wallet_transactions))
        // Dedicated export path — avoids colliding with the `:id`
        // wildcard below which would otherwise try to parse "export"
        // as a UUID. Axum's matchit treats a `.` in the path segment
        // as part of the segment but the static-file fallback can
        // beat us to it; keep the path extension-free.
        .route(
            "/api/wallet/export/transactions",
            get(api_wallet_transactions_export),
        )
        .route("/api/wallet/transactions/:id", get(api_transaction_detail))
        .route("/api/wallet/deposit-settings", get(api_deposit_settings))
}
