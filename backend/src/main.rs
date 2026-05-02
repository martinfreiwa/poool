#![recursion_limit = "256"]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::double_ended_iterator_last)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::inconsistent_digit_grouping)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::manual_strip)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::useless_format)]
#![allow(clippy::useless_vec)]
#![deny(missing_docs)]

/*!
POOOL Backend  Main entry point.

This file sets up the Tokio runtime, initializes configuration, structured logging,
the database connection pool, and the Axum HTTP router.

The server serves both the static frontend (HTML/CSS/JS) and a JSON API.
Routes are conceptually grouped into:
1. Public endpoints (Auth, Static assets)
2. Protected HTML Views (Dashboard, Settings, Developer pages, Admin pages)
3. Internal API Endpoints (Cart, Checkout, User Settings, Admin Stats)
*/

/// Admin module containing handlers for platform-wide oversight, user management, and financial auditing.
pub mod admin;
#[allow(missing_docs)]
pub mod assets;
mod auth;
/// Blog module containing handlers for blog articles, authors, and categories.
/// Blockchain integration module — on-chain settlement via POOOLProperty1155 on Polygon.
mod blockchain;
mod blog;
mod cart;
mod common;
mod community;
mod config;
mod db;
mod developer;

/// Dividend system — calculation, anti-sniping, and payout execution for rental income.
mod dividends;
mod email;
mod error;
/// IPFS integration — Pinata-based metadata storage for ERC-1155 asset tokens.
mod ipfs;
mod kyc;
mod leaderboard;
/// Legal module containing handlers for terms of service, privacy policy, and other legal documents.
pub mod legal;
/// Marketplace module — secondary market trading engine for tokenized assets.
mod marketplace;
mod payment_methods;
mod payments;
mod portfolio;
mod rewards;
mod settings;
mod storage;
#[allow(missing_docs)]
pub mod support; // Added support module
mod templates;
mod wallet;

use admin::extractors::{AdminUser, ApiError};
use auth::routes::AppState;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json, Redirect},
    routing::{get, post},
    Router, ServiceExt,
};
use axum_extra::extract::cookie::CookieJar;

