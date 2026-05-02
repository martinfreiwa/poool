//! Cross-replica leader election via Postgres advisory locks.
//!
//! Workers that must run as a singleton (matching engine, settlement
//! workers, indexers) wrap their main loop with [`run_as_leader`]. Each
//! replica calls in; only the one that wins the advisory lock actually
//! executes the worker body. The losers re-poll every `RETRY_SECS` and
//! take over if the leader's connection drops (DB session ends → lock
//! auto-released).
//!
//! 🔴 IMPORTANT: each worker MUST use a unique `lock_key` (see
//! [`LockKey`] for the canonical list). Re-using a key collapses two
//! independent workers into one.
//!
//! Why advisory locks and not a row-based lease:
//! - Auto-release on session end. No stuck leader if the replica OOMs.
//! - No table writes, no churn, no migrations.
//! - Zero perf cost while held.

use sqlx::PgPool;
use std::future::Future;
use std::time::Duration;

/// How long losers wait before retrying acquisition.
const RETRY_SECS: u64 = 30;

/// Stable, hand-picked i64 keys for each leader-elected worker.
///
/// Numbers are arbitrary but MUST be unique across the codebase. Pick from
/// this enum rather than passing raw numbers — that way the compiler tells
/// you when you're about to collide with an existing worker.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum LockKey {
    MarketplaceMatching = 1001,
    MarketplaceSettlement = 1002,
    MarketplaceOrderExpiry = 1003,
    MarketplaceRedisSync = 1004,
    MarketplacePriceSnapshot = 1005,
    BlockchainSettlement = 2001,
    BlockchainEventIndexer = 2002,
    BlockchainKycWhitelist = 2003,
    BlockchainReconciler = 2004,
    BlockchainGasMonitor = 2005,
    EmailScheduler = 3001,
    EmailOutbox = 3002,
    SlaMonitor = 3003,
    AffiliateHoldback = 3004,
    AffiliateTierProgression = 3005,
    PrimaryEscrowAutoRefund = 3006,
}

impl LockKey {
    fn as_i64(self) -> i64 {
        self as i64
    }

    fn name(self) -> &'static str {
        match self {
            LockKey::MarketplaceMatching => "marketplace_matching",
            LockKey::MarketplaceSettlement => "marketplace_settlement",
            LockKey::MarketplaceOrderExpiry => "marketplace_order_expiry",
            LockKey::MarketplaceRedisSync => "marketplace_redis_sync",
            LockKey::MarketplacePriceSnapshot => "marketplace_price_snapshot",
            LockKey::BlockchainSettlement => "blockchain_settlement",
            LockKey::BlockchainEventIndexer => "blockchain_event_indexer",
            LockKey::BlockchainKycWhitelist => "blockchain_kyc_whitelist",
            LockKey::BlockchainReconciler => "blockchain_reconciler",
            LockKey::BlockchainGasMonitor => "blockchain_gas_monitor",
            LockKey::EmailScheduler => "email_scheduler",
            LockKey::EmailOutbox => "email_outbox",
            LockKey::SlaMonitor => "sla_monitor",
            LockKey::AffiliateHoldback => "affiliate_holdback",
            LockKey::AffiliateTierProgression => "affiliate_tier_progression",
            LockKey::PrimaryEscrowAutoRefund => "primary_escrow_auto_refund",
        }
    }
}

/// Run `worker` as the unique leader for this `lock_key`. Polls until it
/// can acquire the advisory lock, runs the worker while holding it, and
/// returns only if the worker returns (lock auto-releases when the
/// dedicated connection drops).
///
/// The worker must use its own pool/handles for DB work; the lock-holding
/// connection stays parked just to hold the advisory lock.
pub async fn run_as_leader<F, Fut>(pool: PgPool, lock_key: LockKey, worker: F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = ()>,
{
    let key = lock_key.as_i64();
    let name = lock_key.name();

    loop {
        // Acquire a dedicated connection to hold the session-scope lock.
        // We must use the same connection for the lock and any subsequent
        // unlock check, so we own it explicitly here.
        let mut conn = match pool.acquire().await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    "🔒 [{}] couldn't acquire DB connection for leader lock: {} — retrying in {}s",
                    name,
                    e,
                    RETRY_SECS
                );
                tokio::time::sleep(Duration::from_secs(RETRY_SECS)).await;
                continue;
            }
        };

        let acquired: bool = match sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock($1)")
            .bind(key)
            .fetch_one(&mut *conn)
            .await
        {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(
                    "🔒 [{}] pg_try_advisory_lock failed: {} — retrying in {}s",
                    name,
                    e,
                    RETRY_SECS
                );
                drop(conn);
                tokio::time::sleep(Duration::from_secs(RETRY_SECS)).await;
                continue;
            }
        };

        if !acquired {
            tracing::debug!(
                "🔒 [{}] another replica is leader — sleeping {}s",
                name,
                RETRY_SECS
            );
            drop(conn);
            tokio::time::sleep(Duration::from_secs(RETRY_SECS)).await;
            continue;
        }

        tracing::info!("🔒 [{}] acquired leader lock — starting worker", name);

        // Run the worker. `conn` stays in scope until this block exits,
        // which keeps the session (and thus the advisory lock) alive for
        // the worker's lifetime. Workers must not borrow `conn` for their
        // own queries, because that would block the lock-holding session.
        worker().await;

        // Worker returned. Explicitly release before dropping (defensive;
        // session close would auto-release anyway).
        let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(key)
            .execute(&mut *conn)
            .await;
        drop(conn);

        tracing::warn!(
            "🔒 [{}] worker returned — leader lock released, will re-acquire",
            name
        );
    }
}