use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install rustls crypto provider before any TLS connections (required by rustls 0.23+).
    // Must run before Sentry, Redis, reqwest, or any TLS-using library initializes.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // ── 1. Configuration ─────────────────────────────────────────
    let config = config::Config::from_env();

    // ── 2. Sentry Initialisation (must happen before tracing setup) ─
    // The guard must be kept alive for the whole process lifetime.
    // If SENTRY_DSN is absent (local dev / CI), Sentry is a no-op.
    let _sentry_guard = config.sentry_dsn.as_ref().map(|dsn| {
        let env_name = config.app_env.clone();
        let sample_rate = if env_name == "production" { 0.2 } else { 1.0 };

        sentry::init((
            dsn.as_str(),
            sentry::ClientOptions {
                release: sentry::release_name!(),
                environment: Some(env_name.into()),
                traces_sample_rate: sample_rate,
                send_default_pii: false,
                attach_stacktrace: true,
                server_name: std::env::var("K_REVISION").ok().map(|s| s.into()),
                ..Default::default()
            },
        ))
    });
    if _sentry_guard.is_some() {
        tracing::info!("Sentry initialised");
    }

    // ── 2b. Critical-env preflight check ─────────────────────────
    // Surfaces missing/misconfigured env vars LOUDLY at startup so they
    // don't show up later as opaque "An unexpected error" responses to
    // the user. Each entry is checked but the process keeps booting —
    // we only log + Sentry. Fix the env, don't fix the symptom downstream.
    {
        struct Check {
            keys: &'static [&'static str],
            why: &'static str,
        }
        let checks = [
            Check {
                keys: &["TOTP_SECRET_ENCRYPTION_KEY", "ENCRYPTION_KEY"],
                why: "TOTP setup + step-up 2FA will fail with 'An unexpected error'",
            },
            Check {
                keys: &["DATABASE_URL"],
                why: "DB connection will fail",
            },
            Check {
                keys: &["SESSION_SECRET", "JWT_SECRET"],
                why: "Session cookies will be unsignable",
            },
        ];
        for c in checks {
            let present_key = c.keys.iter().find(|key| {
                std::env::var(key)
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
            });
            if let Some(key) = present_key {
                tracing::info!("✅ env {} present", key);
            } else {
                let key_list = c.keys.join(" or ");
                tracing::error!("🚨 env {} MISSING — {}", key_list, c.why);
                sentry::capture_message(
                    &format!("Startup env check failed: {} missing — {}", key_list, c.why),
                    sentry::Level::Error,
                );
            }
        }
    }

    // ── 3. Tracing / Logging Setup ───────────────────────────────────
    // sentry_tracing integrates so that every `tracing::error!` call
    // is automatically forwarded to Sentry as a breadcrumb / event.

    use tracing_subscriber::prelude::*;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer().with_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            ),
        )
        .with(sentry_tracing::layer())
        .init();

    tracing::info!("Starting POOOL Backend...");

    //  2. Database & State Initialization (Phase 1.1: Dual pools)
    let pools = db::create_pools(&config).await;
    let pool = pools.primary.clone();

    // ── 4. Run database migrations ────────────────────────────────
    // Reads .sql files from ../database/ in alphanumeric order, tracks which
    // have been applied in a `_schema_migrations` table. Idempotent.
    run_migrations(&pool, "../database", "core").await;

    if let Some(ref comm_pool) = pools.community {
        run_migrations(comm_pool, "../database/community", "community").await;
    }

    let templates = templates::create_engine();

    let redis_pool = if let Some(url) = &config.redis_url {
        use deadpool_redis::{Config, Runtime};

        // Enforce TLS (`rediss://`) outside dev/local — data in flight to
        // Redis includes session tokens, TOTP replay nonces, CSRF-adjacent
        // state and rate-limit counters. `redis://` over an untrusted
        // network exposes all of them. We fail closed rather than silently
        // downgrade.
        let is_dev = matches!(
            std::env::var("POOOL_ENV").as_deref(),
            Ok("development") | Ok("dev") | Ok("local")
        );
        let is_tls = url.starts_with("rediss://") || url.starts_with("unix://");
        if !is_tls {
            if is_dev {
                tracing::warn!("Redis URL uses plaintext transport — acceptable in dev only");
            } else {
                tracing::error!(
                    "REDIS_URL must use 'rediss://' (TLS) in production; refusing to start a plaintext connection. Set POOOL_ENV=development to override locally."
                );
                panic!("Insecure REDIS_URL rejected (non-TLS in non-dev environment)");
            }
        }

        // Redact credentials before logging. Parse conservatively — if the
        // URL is malformed we simply omit the host rather than echo it.
        let safe_host = url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
            .unwrap_or_else(|| "<redacted>".to_string());

        let cfg = Config::from_url(url);
        match cfg.create_pool(Some(Runtime::Tokio1)) {
            Ok(p) => {
                tracing::info!(
                    "Redis pool initialized (host={}, tls={})",
                    safe_host,
                    is_tls
                );
                Some(p)
            }
            Err(e) => {
                tracing::error!("Failed to create Redis pool: {}", e);
                None
            }
        }
    } else {
        None
    };

    let auth_rate_limiter = auth::rate_limit::RateLimiter::disabled();
    tracing::info!("Rate limiter: disabled");

    let state = AppState {
        db: pool.clone(),
        db_replica: pools.replica,
        community_db: pools.community,
        templates,
        config: config.clone(),
        redis: redis_pool,
        auth_rate_limiter: auth_rate_limiter.clone(),
    };

    // Spawn background tasks
    tokio::spawn(email::run_email_scheduler(pool.clone()));
    tokio::spawn(email::run_transactional_email_outbox_worker(pool.clone()));
    tokio::spawn(support::sla::monitor_sla_breaches(pool.clone()));

    if let Some(c_pool) = &state.community_db {
        tokio::spawn(community::background::monitor_asset_velocity(
            c_pool.clone(),
            pool.clone(),
        ));
        tokio::spawn(community::background::gamification_worker(
            c_pool.clone(),
            pool.clone(),
        ));
        tokio::spawn(community::background::xp_aggregation_worker(c_pool.clone()));
        tokio::spawn(community::background::circle_invite_expiry_worker(
            c_pool.clone(),
        ));
        tokio::spawn(community::background::circle_retry_worker(
            c_pool.clone(),
            pool.clone(),
        ));
        tokio::spawn(community::background::gdpr_anonymization_worker(
            c_pool.clone(),
            pool.clone(),
        ));
        tokio::spawn(community::background::weekly_digest_worker(
            c_pool.clone(),
            pool.clone(),
        ));
    }

    // Auto-refund worker for expired primary escrow offerings
    tokio::spawn(admin::primary_escrow::run_auto_refund_worker(pool.clone()));

    // Affiliate holdback worker (runs every 6 hours)
    tokio::spawn(rewards::service::run_affiliate_holdback_worker(
        pool.clone(),
    ));

    // Affiliate tier progression worker (runs every 24 hours)
    tokio::spawn(rewards::service::run_affiliate_tier_progression_worker(
        pool.clone(),
    ));

    // Rate limiter cleanup (every 10 minutes)
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10 * 60));
        loop {
            interval.tick().await;
            auth_rate_limiter.cleanup().await;
        }
    });

    // Payments: Token reclaim worker
    let cleanup_pool = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15 * 60)); // 15 mins
        loop {
            interval.tick().await;
            if let Err(e) = crate::payments::service::cleanup_expired_orders(&cleanup_pool).await {
                tracing::error!("Error cleaning up expired orders: {}", e);
            }
        }
    });

    // Leaderboard: Refresh metrics and rankings periodically
    let leaderboard_pool = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15 * 60)); // 15 mins
                                                                                           // Small initial delay to avoid slamming the DB on immediate startup
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        loop {
            if let Err(e) = crate::leaderboard::service::refresh_all_scores(&leaderboard_pool).await
            {
                tracing::error!("Error refreshing leaderboard scores: {}", e);
            }
            interval.tick().await;
        }
    });

    // Housekeeping: Purge expired sessions and used password reset tokens
    let housekeeping_pool = pool.clone();
    tokio::spawn(async move {
        // Run every 6 hours
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(6 * 60 * 60));
        loop {
            interval.tick().await;
            // 1. Delete expired sessions
            match sqlx::query("DELETE FROM user_sessions WHERE expires_at < NOW()")
                .execute(&housekeeping_pool)
                .await
            {
                Ok(res) => {
                    if res.rows_affected() > 0 {
                        tracing::info!(
                            "Housekeeping: purged {} expired sessions",
                            res.rows_affected()
                        );
                    }
                }
                Err(e) => tracing::error!("Housekeeping: failed to purge sessions: {}", e),
            }

            // 2. Delete used or expired password reset tokens (older than 24h)
            match sqlx::query(
                "DELETE FROM password_reset_tokens WHERE used_at IS NOT NULL OR expires_at < NOW() - interval '24 hours'"
            )
            .execute(&housekeeping_pool)
            .await
            {
                Ok(res) => {
                    if res.rows_affected() > 0 {
                        tracing::info!(
                            "Housekeeping: purged {} spent/expired reset tokens",
                            res.rows_affected()
                        );
                    }
                }
                Err(e) => tracing::error!("Housekeeping: failed to purge reset tokens: {}", e),
            }

            // 3. Delete expired email verification tokens (older than 48h)
            match sqlx::query(
                "DELETE FROM email_verification_tokens WHERE expires_at < NOW() - interval '48 hours'"
            )
            .execute(&housekeeping_pool)
            .await
            {
                Ok(res) => {
                    if res.rows_affected() > 0 {
                        tracing::info!(
                            "Housekeeping: purged {} expired email verification tokens",
                            res.rows_affected()
                        );
                    }
                }
                Err(e) => {
                    // Table might not exist yet — suppress error
                    if !e.to_string().contains("does not exist") {
                        tracing::error!("Housekeeping: failed to purge email tokens: {}", e);
                    }
                }
            }
        }
    });

    // Masterplan Priority 2: Daily Financial Reconciliation (Phase 1.9 — enhanced)
    // Runs every 24 hours with the following checks:
    // 1. Cash balance invariant: SUM(Wallets) == SUM(Deposits) - SUM(Withdrawals) - SUM(Purchases)
    // 2. Token balance invariant: SUM(tokens_owned) per asset == tokens_total - tokens_available
    // 3. Negative wallet balance detection
    // 4. Sentry Fatal alert for cash deltas >$1
    let recon_pool = pool.clone();
    tokio::spawn(async move {
        // Initial delay to not slam the DB on startup
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;

        // Check once initially, then every 24h
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 60 * 60));
        loop {
            interval.tick().await;
            tracing::info!("Starting daily financial reconciliation...");

            // Accumulators for persisting to reconciliation_reports (Task 10.8)
            let mut recon_cash_delta: i64 = 0;
            let mut recon_total_wallets: i64 = 0;
            let mut recon_total_deposits: i64 = 0;
            let mut recon_total_withdrawals: i64 = 0;
            let mut recon_total_purchases: i64 = 0;
            let mut recon_token_mismatches: i32 = 0;
            let mut recon_negative_count: i32 = 0;

            // ── Check 1: Cash Balance Invariant ────────────────────────
            let totals = sqlx::query!(
                r#"
                SELECT 
                    (SELECT COALESCE(SUM(balance_cents), 0)::bigint FROM wallets WHERE wallet_type = 'cash' AND currency = 'USD') as total_wallets,
                    (SELECT COALESCE(SUM(amount_cents), 0)::bigint FROM deposit_requests WHERE status IN ('approved', 'completed', 'paid') AND currency = 'USD') as total_deposits,
                    (SELECT COALESCE(SUM(amount_cents), 0)::bigint FROM withdrawal_requests WHERE status != 'rejected' AND currency = 'USD') as total_withdrawals,
                    (SELECT COALESCE(SUM(total_cents), 0)::bigint FROM orders WHERE status IN ('completed', 'pending_kyc') AND payment_method = 'wallet') as total_purchases,
                    (SELECT COALESCE(SUM(amount_cents), 0)::bigint FROM wallet_transactions WHERE wallet_id IN (SELECT id FROM wallets WHERE wallet_type = 'cash' AND currency = 'USD') AND status = 'completed') as expected_wallets_ledger
                "#
            )
            .fetch_one(&recon_pool)
            .await;

            match totals {
                Ok(t) => {
                    let total_wallets = t.total_wallets.unwrap_or(0);
                    let total_deposits = t.total_deposits.unwrap_or(0);
                    let total_withdrawals = t.total_withdrawals.unwrap_or(0);
                    let total_purchases = t.total_purchases.unwrap_or(0);
                    let expected_wallets = t.expected_wallets_ledger.unwrap_or(0);

                    recon_total_wallets = total_wallets;
                    recon_total_deposits = total_deposits;
                    recon_total_withdrawals = total_withdrawals;
                    recon_total_purchases = total_purchases;
                    let delta = total_wallets - expected_wallets;
                    recon_cash_delta = delta;

                    if delta.abs() > 100 {
                        let msg = format!(
                            "RECONCILIATION FATAL: Cash delta of {} cents (wallets={}, expected={}). Deposits={}, Withdrawals={}, Purchases={}",
                            delta, total_wallets, expected_wallets, total_deposits, total_withdrawals, total_purchases
                        );
                        tracing::error!("{}", msg);
                        sentry::with_scope(
                            |scope| {
                                scope.set_tag("security.event", "reconciliation_fatal");
                                scope.set_tag("reconciliation.delta_cents", delta.to_string());
                            },
                            || {
                                sentry::capture_message(&msg, sentry::Level::Fatal);
                            },
                        );
                    } else if delta != 0 {
                        let msg = format!(
                            "RECONCILIATION WARNING: Minor delta of {} cents (wallets={}, expected={})",
                            delta, total_wallets, expected_wallets
                        );
                        tracing::warn!("{}", msg);
                        sentry::capture_message(&msg, sentry::Level::Warning);
                    } else {
                        tracing::info!("Reconciliation check 1/3 PASS: Cash wallets perfectly match deposits/withdrawals/purchases.");
                    }
                }
                Err(e) => {
                    tracing::error!("Reconciliation check 1 FAILED: {}", e);
                }
            }

            // ── Check 2: Token Balance Invariant ───────────────────────
            let token_mismatches = sqlx::query!(
                r#"
                SELECT a.id, a.title as "title!", a.tokens_total as "tokens_total!", a.tokens_available as "tokens_available!",
                       COALESCE(inv.total_owned, 0)::int as "total_owned!"
                FROM assets a
                LEFT JOIN (
                    SELECT asset_id, SUM(tokens_owned)::int as total_owned
                    FROM investments
                    WHERE status != 'exited'
                    GROUP BY asset_id
                ) inv ON inv.asset_id = a.id
                WHERE a.funding_status IN ('funding_open', 'funding_in_progress', 'funded')
                  AND (a.tokens_total - a.tokens_available) != COALESCE(inv.total_owned, 0)
                "#
            )
            .fetch_all(&recon_pool)
            .await;

            match token_mismatches {
                Ok(rows) => {
                    recon_token_mismatches = rows.len() as i32;
                    if rows.is_empty() {
                        tracing::info!("Reconciliation check 2/3 PASS: All token balances match.");
                    } else {
                        for row in &rows {
                            let expected_sold = row.tokens_total - row.tokens_available;
                            let msg = format!(
                                "TOKEN MISMATCH: Asset '{}' ({:?}): sold={} but investments show {} tokens",
                                row.title,
                                row.id,
                                expected_sold,
                                row.total_owned
                            );
                            tracing::error!("{}", msg);
                            sentry::capture_message(&msg, sentry::Level::Error);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Reconciliation check 2 FAILED: {}", e);
                }
            }

            // ── Check 3: Negative Balance Detection ────────────────────
            let negative_wallets = sqlx::query!(
                r#"
                SELECT w.id, w.user_id, w.wallet_type, w.currency, w.balance_cents, u.email
                FROM wallets w
                JOIN users u ON u.id = w.user_id
                WHERE w.balance_cents < 0
                LIMIT 50
                "#
            )
            .fetch_all(&recon_pool)
            .await;

            match negative_wallets {
                Ok(rows) => {
                    recon_negative_count = rows.len() as i32;
                    if rows.is_empty() {
                        tracing::info!(
                            "Reconciliation check 3/3 PASS: No negative wallet balances."
                        );
                    } else {
                        for row in &rows {
                            let msg = format!(
                                "NEGATIVE BALANCE: User {} ({}) has {} cents in {} {} wallet",
                                row.user_id,
                                row.email,
                                row.balance_cents,
                                row.currency,
                                row.wallet_type
                            );
                            tracing::error!("{}", msg);
                            sentry::capture_message(&msg, sentry::Level::Fatal);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Reconciliation check 3 FAILED: {}", e);
                }
            }

            // ── Task 10.8: Persist reconciliation results ──────────────
            let report_date = chrono::Utc::now().date_naive();
            let status = if recon_cash_delta.abs() > 100
                || recon_token_mismatches > 0
                || recon_negative_count > 0
            {
                "fail"
            } else if recon_cash_delta != 0 {
                "warning"
            } else {
                "pass"
            };
            let notes = format!(
                "Cash delta: {} cents, Token mismatches: {}, Negative wallets: {}",
                recon_cash_delta, recon_token_mismatches, recon_negative_count
            );
            if let Err(e) = sqlx::query(
                r#"INSERT INTO reconciliation_reports (
                    report_date, total_wallet_cents, total_deposits_cents,
                    total_withdrawals_cents, total_purchases_cents, cash_delta_cents,
                    total_fees_earned_cents, fee_wallet_cents, fee_delta_cents,
                    token_mismatches, status, notes
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (report_date) DO UPDATE SET
                    total_wallet_cents = EXCLUDED.total_wallet_cents,
                    total_deposits_cents = EXCLUDED.total_deposits_cents,
                    total_withdrawals_cents = EXCLUDED.total_withdrawals_cents,
                    total_purchases_cents = EXCLUDED.total_purchases_cents,
                    cash_delta_cents = EXCLUDED.cash_delta_cents,
                    total_fees_earned_cents = EXCLUDED.total_fees_earned_cents,
                    fee_wallet_cents = EXCLUDED.fee_wallet_cents,
                    fee_delta_cents = EXCLUDED.fee_delta_cents,
                    token_mismatches = EXCLUDED.token_mismatches,
                    status = EXCLUDED.status,
                    notes = EXCLUDED.notes"#,
            )
            .bind(report_date)
            .bind(recon_total_wallets)
            .bind(recon_total_deposits)
            .bind(recon_total_withdrawals)
            .bind(recon_total_purchases)
            .bind(recon_cash_delta)
            .bind(0_i64) // total_fees_earned_cents — tracked separately when fee system is wired
            .bind(0_i64) // fee_wallet_cents — tracked separately when fee system is wired
            .bind(0_i64) // fee_delta_cents
            .bind(recon_token_mismatches)
            .bind(status)
            .bind(&notes)
            .execute(&recon_pool)
            .await
            {
                tracing::error!("Failed to persist reconciliation report: {}", e);
            } else {
                tracing::info!(
                    "📊 Reconciliation report persisted for {} — status: {}",
                    report_date,
                    status
                );
            }

            tracing::info!("Daily financial reconciliation completed.");
        }
    });

    // ── Marketplace: Matching Engine + Settlement Worker ───────
    // These are the core trading engine tasks. They only start if Redis is
    // configured (the orderbook and match queue live in Redis).
    if let Some(ref redis) = state.redis {
        // Matching Engine: leader-elected. Without this, every replica
        // would push duplicate match events, poisoning settlement.
        // Closures clone-capture so re-acquisition gets a fresh future.
        let match_redis = redis.clone();
        let match_pool = pool.clone();
        let match_lock_pool = pool.clone();
        tokio::spawn(common::leader::run_as_leader(
            match_lock_pool,
            common::leader::LockKey::MarketplaceMatching,
            move || {
                let r = match_redis.clone();
                let p = match_pool.clone();
                async move { marketplace::matching::run_matching_engine(&r, &p).await }
            },
        ));

        // Settlement Worker: leader-elected.
        let settle_redis = redis.clone();
        let settle_pool = pool.clone();
        let settle_lock_pool = pool.clone();
        tokio::spawn(common::leader::run_as_leader(
            settle_lock_pool,
            common::leader::LockKey::MarketplaceSettlement,
            move || {
                let r = settle_redis.clone();
                let p = settle_pool.clone();
                async move { marketplace::settlement::run_settlement_worker(&r, &p).await }
            },
        ));

        // Background Worker #1: Order Expiry
        let expiry_redis = redis.clone();
        let expiry_pool = pool.clone();
        let expiry_lock_pool = pool.clone();
        tokio::spawn(common::leader::run_as_leader(
            expiry_lock_pool,
            common::leader::LockKey::MarketplaceOrderExpiry,
            move || {
                let r = expiry_redis.clone();
                let p = expiry_pool.clone();
                async move { marketplace::background::run_order_expiry_worker(&r, &p).await }
            },
        ));

        // Background Worker #2: Redis Sync
        let sync_redis = redis.clone();
        let sync_pool = pool.clone();
        let sync_lock_pool = pool.clone();
        tokio::spawn(common::leader::run_as_leader(
            sync_lock_pool,
            common::leader::LockKey::MarketplaceRedisSync,
            move || {
                let r = sync_redis.clone();
                let p = sync_pool.clone();
                async move { marketplace::background::run_redis_sync_worker(&r, &p).await }
            },
        ));

        // Background Worker #3: Price Snapshot
        let price_redis = redis.clone();
        let price_pool = pool.clone();
        let price_lock_pool = pool.clone();
        tokio::spawn(common::leader::run_as_leader(
            price_lock_pool,
            common::leader::LockKey::MarketplacePriceSnapshot,
            move || {
                let r = price_redis.clone();
                let p = price_pool.clone();
                async move { marketplace::background::run_price_snapshot_worker(&r, &p).await }
            },
        ));

        // Background Worker #4: Alert Escalation
        let escalate_pool = pool.clone();
        let escalate_lock_pool = pool.clone();
        tokio::spawn(common::leader::run_as_leader(
            escalate_lock_pool,
            common::leader::LockKey::MarketplaceAlertEscalation,
            move || {
                let p = escalate_pool.clone();
                async move { marketplace::background::run_alert_escalation_worker(&p).await }
            },
        ));

        // WebSocket: Redis Pub/Sub subscriber (cross-instance message delivery)
        let pubsub_redis = redis.clone();
        tokio::spawn(async move {
            marketplace::websocket::run_pubsub_subscriber(&pubsub_redis).await;
        });

        tracing::info!("🚀 Marketplace engine started (Matching + Settlement + WebSocket + 3 Background Workers)");
    } else {
        tracing::warn!("⚠️ Redis not configured — Marketplace trading is DISABLED. Order submission still works but matching won't occur.");
    }

    // ── Blockchain workers — all leader-elected ──────────────
    // The on-chain pipeline is especially sensitive to multi-replica
    // duplication: two indexers would double-count balance deltas, two
    // settlement workers would Sentry-spam reservation races, two
    // reconcilers/KYC workers would waste gas on duplicate TXs.
    let chain_pool = pool.clone();
    tokio::spawn(common::leader::run_as_leader(
        pool.clone(),
        common::leader::LockKey::BlockchainSettlement,
        move || {
            let p = chain_pool.clone();
            async move { blockchain::service::run_settlement_worker(&p).await }
        },
    ));

    let indexer_pool = pool.clone();
    tokio::spawn(common::leader::run_as_leader(
        pool.clone(),
        common::leader::LockKey::BlockchainEventIndexer,
        move || {
            let p = indexer_pool.clone();
            async move { blockchain::event_indexer::run_event_indexer(&p).await }
        },
    ));

    let whitelist_pool = pool.clone();
    tokio::spawn(common::leader::run_as_leader(
        pool.clone(),
        common::leader::LockKey::BlockchainKycWhitelist,
        move || {
            let p = whitelist_pool.clone();
            async move { blockchain::kyc_whitelist::run_kyc_whitelist_worker(&p).await }
        },
    ));

    let reconciler_pool = pool.clone();
    tokio::spawn(common::leader::run_as_leader(
        pool.clone(),
        common::leader::LockKey::BlockchainReconciler,
        move || {
            let p = reconciler_pool.clone();
            async move { blockchain::reconciler::run_reconciler(&p).await }
        },
    ));

    let gas_pool = pool.clone();
    tokio::spawn(common::leader::run_as_leader(
        pool.clone(),
        common::leader::LockKey::BlockchainGasMonitor,
        move || {
            let p = gas_pool.clone();
            async move { blockchain::gas_monitor::run_gas_monitor(&p).await }
        },
    ));

    // Marketplace settings scheduler (#28): apply scheduled settings changes
    let mp_sched_pool = pool.clone();
    let mp_sched_redis = state.redis.clone();
    tokio::spawn(common::leader::run_as_leader(
        pool.clone(),
        common::leader::LockKey::MarketplaceSettingsScheduler,
        move || {
            let p = mp_sched_pool.clone();
            let r = mp_sched_redis.clone();
            async move { admin::marketplace::run_settings_scheduler(p, r).await }
        },
    ));

    // ── Marketplace: Fund-conservation invariant worker ──────
    // Hourly check that SUM(wallet balances) == SUM(deposits - withdrawals).
    // Any drift = critical Sentry alert. Also asserts per-wallet
    // (held <= balance) and per-asset (sum_owned <= tokens_total) bounds.
    let invariant_pool = pool.clone();
    tokio::spawn(async move {
        marketplace::invariants::run_invariant_worker(&invariant_pool).await;
    });

    //  3. Router configuration

    //
    // Two routers for host-based routing:
    //   • www_router   → Landing page (www.poool.app)
    //   • platform_router → Dashboard / API (platform.poool.app)
    //   • poool.app (bare domain) → 301 redirect to www.poool.app
    //   • localhost → serves platform (dev default)

    // ── WWW Landing Page Router ───────────────────────────────────────
    let www_router = Router::new()
        .route("/health", get(handle_health_basic))
        // Language-specific landing pages
        .nest_service("/id", ServeDir::new("../frontend/www/id"))
        // Shared assets referenced by the Angular SPA
        .nest_service("/fonts", ServeDir::new("../frontend/www/fonts"))
        .nest_service("/png", ServeDir::new("../frontend/www/png"))
        .nest_service("/svg", ServeDir::new("../frontend/www/svg"))
        .nest_service("/webp", ServeDir::new("../frontend/www/webp"))
        .nest_service("/webm", ServeDir::new("../frontend/www/webm"))
        .route_service("/robots.txt", ServeFile::new("../frontend/www/robots.txt"))
        .route_service(
            "/sitemap.xml",
            ServeFile::new("../frontend/www/sitemap.xml"),
        )
        // Shared assets for landing-v2
        .nest_service("/static", ServeDir::new("../frontend/platform/static"))
        .nest_service(
            "/images",
            ServeDir::new("../frontend/platform/static/images/seed")
                .fallback(ServeDir::new("../frontend/platform/static/images/ui")),
        )
        .route(
            "/platform",
            get(|| async { Redirect::to("https://platform.poool.app/") }),
        )
        .route(
            "/platform/",
            get(|| async { Redirect::to("https://platform.poool.app/") }),
        )
        .route(
            "/auth/login",
            get(|| async { Redirect::to("https://platform.poool.app/auth/login") }),
        )
        .route(
            "/auth/signup",
            get(|| async { Redirect::to("https://platform.poool.app/auth/signup") }),
        )
        .route(
            "/signup",
            get(|| async { Redirect::to("https://platform.poool.app/auth/signup") }),
        )
        .route(
            "/marketplace",
            get(|| async { Redirect::to("https://platform.poool.app/marketplace") }),
        )
        .route(
            "/blog",
            get(|| async { Redirect::to("https://platform.poool.app/blog") }),
        )
        .route(
            "/terms",
            get(|| async { Redirect::to("https://platform.poool.app/terms") }),
        )
        .route(
            "/terms-and-conditions",
            get(|| async { Redirect::to("https://platform.poool.app/terms") }),
        )
        .route(
            "/cookies",
            get(|| async { Redirect::to("https://platform.poool.app/cookies") }),
        )
        .route(
            "/privacy-policy",
            get(|| async { Redirect::to("https://platform.poool.app/privacy-policy") }),
        )
        .route(
            "/privacy",
            get(|| async { Redirect::to("https://platform.poool.app/privacy-policy") }),
        )
        .route(
            "/currency-policy",
            get(|| async { Redirect::to("https://platform.poool.app/currency-policy") }),
        )
        .route(
            "/currency",
            get(|| async { Redirect::to("https://platform.poool.app/currency-policy") }),
        )
        .route(
            "/aml-kyc-policy",
            get(|| async { Redirect::to("https://platform.poool.app/aml-kyc-policy") }),
        )
        .route(
            "/imprint",
            get(|| async { Redirect::to("https://platform.poool.app/imprint") }),
        )
        .route(
            "/gdpr-data-request",
            get(|| async { Redirect::to("https://platform.poool.app/gdpr-data-request") }),
        )
        .route("/p/:slug", get(redirect_www_property))
        // Root → landing-v2
        .route_service("/", ServeFile::new("../frontend/platform/landing-v2.html"))
        .fallback_service(
            ServeDir::new("../frontend/www/en")
                .fallback(ServeFile::new("../frontend/platform/landing-v2.html")),
        )
        .layer(tower_http::compression::CompressionLayer::new())
        .layer(axum::middleware::from_fn(apply_security_headers));

    // ── Platform Router (login, dashboard, API) ──────────────────────
    let platform_router = Router::new()
        .nest("/auth", auth::routes::router(state.clone()))
        .route(
            "/logout",
            get(auth::routes::logout_page).post(auth::routes::logout),
        )
        // ── Domain Routers (each module owns its routes) ────────────────
        .merge(assets::router())
        .merge(kyc::router())
        .merge(rewards::router())
        .merge(leaderboard::router())
        .merge(settings::router())
        .merge(storage::router())
        .merge(wallet::router())
        .merge(cart::router())
        .merge(payments::router())
        .merge(portfolio::router())
        .merge(payment_methods::router())
        .merge(developer::router())
        .merge(legal::router())
        .merge(admin::router())
        .merge(blog::router())
        .merge(community::router())
        // ── Marketplace (trading engine APIs) ─────────────────────────
        .merge(marketplace::router())
        // ── Support (merged router handles /support and /api/support) ──
        .merge(support::router(state.clone()))
        // ── User-facing utility API ────────────────────────────────────
        .route("/api/me", get(api_me))
        .route("/api/user/legal-status", get(api_user_legal_status))
        .route("/api/user/legal-accept", post(api_user_legal_accept))
        // ── Deposit & order status polling ────────────────────────────
        .route("/api/orders/:order_id", get(api_order_detail))
        .route("/api/deposits/:deposit_id/status", get(api_deposit_status))
        // ── IPFS: Public metadata endpoint (read by smart contracts & IPFS gateways) ──
        .route(
            "/api/assets/:asset_id/metadata.json",
            get(api_asset_metadata),
        )
        // ── Featured assets JSON (for leaderboard spotlight card) ──
        .route("/api/assets/featured", get(api_assets_featured))
        // ── Reports (still in main.rs pending Phase 5 extraction) ─────
        .route("/api/admin/reports/:report_type", get(api_admin_reports))
        // ── Profile / KYC page redirect ───────────────────────────────
        .route("/profile", get(auth::routes::page_profile))
        .route("/welcome", get(auth::routes::page_welcome))
        // ── Payment result pages ──────────────────────────────────────
        .route("/payment-success", get(page_payment_success))
        .route("/payment-in-progress", get(page_payment_in_progress))
        // ── Community (demo + SSR Post Pages) ─────────────────────────
        .route("/community", get(page_community))
        .route("/community/post/:id", get(page_community_post))
        .route(
            "/community/partials/feed/list",
            get(community_feed_list_htmx),
        )
        .route(
            "/community/partials/announcements/list",
            get(community_announcements_list_htmx),
        )
        .route("/community/partials/:tab", get(community_htmx_partial))
        // ── Rewards V2 (premium layout) ───────────────────────────────
        .route("/rewards-v2", get(page_rewards_v2))
        // ── Marketplace (demo) ─────────────────────────────────────────
        .route("/marketplace-trading-v2", get(page_marketplace_trading_v2))
        .route("/marketplace-trading-v3", get(page_marketplace_trading_v3))
        .route("/marketplace-secondary", get(page_marketplace_secondary))
        .route("/my-trading", get(page_my_trading))
        .route("/trade-success", get(page_trade_success))
        .route("/tax-report", get(marketplace::routes::page_tax_report_pdf))
        // ── Design system templates (protected, MiniJinja-rendered) ────
        .route("/statistics-template.html", get(page_statistics_template))
        .route("/forms-template.html", get(page_forms_template))
        .route("/table-template.html", get(page_table_template))
        .route("/overlays-template.html", get(page_overlays_template))
        .route("/fonts-template.html", get(page_fonts_template))
        // ── Static file serving & fallbacks ───────────────────────────
        .route("/", get(handle_root))
        .nest_service("/en", ServeDir::new("../frontend/www/en"))
        .nest_service("/id", ServeDir::new("../frontend/www/id"))
        .nest_service("/fonts", ServeDir::new("../frontend/www/fonts"))
        .nest_service("/static", ServeDir::new("../frontend/platform/static"))
        .nest_service(
            "/images",
            ServeDir::new("../frontend/platform/static/images/seed")
                .fallback(ServeDir::new("../frontend/platform/static/images/ui")),
        )
        .nest_service("/uploads", ServeDir::new("../uploads"))
        .route("/health", get(handle_health))
        .fallback_service(
            ServeDir::new("../frontend/platform")
                .fallback(ServeFile::new("../frontend/platform/404.html")),
        )
        .with_state(state.clone())
        .layer(axum::extract::DefaultBodyLimit::max(25 * 1024 * 1024)) // 25 MB for file uploads
        .layer(tower_http::compression::CompressionLayer::new())
        .layer({
            let cors = tower_http::cors::CorsLayer::new()
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                ])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                ]);

            // In production/staging, restrict CORS to configured domains.
            // In development, allow any origin for local testing convenience.
            let is_dev = matches!(
                std::env::var("POOOL_ENV").as_deref(),
                Ok("development") | Ok("dev") | Ok("local")
            );

            if is_dev {
                cors.allow_origin(tower_http::cors::Any)
            } else {
                // Dynamically allow origins based on BASE_URL
                let base_url = std::env::var("BASE_URL")
                    .unwrap_or_else(|_| "https://platform.poool.app".to_string());
                let mut origins: Vec<axum::http::HeaderValue> = Vec::new();
                if let Ok(v) = base_url.parse() {
                    origins.push(v);
                }

                let base_host = base_url.replace("https://", "").replace("http://", "");
                let bare_domain = base_host.replace("platform.", "");

                let scheme = if base_url.starts_with("https") {
                    "https"
                } else {
                    "http"
                };

                let www_url = format!("{}://www.{}", scheme, bare_domain);
                if let Ok(v) = www_url.parse() {
                    origins.push(v);
                }

                let bare_url = format!("{}://{}", scheme, bare_domain);
                if let Ok(v) = bare_url.parse() {
                    origins.push(v);
                }

                cors.allow_origin(origins)
            }
        })
        .layer(tower::limit::concurrency::ConcurrencyLimitLayer::new(100))
        .layer(axum::middleware::from_fn(auth::csrf::csrf_middleware))
        .layer(axum::middleware::from_fn(apply_security_headers))
        // Sentry user context: attach user.id + email to every Sentry event
        .layer(axum::middleware::from_fn_with_state(
            state,
            sentry_user_context,
        ))
        // Sentry layers MUST be outermost (last added = wraps everything)
        .layer(sentry::integrations::tower::SentryLayer::new_from_top())
        .layer(sentry::integrations::tower::NewSentryLayer::new_from_top());

    // ── Host-based dispatch ───────────────────────────────────────────
    let app = HostDispatch {
        www: www_router,
        platform: platform_router,
    };

    //  Start server
    let addr = SocketAddr::from((
        config
            .server_host
            .parse::<std::net::IpAddr>()
            .unwrap_or([0, 0, 0, 0].into()),
        config.server_port,
    ));
    tracing::info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn apply_security_headers(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    let is_local_request = req
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|host| host.to_str().ok())
        .map(|host| {
            host.starts_with("localhost:")
                || host == "localhost"
                || host.starts_with("127.0.0.1:")
                || host == "127.0.0.1"
                || host.starts_with("[::1]:")
                || host == "[::1]"
        })
        .unwrap_or(false);

    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        axum::http::header::X_FRAME_OPTIONS,
        axum::http::HeaderValue::from_static("DENY"),
    );
    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    if !is_local_request {
        headers.insert(
            axum::http::header::STRICT_TRANSPORT_SECURITY,
            // Two-year max-age with includeSubDomains + preload — required
            // thresholds for HSTS preload list submission. Any subdomain
            // accidentally served over HTTP is caught by browsers that have
            // the preload entry (plus our existing upgrade-insecure-requests
            // CSP directive catches requests from TLS pages).
            axum::http::HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
        );
    }
    headers.insert(
        axum::http::header::CONTENT_SECURITY_POLICY,
        // BUG-003: Added https://www.youtube.com https://player.vimeo.com https://*.dropbox.com to frame-src to unblock video embeds
        // 'unsafe-eval' is required by Alpine.js v3 — it uses new Function() to parse
        // x-data/x-bind/@click string expressions. Without it Alpine loads but all
        // reactive bindings silently no-op.
        // 'unsafe-inline' remains pending a template-wide nonce rollout.
        if is_local_request {
            axum::http::HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval' blob: https://cdn.jsdelivr.net https://unpkg.com https://js.stripe.com https://browser.sentry-cdn.com https://cdnjs.cloudflare.com https://cdn.quilljs.com; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdnjs.cloudflare.com https://cdn.quilljs.com https://cdn.jsdelivr.net; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: blob: https:; connect-src 'self' http: https: ws: wss: https://*.ingest.de.sentry.io; frame-src https://js.stripe.com https://www.google.com https://www.youtube.com https://player.vimeo.com https://*.dropbox.com https://*.metabase.com; frame-ancestors 'none'; worker-src 'self' blob:; base-uri 'self'; form-action 'self';")
        } else {
            axum::http::HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval' blob: https://cdn.jsdelivr.net https://unpkg.com https://js.stripe.com https://browser.sentry-cdn.com https://cdnjs.cloudflare.com https://cdn.quilljs.com; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdnjs.cloudflare.com https://cdn.quilljs.com https://cdn.jsdelivr.net; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: blob: https:; connect-src 'self' https: wss: https://*.ingest.de.sentry.io; frame-src https://js.stripe.com https://www.google.com https://www.youtube.com https://player.vimeo.com https://*.dropbox.com https://*.metabase.com; frame-ancestors 'none'; worker-src 'self' blob:; base-uri 'self'; form-action 'self'; upgrade-insecure-requests;")
        },
    );
    headers.insert(
        axum::http::header::REFERRER_POLICY,
        // `same-origin` — cross-origin navigations (e.g. user clicks a
        // link to an external help article or OAuth provider) send NO
        // referrer at all. Prevents leaking authenticated URLs containing
        // tokens/ids to third parties while preserving same-origin UX
        // (analytics, in-app nav).
        axum::http::HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        axum::http::header::HeaderName::from_static("x-permitted-cross-domain-policies"),
        axum::http::HeaderValue::from_static("none"),
    );
    headers.insert(
        axum::http::header::HeaderName::from_static("permissions-policy"),
        axum::http::HeaderValue::from_static("accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()"),
    );

    response
}

/// Sentry user-context middleware: attaches user.id + email to every Sentry event
/// so that errors can be correlated with affected users. Skips static-asset paths
/// to avoid unnecessary DB lookups.
async fn sentry_user_context(
    State(state): State<AppState>,
    jar: CookieJar,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    let path = req.uri().path();
    // Skip static-asset paths — no session to look up
    if !path.starts_with("/static/")
        && !path.starts_with("/fonts/")
        && !path.starts_with("/images/")
        && !path.starts_with("/en/")
        && !path.starts_with("/id/")
    {
        if let Some(user) = crate::auth::middleware::get_current_user(&jar, &state.db).await {
            sentry::configure_scope(|scope| {
                scope.set_user(Some(sentry::User {
                    id: Some(user.id.to_string()),
                    email: Some(user.email.clone()),
                    ..Default::default()
                }));
            });
        }
    }
    next.run(req).await
}

//  Root redirect

//  Root redirect

/// GET /api/user/legal-status — Check whether logged-in user has accepted the current Terms version.
/// Returns { needs_reaccept: bool, current_version: "1.0", accepted_version: "1.0"|null }
/// The frontend uses this to show/hide the re-acceptance banner on every page load.
async fn api_user_legal_status(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    // Fetch current terms version from platform settings
    let current_version: String =
        sqlx::query_scalar("SELECT value FROM platform_settings WHERE key = 'legal_terms_version'")
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "1.0".to_string());

    // Fetch user's most recently accepted version
    let accepted_version: Option<String> = sqlx::query_scalar(
        "SELECT terms_version FROM user_consents WHERE user_id = $1 ORDER BY accepted_at DESC LIMIT 1"
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let needs_reaccept = accepted_version.as_deref() != Some(current_version.as_str());

    Json(serde_json::json!({
        "needs_reaccept": needs_reaccept,
        "current_version": current_version,
        "accepted_version": accepted_version,
    }))
    .into_response()
}

/// POST /api/user/legal-accept — Record user's acceptance of the current Terms version.
/// Called when user clicks "Accept" on the re-acceptance banner.
async fn api_user_legal_accept(
    jar: CookieJar,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let current_version: String =
        sqlx::query_scalar("SELECT value FROM platform_settings WHERE key = 'legal_terms_version'")
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "1.0".to_string());

    let result = sqlx::query(
        "INSERT INTO user_consents (user_id, terms_version, ip_address, user_agent) VALUES ($1, $2, $3, $4)"
    )
    .bind(user.id)
    .bind(&current_version)
    .bind(ip.as_deref())
    .bind(user_agent.as_deref())
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            Json(serde_json::json!({"status": "success", "accepted_version": current_version}))
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to record legal acceptance: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to record acceptance"})),
            )
                .into_response()
        }
    }
}

/// GET /api/admin/reports/:report_type — Generate aggregated report data with date filtering.
///
/// Supports report types: financial-summary, user-growth, kyc-status, asset-performance,
/// aml-compliance, rewards-liability, referral-effectiveness, multi-currency, invoice-summary,
/// deposit-summary, order-summary, investment-summary, support-summary, audit-summary.
///
/// Query params: ?from=YYYY-MM-DD&to=YYYY-MM-DD
async fn api_admin_reports(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(report_type): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    headers: axum::http::HeaderMap,
) -> Result<axum::response::Response, ApiError> {
    let report = resolve_admin_report(&report_type)?;
    require_admin_report_permissions(&admin, &state.db, report).await?;

    let date_range = parse_admin_report_date_range(
        params.get("from").map(String::as_str),
        params.get("to").map(String::as_str),
    )?;
    let is_preview = params.get("mode").is_some_and(|mode| mode == "preview");
    let date_from = date_range
        .from
        .map(|date| date.to_string())
        .unwrap_or_default();
    let date_to = date_range
        .to
        .map(|date| date.to_string())
        .unwrap_or_default();

    use sqlx::Row;

    let data: Vec<serde_json::Value> = match report.canonical_type {
        "financial-summary" | "monthly-financial" => {
            let rows = sqlx::query(
                r#"SELECT
                     TO_CHAR(created_at, 'YYYY-MM') as month,
                     type as transaction_type,
                     status,
                     currency,
                     COUNT(*) as tx_count,
                     COALESCE(SUM(amount_cents), 0)::bigint as total_cents
                   FROM wallet_transactions
                   WHERE ($1::date IS NULL OR created_at >= $1::date)
                     AND ($2::date IS NULL OR created_at <= ($2::date + interval '1 day'))
                   GROUP BY month, type, status, currency
                   ORDER BY month DESC, type"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "month": r.get::<String, _>("month"),
                        "transaction_type": r.get::<String, _>("transaction_type"),
                        "status": r.get::<String, _>("status"),
                        "currency": r.get::<String, _>("currency"),
                        "tx_count": r.get::<i64, _>("tx_count"),
                        "total_cents": r.get::<i64, _>("total_cents"),
                    })
                })
                .collect()
        }

        "user-growth" => {
            let rows = sqlx::query(
                r#"SELECT
                     TO_CHAR(created_at, 'YYYY-MM-DD') as signup_date,
                     COUNT(*) as signups,
                     SUM(CASE WHEN email_verified THEN 1 ELSE 0 END) as verified
                   FROM users
                   WHERE ($1::date IS NULL OR created_at >= $1::date)
                     AND ($2::date IS NULL OR created_at <= ($2::date + interval '1 day'))
                   GROUP BY signup_date
                   ORDER BY signup_date DESC"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "date": r.get::<String, _>("signup_date"),
                        "signups": r.get::<i64, _>("signups"),
                        "verified": r.get::<i64, _>("verified"),
                    })
                })
                .collect()
        }

        "kyc" | "kyc-status" => {
            let rows = sqlx::query(
                r#"SELECT
                     k.id::text, u.email, k.status, k.document_type,
                     k.pep_check_passed, k.sanctions_check,
                     k.rejection_reason,
                     k.created_at::text, k.verified_at::text, k.expires_at::text
                   FROM kyc_records k
                   JOIN users u ON k.user_id = u.id
                   WHERE ($1::date IS NULL OR k.created_at >= $1::date)
                     AND ($2::date IS NULL OR k.created_at <= ($2::date + interval '1 day'))
                   ORDER BY k.created_at DESC"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "email": r.get::<String, _>("email"),
                        "status": r.get::<String, _>("status"),
                        "document_type": r.get::<Option<String>, _>("document_type"),
                        "pep_check_passed": r.get::<Option<bool>, _>("pep_check_passed"),
                        "sanctions_check": r.get::<Option<bool>, _>("sanctions_check"),
                        "rejection_reason": r.get::<Option<String>, _>("rejection_reason"),
                        "created_at": r.get::<String, _>("created_at"),
                        "verified_at": r.get::<Option<String>, _>("verified_at"),
                        "expires_at": r.get::<Option<String>, _>("expires_at"),
                    })
                })
                .collect()
        }

        "aml" | "aml-compliance" => {
            let rows = sqlx::query(
                r#"SELECT
                     k.id::text, u.email,
                     COALESCE(p.first_name, '') || ' ' || COALESCE(p.last_name, '') as full_name,
                     k.status, k.pep_check_passed, k.sanctions_check,
                     k.rejection_reason, k.created_at::text
                   FROM kyc_records k
                   JOIN users u ON k.user_id = u.id
                   LEFT JOIN user_profiles p ON k.user_id = p.user_id
                   WHERE (k.pep_check_passed = true OR k.sanctions_check = true
                          OR k.status = 'rejected')
                     AND ($1::date IS NULL OR k.created_at >= $1::date)
                     AND ($2::date IS NULL OR k.created_at <= ($2::date + interval '1 day'))
                   ORDER BY k.created_at DESC"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "email": r.get::<String, _>("email"),
                        "full_name": r.get::<String, _>("full_name"),
                        "status": r.get::<String, _>("status"),
                        "pep_check_passed": r.get::<Option<bool>, _>("pep_check_passed"),
                        "sanctions_check": r.get::<Option<bool>, _>("sanctions_check"),
                        "rejection_reason": r.get::<Option<String>, _>("rejection_reason"),
                        "created_at": r.get::<String, _>("created_at"),
                    })
                })
                .collect()
        }

        "assets" | "asset-performance" => {
            let rows = sqlx::query(
                r#"SELECT
                     a.id::text, a.title, a.asset_type, a.funding_status,
                     a.total_value_cents, a.token_price_cents,
                     a.tokens_total, a.tokens_available,
                     a.annual_yield_bps, a.capital_appreciation_bps,
                     a.occupancy_rate_bps, a.featured, a.published,
                     a.created_at::text
                   FROM assets a
                   WHERE ($1::date IS NULL OR a.created_at >= $1::date)
                     AND ($2::date IS NULL OR a.created_at <= ($2::date + interval '1 day'))
                   ORDER BY a.created_at DESC"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    let total: i32 = r.get::<Option<i32>, _>("tokens_total").unwrap_or(0);
                    let available: i32 = r.get::<Option<i32>, _>("tokens_available").unwrap_or(0);
                    let pct = if total > 0 {
                        (total - available) as f64 / total as f64 * 100.0
                    } else {
                        0.0
                    };
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "title": r.get::<Option<String>, _>("title"),
                        "asset_type": r.get::<Option<String>, _>("asset_type"),
                        "funding_status": r.get::<Option<String>, _>("funding_status"),
                        "total_value_cents": r.get::<Option<i64>, _>("total_value_cents"),
                        "token_price_cents": r.get::<Option<i64>, _>("token_price_cents"),
                        "tokens_total": total,
                        "tokens_available": available,
                        "pct_funded": format!("{:.1}", pct),
                        "annual_yield_bps": r.get::<Option<i32>, _>("annual_yield_bps"),
                        "featured": r.get::<Option<bool>, _>("featured"),
                        "published": r.get::<Option<bool>, _>("published"),
                        "created_at": r.get::<String, _>("created_at"),
                    })
                })
                .collect()
        }

        "investments" | "investment-summary" => {
            let rows = sqlx::query(
                r#"SELECT
                     i.id::text, u.email,
                     a.title as asset_title,
                     i.tokens_owned, i.purchase_value_cents, i.current_value_cents,
                     i.total_rental_cents, i.status, i.purchased_at::text as created_at
                   FROM investments i
                   JOIN users u ON i.user_id = u.id
                   JOIN assets a ON i.asset_id = a.id
                   WHERE ($1::date IS NULL OR i.purchased_at >= $1::date)
                     AND ($2::date IS NULL OR i.purchased_at <= ($2::date + interval '1 day'))
                   ORDER BY i.purchased_at DESC"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "email": r.get::<String, _>("email"),
                        "asset_title": r.get::<Option<String>, _>("asset_title"),
                        "tokens_owned": r.get::<Option<i32>, _>("tokens_owned"),
                        "purchase_value_cents": r.get::<Option<i64>, _>("purchase_value_cents"),
                        "current_value_cents": r.get::<Option<i64>, _>("current_value_cents"),
                        "total_rental_cents": r.get::<Option<i64>, _>("total_rental_cents"),
                        "status": r.get::<Option<String>, _>("status"),
                        "created_at": r.get::<String, _>("created_at"),
                    })
                })
                .collect()
        }

        "tax-pl" | "tax-reporting" => {
            let rows = sqlx::query(
                r#"SELECT
                     u.email, t.fiscal_year as year, t.total_dividends_cents as total_profit_cents, 
                     t.capital_gains_cents, t.withholding_tax_cents, t.status, 
                     t.generated_at::text, t.pdf_url as report_url
                   FROM tax_reports t
                   JOIN users u ON t.user_id = u.id
                   WHERE ($1::date IS NULL OR t.created_at >= $1::date)
                     AND ($2::date IS NULL OR t.created_at <= ($2::date + interval '1 day'))
                   ORDER BY t.created_at DESC"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "email": r.get::<String, _>("email"),
                        "year": r.get::<i32, _>("year"),
                        "total_profit_cents": r.get::<i64, _>("total_profit_cents"),
                        "capital_gains_cents": r.get::<i64, _>("capital_gains_cents"),
                        "withholding_tax_cents": r.get::<i64, _>("withholding_tax_cents"),
                        "status": r.get::<String, _>("status"),
                        "generated_at": r.get::<Option<String>, _>("generated_at"),
                        "report_url": r.get::<Option<String>, _>("report_url"),
                    })
                })
                .collect()
        }

        "tax-withholding" => {
            let rows = sqlx::query(
                r#"SELECT
                     u.email, t.fiscal_year as year, t.withholding_tax_cents, t.status, 
                     t.generated_at::text, t.pdf_url as report_url
                   FROM tax_reports t
                   JOIN users u ON t.user_id = u.id
                   WHERE ($1::date IS NULL OR t.created_at >= $1::date)
                     AND ($2::date IS NULL OR t.created_at <= ($2::date + interval '1 day'))
                   ORDER BY t.created_at DESC"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "email": r.get::<String, _>("email"),
                        "year": r.get::<i32, _>("year"),
                        "withholding_tax_cents": r.get::<i64, _>("withholding_tax_cents"),
                        "status": r.get::<String, _>("status"),
                        "generated_at": r.get::<Option<String>, _>("generated_at"),
                        "report_url": r.get::<Option<String>, _>("report_url"),
                    })
                })
                .collect()
        }

        "orders" | "order-summary" => {
            let rows = sqlx::query(
                r#"SELECT
                     o.id::text, o.order_number, u.email,
                     o.total_cents, o.currency, o.status,
                     o.payment_method, o.payment_ref_id,
                     o.created_at::text, o.completed_at::text
                   FROM orders o
                   JOIN users u ON o.user_id = u.id
                   WHERE ($1::date IS NULL OR o.created_at >= $1::date)
                     AND ($2::date IS NULL OR o.created_at <= ($2::date + interval '1 day'))
                   ORDER BY o.created_at DESC"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "order_number": r.get::<Option<String>, _>("order_number"),
                        "email": r.get::<String, _>("email"),
                        "total_cents": r.get::<Option<i64>, _>("total_cents"),
                        "currency": r.get::<Option<String>, _>("currency"),
                        "status": r.get::<Option<String>, _>("status"),
                        "payment_method": r.get::<Option<String>, _>("payment_method"),
                        "payment_ref_id": r.get::<Option<String>, _>("payment_ref_id"),
                        "created_at": r.get::<String, _>("created_at"),
                        "completed_at": r.get::<Option<String>, _>("completed_at"),
                    })
                })
                .collect()
        }

        "rewards-liability" => {
            let rows = sqlx::query(
                r#"SELECT
                     u.email,
                     rb.cashback, rb.referrals, rb.promotions,
                     (rb.cashback + rb.referrals + rb.promotions) as total_cents,
                     t.name as tier_name,
                     ut.invested_12m
                   FROM rewards_balances rb
                   JOIN users u ON rb.user_id = u.id
                   LEFT JOIN user_tiers ut ON rb.user_id = ut.user_id
                   LEFT JOIN tiers t ON ut.tier_id = t.id
                   WHERE ($1::date IS NULL OR rb.updated_at >= $1::date)
                     AND ($2::date IS NULL OR rb.updated_at <= ($2::date + interval '1 day'))
                   ORDER BY (rb.cashback + rb.referrals + rb.promotions) DESC"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "email": r.get::<String, _>("email"),
                        "cashback_cents": r.get::<Option<i64>, _>("cashback"),
                        "referral_cents": r.get::<Option<i64>, _>("referrals"),
                        "promotion_cents": r.get::<Option<i64>, _>("promotions"),
                        "total_cents": r.get::<Option<i64>, _>("total_cents"),
                        "tier": r.get::<Option<String>, _>("tier_name"),
                        "invested_12m_cents": r.get::<Option<i64>, _>("invested_12m"),
                    })
                })
                .collect()
        }

        "referral-effectiveness" => {
            let rows = sqlx::query(
                r#"SELECT
                     u1.email as referrer_email,
                     u2.email as referred_email,
                     rt.status,
                     rt.referrer_reward, rt.referred_reward,
                     rt.created_at::text, rt.qualified_at::text
                   FROM referral_tracking rt
                   JOIN users u1 ON rt.referrer_id = u1.id
                   JOIN users u2 ON rt.referred_id = u2.id
                   WHERE ($1::date IS NULL OR rt.created_at >= $1::date)
                     AND ($2::date IS NULL OR rt.created_at <= ($2::date + interval '1 day'))
                   ORDER BY rt.created_at DESC"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "referrer_email": r.get::<String, _>("referrer_email"),
                        "referred_email": r.get::<String, _>("referred_email"),
                        "status": r.get::<String, _>("status"),
                        "referrer_reward_cents": r.get::<i64, _>("referrer_reward"),
                        "referred_reward_cents": r.get::<i64, _>("referred_reward"),
                        "created_at": r.get::<String, _>("created_at"),
                        "qualified_at": r.get::<Option<String>, _>("qualified_at"),
                    })
                })
                .collect()
        }

        "multi-currency" => {
            let rows = sqlx::query(
                r#"SELECT
                     currency,
                     wallet_type,
                     COUNT(*) as wallet_count,
                     COALESCE(SUM(balance_cents), 0)::bigint as total_balance_cents
                   FROM wallets
                   GROUP BY currency, wallet_type
                   ORDER BY currency, wallet_type"#,
            )
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "currency": r.get::<String, _>("currency"),
                        "wallet_type": r.get::<String, _>("wallet_type"),
                        "wallet_count": r.get::<i64, _>("wallet_count"),
                        "total_balance_cents": r.get::<i64, _>("total_balance_cents"),
                    })
                })
                .collect()
        }

        "invoices" | "invoice-summary" => {
            let rows = sqlx::query(
                r#"SELECT
                     i.id::text, i.invoice_number, u.email,
                     i.subtotal_cents, i.tax_cents, i.total_cents,
                     i.currency, i.status, i.issued_at::text
                   FROM invoices i
                   JOIN orders o ON i.order_id = o.id
                   JOIN users u ON o.user_id = u.id
                   WHERE ($1::date IS NULL OR i.issued_at >= $1::date)
                     AND ($2::date IS NULL OR i.issued_at <= ($2::date + interval '1 day'))
                   ORDER BY i.issued_at DESC"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "invoice_number": r.get::<Option<String>, _>("invoice_number"),
                        "email": r.get::<String, _>("email"),
                        "subtotal_cents": r.get::<Option<i64>, _>("subtotal_cents"),
                        "tax_cents": r.get::<Option<i64>, _>("tax_cents"),
                        "total_cents": r.get::<Option<i64>, _>("total_cents"),
                        "currency": r.get::<Option<String>, _>("currency"),
                        "status": r.get::<Option<String>, _>("status"),
                        "issued_at": r.get::<Option<String>, _>("issued_at"),
                    })
                })
                .collect()
        }

        "support" | "support-summary" => {
            let rows = sqlx::query(
                r#"SELECT
                     t.id::text, t.subject, u.email,
                     t.status, t.priority,
                     t.created_at::text, t.updated_at::text
                   FROM support_tickets t
                   JOIN users u ON t.user_id = u.id
                   WHERE ($1::date IS NULL OR t.created_at >= $1::date)
                     AND ($2::date IS NULL OR t.created_at <= ($2::date + interval '1 day'))
                   ORDER BY t.created_at DESC"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "subject": r.get::<String, _>("subject"),
                        "email": r.get::<String, _>("email"),
                        "status": r.get::<String, _>("status"),
                        "priority": r.get::<String, _>("priority"),
                        "created_at": r.get::<String, _>("created_at"),
                        "updated_at": r.get::<Option<String>, _>("updated_at"),
                    })
                })
                .collect()
        }

        "audit" | "audit-summary" => {
            let rows = sqlx::query(
                r#"SELECT
                     a.id::text,
                     COALESCE(u.email, 'system') as actor_email,
                     a.action, a.entity_type, a.entity_id::text,
                     a.ip_address::text, a.created_at::text
                   FROM audit_logs a
                   LEFT JOIN users u ON a.actor_user_id = u.id
                   WHERE ($1::date IS NULL OR a.created_at >= $1::date)
                     AND ($2::date IS NULL OR a.created_at <= ($2::date + interval '1 day'))
                   ORDER BY a.created_at DESC
                   LIMIT 5000"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "actor_email": r.get::<String, _>("actor_email"),
                        "action": r.get::<Option<String>, _>("action"),
                        "entity_type": r.get::<Option<String>, _>("entity_type"),
                        "entity_id": r.get::<Option<String>, _>("entity_id"),
                        "ip_address": r.get::<Option<String>, _>("ip_address"),
                        "created_at": r.get::<String, _>("created_at"),
                    })
                })
                .collect()
        }

        "wallet-transactions" => {
            let rows = sqlx::query(
                r#"SELECT
                     wt.id::text, u.email,
                     w.wallet_type, wt.type as tx_type,
                     wt.status, wt.amount_cents, wt.currency,
                     wt.description, wt.external_ref_id,
                     wt.created_at::text
                   FROM wallet_transactions wt
                   JOIN wallets w ON wt.wallet_id = w.id
                   JOIN users u ON w.user_id = u.id
                   WHERE ($1::date IS NULL OR wt.created_at >= $1::date)
                     AND ($2::date IS NULL OR wt.created_at <= ($2::date + interval '1 day'))
                   ORDER BY wt.created_at DESC
                   LIMIT 5000"#,
            )
            .bind(date_range.from)
            .bind(date_range.to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| report_query_error(report.canonical_type, e))?;

            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "email": r.get::<String, _>("email"),
                        "wallet_type": r.get::<String, _>("wallet_type"),
                        "tx_type": r.get::<String, _>("tx_type"),
                        "status": r.get::<String, _>("status"),
                        "amount_cents": r.get::<i64, _>("amount_cents"),
                        "currency": r.get::<String, _>("currency"),
                        "description": r.get::<Option<String>, _>("description"),
                        "external_ref_id": r.get::<Option<String>, _>("external_ref_id"),
                        "created_at": r.get::<String, _>("created_at"),
                    })
                })
                .collect()
        }

        _ => return Err(ApiError::BadRequest("Unknown report type".to_string())),
    };

    if !is_preview {
        log_admin_report_export(
            &state.db,
            &admin,
            report,
            &date_from,
            &date_to,
            data.len(),
            headers
                .get(axum::http::header::USER_AGENT)
                .and_then(|value| value.to_str().ok()),
        )
        .await?;
    }

    Ok(Json(serde_json::json!({
        "report_type": report.canonical_type,
        "date_from": date_from,
        "date_to": date_to,
        "rows": data
    }))
    .into_response())
}

#[derive(Clone, Copy)]
struct AdminReportDefinition {
    canonical_type: &'static str,
    aliases: &'static [&'static str],
    required_permissions: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct AdminReportDateRange {
    from: Option<chrono::NaiveDate>,
    to: Option<chrono::NaiveDate>,
}

const ADMIN_REPORTS: &[AdminReportDefinition] = &[
    AdminReportDefinition {
        canonical_type: "financial-summary",
        aliases: &["monthly-financial"],
        required_permissions: &["treasury.read"],
    },
    AdminReportDefinition {
        canonical_type: "wallet-transactions",
        aliases: &[],
        required_permissions: &["treasury.read"],
    },
    AdminReportDefinition {
        canonical_type: "invoice-summary",
        aliases: &["invoices"],
        required_permissions: &["treasury.read"],
    },
    AdminReportDefinition {
        canonical_type: "multi-currency",
        aliases: &[],
        required_permissions: &["treasury.read"],
    },
    AdminReportDefinition {
        canonical_type: "user-growth",
        aliases: &[],
        required_permissions: &[],
    },
    AdminReportDefinition {
        canonical_type: "kyc-status",
        aliases: &["kyc"],
        required_permissions: &["kyc.read"],
    },
    AdminReportDefinition {
        canonical_type: "audit-summary",
        aliases: &["audit"],
        required_permissions: &["audit.read"],
    },
    AdminReportDefinition {
        canonical_type: "aml-compliance",
        aliases: &["aml"],
        required_permissions: &["kyc.read"],
    },
    AdminReportDefinition {
        canonical_type: "investment-summary",
        aliases: &["investments"],
        required_permissions: &["treasury.read"],
    },
    AdminReportDefinition {
        canonical_type: "asset-performance",
        aliases: &["assets"],
        required_permissions: &[],
    },
    AdminReportDefinition {
        canonical_type: "order-summary",
        aliases: &["orders"],
        required_permissions: &["treasury.read"],
    },
    AdminReportDefinition {
        canonical_type: "rewards-liability",
        aliases: &[],
        required_permissions: &[],
    },
    AdminReportDefinition {
        canonical_type: "referral-effectiveness",
        aliases: &[],
        required_permissions: &[],
    },
    AdminReportDefinition {
        canonical_type: "support-summary",
        aliases: &["support"],
        required_permissions: &["support.read"],
    },
    AdminReportDefinition {
        canonical_type: "tax-reporting",
        aliases: &["tax-pl"],
        required_permissions: &["treasury.read"],
    },
    AdminReportDefinition {
        canonical_type: "tax-withholding",
        aliases: &[],
        required_permissions: &["treasury.read"],
    },
];

fn resolve_admin_report(report_type: &str) -> Result<&'static AdminReportDefinition, ApiError> {
    ADMIN_REPORTS
        .iter()
        .find(|report| {
            report.canonical_type == report_type || report.aliases.contains(&report_type)
        })
        .ok_or_else(|| {
            ApiError::BadRequest(format!(
                "Unknown report type. Available types: {}",
                ADMIN_REPORTS
                    .iter()
                    .map(|report| report.canonical_type)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })
}

async fn require_admin_report_permissions(
    admin: &AdminUser,
    pool: &sqlx::PgPool,
    report: &AdminReportDefinition,
) -> Result<(), ApiError> {
    admin.require_permission(pool, "reports.generate").await?;
    for permission in report.required_permissions {
        admin.require_permission(pool, permission).await?;
    }
    Ok(())
}

fn parse_admin_report_date_range(
    from: Option<&str>,
    to: Option<&str>,
) -> Result<AdminReportDateRange, ApiError> {
    fn parse_date(value: Option<&str>, name: &str) -> Result<Option<chrono::NaiveDate>, ApiError> {
        let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
            return Ok(None);
        };
        chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| ApiError::BadRequest(format!("Invalid {} date. Use YYYY-MM-DD.", name)))
    }

    let from = parse_date(from, "from")?;
    let to = parse_date(to, "to")?;
    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(ApiError::BadRequest(
                "Invalid date range: from must be on or before to.".to_string(),
            ));
        }
    }
    Ok(AdminReportDateRange { from, to })
}

fn report_query_error(report_type: &str, error: sqlx::Error) -> ApiError {
    tracing::error!("Failed to generate admin report {}: {}", report_type, error);
    ApiError::Database(error)
}

async fn log_admin_report_export(
    pool: &sqlx::PgPool,
    admin: &AdminUser,
    report: &AdminReportDefinition,
    date_from: &str,
    date_to: &str,
    row_count: usize,
    user_agent: Option<&str>,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state, user_agent, metadata)
           VALUES ($1, 'report.exported', 'admin_report', $2, $3, $4)"#,
    )
    .bind(admin.user.id)
    .bind(serde_json::json!({
        "report_type": report.canonical_type,
        "date_from": date_from,
        "date_to": date_to,
        "row_count": row_count,
    }))
    .bind(user_agent.unwrap_or(""))
    .bind(serde_json::json!({
        "report_type": report.canonical_type,
        "row_count": row_count,
    }))
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!(
            "Failed to audit admin report export {} for actor {}: {}",
            report.canonical_type,
            admin.user.id,
            e
        );
        ApiError::Database(e)
    })?;

    Ok(())
}

/// GET /api/assets/:asset_id/metadata.json — public ERC-1155 metadata endpoint.
///
/// This is the URI that the smart contract's `uri()` function returns.
/// It MUST be publicly accessible (no auth) because IPFS gateways, indexers,
/// and wallets (e.g., MetaMask, OpenSea) need to fetch it.
///
/// Returns the full ERC-1155 compliant metadata JSON for the asset,
/// built from the database. If the metadata has already been pinned to IPFS,
/// the IPFS CID is included in the response headers.
async fn api_asset_metadata(
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
) -> impl IntoResponse {
    let asset_uuid = match uuid::Uuid::parse_str(&asset_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid asset ID"})),
            )
                .into_response()
        }
    };

    match ipfs::metadata::build_metadata(&state.db, asset_uuid).await {
        Ok(metadata) => {
            match ipfs::metadata::metadata_to_json(&metadata) {
                Ok(json) => {
                    // Strong cache headers — metadata changes rarely
                    let mut response = Json(json).into_response();
                    response.headers_mut().insert(
                        axum::http::header::CACHE_CONTROL,
                        axum::http::HeaderValue::from_static(
                            "public, max-age=3600, s-maxage=86400",
                        ),
                    );
                    response.headers_mut().insert(
                        axum::http::header::CONTENT_TYPE,
                        axum::http::HeaderValue::from_static("application/json"),
                    );
                    response
                }
                Err(e) => (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e})),
                )
                    .into_response(),
            }
        }
        Err(e) => {
            if e.contains("not found") {
                (
                    axum::http::StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": e})),
                )
                    .into_response()
            } else {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e})),
                )
                    .into_response()
            }
        }
    }
}

/// GET /api/assets/featured — returns published featured assets as JSON.
/// Used by the leaderboard spotlight card to rotate through featured investments.
async fn api_assets_featured(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    use sqlx::Row;

    if !crate::auth::middleware::is_authenticated(&jar, &state.db).await {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        )
            .into_response();
    }

    let rows = sqlx::query(
        r#"SELECT
             a.id::text, a.title, a.slug, a.asset_type,
             a.location_city, a.location_country,
             a.total_value_cents, a.token_price_cents,
             a.tokens_total, a.tokens_available,
             a.annual_yield_bps, a.capital_appreciation_bps,
             a.funding_status, a.term_months,
             (SELECT image_url FROM asset_images WHERE asset_id = a.id ORDER BY is_cover DESC, created_at ASC LIMIT 1) as cover_image
           FROM assets a
           WHERE a.published = true
             AND a.featured = true
             AND a.funding_status IN ('funding_open', 'funding_in_progress')
           ORDER BY a.created_at DESC
           LIMIT 10"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let assets: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let total: i32 = r.get::<Option<i32>, _>("tokens_total").unwrap_or(0);
            let available: i32 = r.get::<Option<i32>, _>("tokens_available").unwrap_or(0);
            let funded_pct = if total > 0 {
                (total - available) as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            let cover = r
                .get::<Option<String>, _>("cover_image")
                .map(|u| crate::storage::service::rewrite_gcs_url(&u))
                .unwrap_or_else(|| "/images/villa1.webp".to_string());

            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "title": r.get::<Option<String>, _>("title"),
                "slug": r.get::<Option<String>, _>("slug"),
                "asset_type": r.get::<Option<String>, _>("asset_type"),
                "location_city": r.get::<Option<String>, _>("location_city"),
                "location_country": r.get::<Option<String>, _>("location_country"),
                "total_value_cents": r.get::<Option<i64>, _>("total_value_cents"),
                "token_price_cents": r.get::<Option<i64>, _>("token_price_cents"),
                "annual_yield_bps": r.get::<Option<i32>, _>("annual_yield_bps"),
                "capital_appreciation_bps": r.get::<Option<i32>, _>("capital_appreciation_bps"),
                "funding_status": r.get::<Option<String>, _>("funding_status"),
                "term_months": r.get::<Option<i32>, _>("term_months"),
                "funded_pct": format!("{:.1}", funded_pct),
                "cover_image": cover,
            })
        })
        .collect();

    Json(serde_json::json!({ "assets": assets })).into_response()
}

/// GET /api/orders/:order_id — returns order + items for the logged-in user
async fn api_order_detail(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> impl IntoResponse {
    use sqlx::Row;
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    let order = sqlx::query(
        "SELECT id, order_number, total_cents, currency, payment_method, status, created_at FROM orders WHERE id::text = $1 AND user_id = $2"
    )
        .bind(&order_id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await;

    match order {
        Ok(Some(row)) => {
            let order_uuid: uuid::Uuid = row.get("id");
            let items_rows = sqlx::query(
                "SELECT oi.tokens_quantity, oi.token_price_cents, oi.subtotal_cents, a.title as asset_title FROM order_items oi LEFT JOIN assets a ON oi.asset_id = a.id WHERE oi.order_id = $1"
            )
                .bind(order_uuid)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();

            let items: Vec<serde_json::Value> = items_rows
                .iter()
                .map(|ir| {
                    serde_json::json!({
                        "asset_title": ir.get::<Option<String>, _>("asset_title"),
                        "tokens_quantity": ir.get::<i32, _>("tokens_quantity"),
                        "token_price_cents": ir.get::<i64, _>("token_price_cents"),
                        "subtotal_cents": ir.get::<i64, _>("subtotal_cents"),
                    })
                })
                .collect();

            // Check for invoice
            let invoice = sqlx::query("SELECT pdf_url FROM invoices WHERE order_id = $1 LIMIT 1")
                .bind(order_uuid)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten();

            Json(serde_json::json!({
                "id": order_id,
                "order_number": row.get::<Option<String>, _>("order_number"),
                "total_cents": row.get::<i64, _>("total_cents"),
                "currency": row.get::<Option<String>, _>("currency"),
                "payment_method": row.get::<Option<String>, _>("payment_method"),
                "status": row.get::<Option<String>, _>("status"),
                "created_at": row.get::<Option<String>, _>("created_at"),
                "items": items,
                "invoice_url": invoice.and_then(|i| i.get::<Option<String>, _>("pdf_url")),
            }))
            .into_response()
        }
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Order not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to load order: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Server error"})),
            )
                .into_response()
        }
    }
}

/// GET /api/deposits/:deposit_id/status — returns deposit request status for polling
async fn api_deposit_status(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(deposit_id): Path<String>,
) -> impl IntoResponse {
    use sqlx::Row;
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    let deposit = sqlx::query(
        "SELECT id::text, amount_cents, currency, status, provider, provider_reference, order_id::text AS order_id, created_at::text AS created_at FROM deposit_requests WHERE id::text = $1 AND user_id = $2"
    )
        .bind(&deposit_id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await;

    match deposit {
        Ok(Some(row)) => Json(serde_json::json!({
            "id": deposit_id,
            "amount_cents": row.get::<i64, _>("amount_cents"),
            "currency": row.get::<Option<String>, _>("currency"),
            "status": row.get::<Option<String>, _>("status"),
            "provider": row.get::<Option<String>, _>("provider"),
            "provider_reference": row.get::<Option<String>, _>("provider_reference"),
            "order_id": row.get::<Option<String>, _>("order_id"),
            "created_at": row.get::<Option<String>, _>("created_at"),
        }))
        .into_response(),
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Deposit not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to load deposit: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Server error"})),
            )
                .into_response()
        }
    }
}

/// GET /health (www router) — simple liveness probe without DB/Redis checks.
async fn handle_health_basic() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn redirect_www_property(Path(slug): Path<String>) -> Redirect {
    Redirect::to(&format!("https://platform.poool.app/p/{slug}"))
}

/// GET /health (platform router) — Health check endpoint for Cloud Run and uptime monitors.
///
/// Probes the database (via `SELECT 1`) and Redis (via `PING`) to determine
/// system health. Returns 200 if the DB is reachable, 503 otherwise.
/// Redis is optional — if not configured, health is still "ok".
async fn handle_health(State(state): State<AppState>) -> impl IntoResponse {
    let mut db_ok = false;
    let mut redis_status = "not_configured";

    // ── DB probe ────────────────────────────────────────────────
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => db_ok = true,
        Err(e) => {
            tracing::error!("Health check: DB probe failed: {}", e);
        }
    }

    // ── Redis probe (optional) ──────────────────────────────────
    if let Some(ref redis_pool) = state.redis {
        match redis_pool.get().await {
            Ok(mut conn) => {
                let ping_result: Result<String, redis::RedisError> =
                    redis::cmd("PING").query_async(&mut *conn).await;
                match ping_result {
                    Ok(_) => redis_status = "ok",
                    Err(e) => {
                        tracing::error!("Health check: Redis PING failed: {}", e);
                        redis_status = "error";
                    }
                }
            }
            Err(e) => {
                tracing::error!("Health check: Redis pool connection failed: {}", e);
                redis_status = "error";
            }
        }
    }

    // ── Critical-env presence (boolean only — never leak the value) ──
    let env_present = |k: &str| {
        std::env::var(k)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    };
    let totp_key_ok = env_present("TOTP_SECRET_ENCRYPTION_KEY") || env_present("ENCRYPTION_KEY");
    let session_secret_ok = env_present("SESSION_SECRET") || env_present("JWT_SECRET");

    let overall_status = if db_ok && totp_key_ok && session_secret_ok {
        "ok"
    } else {
        "degraded"
    };

    let body = serde_json::json!({
        "status": overall_status,
        "version": env!("CARGO_PKG_VERSION"),
        "app_env": std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
        "components": {
            "database": if db_ok { "ok" } else { "error" },
            "redis": redis_status,
            "env": {
                "TOTP_SECRET_ENCRYPTION_KEY_OR_ENCRYPTION_KEY": if totp_key_ok { "ok" } else { "missing" },
                "SESSION_SECRET_OR_JWT_SECRET": if session_secret_ok { "ok" } else { "missing" },
            },
        }
    });

    if db_ok {
        (axum::http::StatusCode::OK, Json(body)).into_response()
    } else {
        (axum::http::StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
    }
}

/// Root `/` redirect (platform only). If authenticated, go to marketplace; otherwise go to login.
async fn handle_root(jar: CookieJar, State(state): State<AppState>) -> Redirect {
    if crate::auth::middleware::is_authenticated(&jar, &state.db).await {
        Redirect::to("/marketplace")
    } else {
        Redirect::to("/auth/login")
    }
}

// ── Host-based routing service ────────────────────────────────────────────
//
// Dispatches incoming requests to the correct router based on the `Host` header:
//   • www.poool.app     → www_router (landing page)
//   • poool.app         → 301 redirect to https://www.poool.app{path}
//   • platform.poool.app / localhost / anything else → platform_router

/// A tower `Service` that dispatches to either the www or platform router
/// based on the HTTP `Host` header.
#[derive(Clone)]
struct HostDispatch {
    www: Router,
    platform: Router,
}

impl tower::Service<axum::http::Request<axum::body::Body>> for HostDispatch {
    type Response = axum::response::Response;
    type Error = std::convert::Infallible;
    type Future =
        Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: axum::http::Request<axum::body::Body>) -> Self::Future {
        let host = req
            .headers()
            .get(axum::http::header::HOST)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("")
            .to_lowercase();

        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "https://platform.poool.app".to_string());
        let base_host = base_url.replace("https://", "").replace("http://", "");
        let bare_domain = base_host.replace("platform.", "");
        let www_domain = format!("www.{}", bare_domain);

        if host == bare_domain {
            // Bare domain → permanent redirect to www
            let path_and_query = req
                .uri()
                .path_and_query()
                .map(|pq| pq.as_str().to_string())
                .unwrap_or_else(|| "/".to_string());
            Box::pin(async move {
                Ok(
                    Redirect::permanent(&format!("https://{}{}", www_domain, path_and_query))
                        .into_response(),
                )
            })
        } else if host == www_domain {
            // WWW → landing page
            let mut router = self.www.clone();
            Box::pin(async move {
                let resp = tower::Service::call(&mut router, req).await;
                Ok(resp.into_response())
            })
        } else if host == "localhost" {
            // localhost dev: landing-page asset paths → www router, platform paths → platform router
            let path = req.uri().path().to_string();
            let is_www_path = path == "/"
                || path.starts_with("/en")
                || path.starts_with("/id")
                || path.starts_with("/webp")
                || path.starts_with("/png")
                || path.starts_with("/svg")
                || path.starts_with("/webm")
                || path == "/fonts"
                || path.starts_with("/fonts/")
                || path == "/robots.txt"
                || path == "/sitemap.xml";
            if is_www_path {
                let mut router = self.www.clone();
                Box::pin(async move {
                    let resp = tower::Service::call(&mut router, req).await;
                    Ok(resp.into_response())
                })
            } else {
                let mut router = self.platform.clone();
                Box::pin(async move {
                    let resp = tower::Service::call(&mut router, req).await;
                    Ok(resp.into_response())
                })
            }
        } else {
            // Everything else (platform, Cloud Run URL) → platform
            let mut router = self.platform.clone();
            Box::pin(async move {
                let resp = tower::Service::call(&mut router, req).await;
                Ok(resp.into_response())
            })
        }
    }
}

//  Protected page handlers
//
// Each handler checks the session cookie. If valid, serves the HTML file.
// If not authenticated, redirects to the login page.
//
// These are intentionally separate functions (not a macro) for clarity
// and to make it easy to add per-page logic later (e.g. user data injection).

/// GET /payment-success  Payment success page (protected).
async fn page_payment_success(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "payment-success.html").await
}

/// GET /statistics-template.html  Design-system statistics template (protected).
async fn page_statistics_template(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "_archive/statistics-template.html").await
}

/// GET /forms-template.html  Design-system forms template (protected).
async fn page_forms_template(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "_archive/forms-template.html").await
}

/// GET /table-template.html  Design-system table template (protected).
async fn page_table_template(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "_archive/table-template.html").await
}

/// GET /overlays-template.html  Design-system overlays template (protected).
async fn page_overlays_template(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "_archive/overlays-template.html").await
}

/// GET /fonts-template.html  Design-system fonts template (protected).
async fn page_fonts_template(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "_archive/fonts-template.html").await
}

/// GET /community/partials/:tab — Serves HTMX partial views for the community tabs.
async fn community_htmx_partial(
    Path(tab): Path<String>,
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let template_name = match tab.as_str() {
        "feed" => "partials/community_feed.html",
        "announcements" => "partials/community_announcements.html",
        "circle" => "partials/community_circle.html",
        "ama" => "partials/community_ama.html",
        "challenges" => "partials/community_challenges.html",
        _ => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                axum::response::Html("Tab not found".to_string()),
            )
                .into_response()
        }
    };
    common::routes_helper::serve_protected(jar, &state, template_name)
        .await
        .into_response()
}

/// GET /community/partials/feed/list — Serves the populated list of posts natively via MiniJinja
async fn community_feed_list_htmx(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<crate::community::routes::FeedQuery>,
) -> Result<axum::response::Response, crate::error::AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db).await;
    let posts = crate::community::routes::get_feed_data(&state, &query, user.as_ref()).await?;

    #[derive(serde::Serialize)]
    struct Context {
        posts: Vec<crate::community::models::PostDisplay>,
        current_feed_mode: String,
        base_url: String,
    }

    let current_feed_mode = query.feed_mode.clone().unwrap_or_else(|| "all".to_string());

    Ok(common::routes_helper::serve_protected_with_context(
        jar,
        &state,
        "partials/community_post_list.html",
        Context {
            posts,
            current_feed_mode,
            base_url: state.config.base_url.clone(),
        },
    )
    .await)
}

/// GET /community/partials/announcements/list — Serves the populated list of announcements natively via MiniJinja
async fn community_announcements_list_htmx(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<crate::community::routes::FeedQuery>,
) -> Result<axum::response::Response, crate::error::AppError> {
    let category = query.category.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    if let Some(category) = category.as_deref() {
        if !matches!(
            category,
            "new_commodity" | "dividend" | "platform_update" | "market_news" | "farm_update"
        ) {
            return Err(crate::error::AppError::BadRequest(
                "Invalid announcement category.".to_string(),
            ));
        }
    }

    let community_pool = state.community_db.as_ref().ok_or_else(|| {
        crate::error::AppError::ServiceUnavailable("Community database is offline".to_string())
    })?;
    let posts = crate::community::service::get_announcements(community_pool, category, 50).await?;

    #[derive(serde::Serialize)]
    struct Context {
        posts: Vec<crate::community::models::AnnouncementDisplay>,
    }

    Ok(common::routes_helper::serve_protected_with_context(
        jar,
        &state,
        "partials/community_announcements_list.html",
        Context { posts },
    )
    .await)
}

/// GET /community — Community demo page (protected).
async fn page_community(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "community.html").await
}

/// GET /community/post/:id — SSR Post Page (Public/Protected mixed)
async fn page_community_post(
    Path(id): Path<uuid::Uuid>,
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    #[derive(sqlx::FromRow)]
    struct PostOgData {
        content: String,
        image_urls: Option<Vec<String>>,
        user_id: uuid::Uuid,
    }

    #[derive(sqlx::FromRow)]
    struct AuthorOgData {
        display_name: Option<String>,
    }

    let current_user = auth::middleware::get_current_user(&jar, &state.db).await;
    let post_record = if let Some(c_pool) = state.community_db.as_ref() {
        sqlx::query_as::<_, PostOgData>(
            r#"
            SELECT p.content, p.image_urls, p.user_id
            FROM posts p
            JOIN community_profiles cp ON p.user_id = cp.user_id
            WHERE p.id = $1
              AND p.is_hidden = false
              AND (cp.is_shadowbanned = false OR p.user_id = $2)
            "#,
        )
        .bind(id)
        .bind(current_user.as_ref().map(|u| u.id))
        .fetch_optional(c_pool)
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    let mut context = serde_json::Map::new();
    let post_found = post_record.is_some();
    if let Some(p) = post_record {
        let content_snippet = if p.content.chars().count() > 150 {
            let snippet: String = p.content.chars().take(147).collect();
            format!("{}...", snippet)
        } else {
            p.content.clone()
        };

        let author = sqlx::query_as::<_, AuthorOgData>(
            r#"
            SELECT up.display_name
            FROM users u
            LEFT JOIN user_profiles up ON up.user_id = u.id
            WHERE u.id = $1
            "#,
        )
        .bind(p.user_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .and_then(|a| a.display_name)
        .unwrap_or_else(|| "Community Member".to_string());

        let og_title = format!("Post by {}", author);
        context.insert("og_title".to_string(), serde_json::Value::String(og_title));
        context.insert(
            "og_description".to_string(),
            serde_json::Value::String(content_snippet),
        );

        if let Some(imgs) = p.image_urls {
            if let Some(s) = imgs.first() {
                context.insert(
                    "og_image".to_string(),
                    serde_json::Value::String(crate::storage::service::rewrite_gcs_url(s)),
                );
            }
        }

        let og_url = format!("https://poool.finance/community/post/{}", id);
        context.insert("og_url".to_string(), serde_json::Value::String(og_url));
    }

    context.insert(
        "ssr_post_id".to_string(),
        serde_json::Value::String(id.to_string()),
    );
    context.insert(
        "ssr_post_found".to_string(),
        serde_json::Value::Bool(post_found),
    );

    // Serve via public with context, which will include user if logged in, but won't force login for crawlers
    let response =
        common::routes_helper::serve_public_with_context(jar, &state, "community.html", context)
            .await
            .into_response();

    if post_found {
        response
    } else {
        let (mut parts, body) = response.into_parts();
        parts.status = axum::http::StatusCode::NOT_FOUND;
        axum::response::Response::from_parts(parts, body)
    }
}

/// GET /marketplace-trading-v2 — V2 Marketplace trading page without charts (protected).
async fn page_marketplace_trading_v2(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let platform_fee_pct: f64 = sqlx::query_scalar(
        "SELECT value FROM platform_settings WHERE key = 'platform_fee_percent'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .and_then(|v: String| v.parse::<f64>().ok())
    .unwrap_or(5.0);

    let fee_pct_display = if platform_fee_pct == platform_fee_pct.floor() {
        format!("{:.0}", platform_fee_pct)
    } else {
        format!("{:.1}", platform_fee_pct)
    };

    let context = serde_json::json!({
        "fee_pct": platform_fee_pct,
        "fee_pct_display": fee_pct_display
    });

    common::routes_helper::serve_protected_with_context(
        jar,
        &state,
        "marketplace-trading-v2.html",
        context,
    )
    .await
}

/// GET /marketplace-secondary — Secondary market overview page (protected).
async fn page_marketplace_secondary(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let platform_fee_pct: f64 = sqlx::query_scalar(
        "SELECT value FROM platform_settings WHERE key = 'platform_fee_percent'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .and_then(|v: String| v.parse::<f64>().ok())
    .unwrap_or(5.0);

    let fee_pct_display = if platform_fee_pct == platform_fee_pct.floor() {
        format!("{:.0}", platform_fee_pct)
    } else {
        format!("{:.1}", platform_fee_pct)
    };

    let context = serde_json::json!({
        "fee_pct": platform_fee_pct,
        "fee_pct_display": fee_pct_display
    });

    common::routes_helper::serve_protected_with_context(
        jar,
        &state,
        "marketplace-secondary.html",
        context,
    )
    .await
}

/// GET /marketplace-trading-v3 — V3 Marketplace trading page with full property content (protected).
async fn page_marketplace_trading_v3(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let platform_fee_pct: f64 = sqlx::query_scalar(
        "SELECT value FROM platform_settings WHERE key = 'platform_fee_percent'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .and_then(|v: String| v.parse::<f64>().ok())
    .unwrap_or(5.0);

    let fee_pct_display = if platform_fee_pct == platform_fee_pct.floor() {
        format!("{:.0}", platform_fee_pct)
    } else {
        format!("{:.1}", platform_fee_pct)
    };

    let context = serde_json::json!({
        "fee_pct": platform_fee_pct,
        "fee_pct_display": fee_pct_display,
        "contact_asset_slug": params.get("asset").cloned().unwrap_or_default()
    });

    common::routes_helper::serve_protected_with_context(
        jar,
        &state,
        "marketplace-trading-v3.html",
        context,
    )
    .await
}

/// GET /my-trading — Investor's personal trading dashboard (orders, trades, buy interests, tax export).
async fn page_my_trading(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    let platform_fee_pct: f64 = sqlx::query_scalar(
        "SELECT value FROM platform_settings WHERE key = 'platform_fee_percent'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .and_then(|v: String| v.parse::<f64>().ok())
    .unwrap_or(5.0);

    let fee_pct_display = if platform_fee_pct == platform_fee_pct.floor() {
        format!("{:.0}", platform_fee_pct)
    } else {
        format!("{:.1}", platform_fee_pct)
    };

    let context = serde_json::json!({
        "fee_pct": platform_fee_pct,
        "fee_pct_display": fee_pct_display
    });

    common::routes_helper::serve_protected_with_context(jar, &state, "my-trading.html", context)
        .await
}

/// GET /trade-success — Confirmation page shown after a successful trade order placement.
async fn page_trade_success(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "trade-success.html").await
}

/// GET /payment-in-progress  Payment in progress page (protected).
async fn page_payment_in_progress(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "payment-in-progress.html").await
}

/// GET /rewards-v2 — Premium rewards page with "Digital Private Office" layout (protected).
async fn page_rewards_v2(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "rewards-v2.html").await
}

//  API Endpoints

/// GET /api/me  Return current user's profile data as JSON.
///
/// Used by the frontend `user-data.js` to replace hardcoded
/// placeholder names with real user data on all pages.
///
/// Returns 401 if not authenticated.
async fn api_me(jar: CookieJar, State(state): State<AppState>) -> axum::response::Response {
    let session_token = match jar.get(auth::middleware::SESSION_COOKIE) {
        Some(cookie) => cookie.value().to_string(),
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Not authenticated"
                })),
            )
                .into_response();
        }
    };

    match auth::service::get_user_profile(&state.db, &session_token).await {
        Ok(Some(profile)) => Json(profile).into_response(),
        Ok(None) => (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Session expired"
            })),
        )
            .into_response(),
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Internal server error"
            })),
        )
            .into_response(),
    }
}

// NOTE: KYC submission is now handled by the `kyc` module's own router.

// ─── Automated Migration Runner ──────────────────────────────────────────────

/// Run all pending SQL migrations from `../database/` in alphanumeric order.
///
/// Migrations are tracked in a `_schema_migrations` table. Each migration file
/// is run exactly once. If a migration fails, the error is logged and the server
/// continues (allowing partial migration scenarios to be debugged).
async fn run_migrations(pool: &sqlx::PgPool, dir: &str, label: &str) {
    // 1. Ensure tracking table exists
    if let Err(e) = sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS _schema_migrations (
            filename TEXT PRIMARY KEY,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
    )
    .execute(pool)
    .await
    {
        tracing::error!("Failed to create _schema_migrations table: {}", e);
        return;
    }

    // 2. Read migration files from the specified directory
    let migrations_dir = std::path::Path::new(dir);
    if !migrations_dir.exists() {
        tracing::warn!(
            "[{}] Migrations directory {:?} not found — skipping migrations",
            label,
            migrations_dir
        );
        return;
    }

    let mut files: Vec<String> = match std::fs::read_dir(migrations_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.ends_with(".sql") && !name.starts_with("seed") {
                    Some(name)
                } else {
                    None
                }
            })
            .collect(),
        Err(e) => {
            tracing::error!("Failed to read migrations directory: {}", e);
            return;
        }
    };

    files.sort();

    // 3. Fetch already-applied migrations
    let applied: Vec<String> =
        sqlx::query_scalar::<_, String>("SELECT filename FROM _schema_migrations")
            .fetch_all(pool)
            .await
            .unwrap_or_default();

    let applied_set: std::collections::HashSet<&str> = applied.iter().map(|s| s.as_str()).collect();

    // 4. Apply pending migrations
    let mut applied_count = 0;
    for filename in &files {
        if applied_set.contains(filename.as_str()) {
            continue;
        }

        let filepath = migrations_dir.join(filename);
        let sql = match std::fs::read_to_string(&filepath) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to read migration file {:?}: {}", filepath, e);
                continue;
            }
        };

        tracing::info!("[{}] Applying migration: {}", label, filename);

        // Run migration in a transaction
        let mut tx = match pool.begin().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    "Failed to begin transaction for migration {}: {}",
                    filename,
                    e
                );
                continue;
            }
        };

        // Split SQL into individual statements — sqlx cannot execute multiple
        // statements in a single prepared-statement call.
        // We must respect dollar-quoted blocks (e.g. DO $$ ... END $$;) which
        // contain semicolons that are NOT statement separators.
        let statements = split_sql_statements(&sql);
        let mut migration_failed = false;
        for stmt in &statements {
            if let Err(e) = sqlx::query(stmt).execute(&mut *tx).await {
                tracing::error!("Migration {} failed on statement: {}", filename, e);
                migration_failed = true;
                break;
            }
        }
        if migration_failed {
            let _ = tx.rollback().await;
            continue;
        }

        // Record as applied
        if let Err(e) = sqlx::query("INSERT INTO _schema_migrations (filename) VALUES ($1)")
            .bind(filename)
            .execute(&mut *tx)
            .await
        {
            tracing::error!("Failed to record migration {}: {}", filename, e);
            let _ = tx.rollback().await;
            continue;
        }

        if let Err(e) = tx.commit().await {
            tracing::error!("Failed to commit migration {}: {}", filename, e);
            continue;
        }

        applied_count += 1;
    }

    if applied_count > 0 {
        tracing::info!("[{}] Applied {} new migration(s)", label, applied_count);
    } else {
        tracing::info!(
            "[{}] Database schema is up to date ({} migrations tracked)",
            label,
            applied.len()
        );
    }
}

/// Split a SQL string into individual statements, respecting dollar-quoting,
/// single-quoted strings, and SQL comments. Only top-level semicolons are used
/// as statement separators.
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = sql.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_dollar_quote = false;
    let mut dollar_tag = String::new();

    while i < len {
        // Check for dollar-quoting: $$ or $tag$
        if chars[i] == '$' && !in_dollar_quote {
            // Try to read a dollar-quote tag: $<optional_identifier>$
            i += 1;
            let mut tag = String::from("$");
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                tag.push(chars[i]);
                i += 1;
            }
            if i < len && chars[i] == '$' {
                tag.push('$');
                i += 1;
                in_dollar_quote = true;
                dollar_tag = tag.clone();
                current.push_str(&dollar_tag);
                continue;
            } else {
                // Not a dollar-quote, push what we consumed
                current.push_str(&tag);
                continue;
            }
        }

        // Check for closing dollar-quote
        if in_dollar_quote && chars[i] == '$' {
            let mut tag = String::from("$");
            i += 1;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                tag.push(chars[i]);
                i += 1;
            }
            if i < len && chars[i] == '$' {
                tag.push('$');
                i += 1;
                if tag == dollar_tag {
                    in_dollar_quote = false;
                    current.push_str(&tag);
                    dollar_tag.clear();
                    continue;
                }
            }
            // Not the closing tag, push what we consumed
            current.push_str(&tag);
            continue;
        }

        // Inside a dollar-quoted block — everything is literal
        if in_dollar_quote {
            current.push(chars[i]);
            i += 1;
            continue;
        }

        // Single-line comment: --
        if chars[i] == '-' && i + 1 < len && chars[i + 1] == '-' {
            while i < len && chars[i] != '\n' {
                current.push(chars[i]);
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '*' {
            current.push('/');
            current.push('*');
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                current.push(chars[i]);
                i += 1;
            }
            if i + 1 < len {
                current.push('*');
                current.push('/');
                i += 2;
            }
            continue;
        }

        // Single-quoted string: '...' (with '' escape)
        if chars[i] == '\'' {
            current.push('\'');
            i += 1;
            while i < len {
                if chars[i] == '\'' {
                    current.push('\'');
                    i += 1;
                    if i < len && chars[i] == '\'' {
                        // escaped quote
                        current.push('\'');
                        i += 1;
                    } else {
                        break;
                    }
                } else {
                    current.push(chars[i]);
                    i += 1;
                }
            }
            continue;
        }

        // Top-level semicolon: statement boundary
        if chars[i] == ';' {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                statements.push(trimmed);
            }
            current.clear();
            i += 1;
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    // Leftover
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        statements.push(trimmed);
    }

    statements
}
