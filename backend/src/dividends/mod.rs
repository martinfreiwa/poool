/// Dividend system — calculation, anti-sniping, and payout execution.
///
/// This module handles the full dividend lifecycle:
/// 1. Admin triggers calculation for an asset + period
/// 2. System snapshots on-chain balances and calculates per-user payouts
/// 3. Anti-sniping logic filters out short-term holders (< N days)
/// 4. Admin reviews + approves the distribution
/// 5. System credits wallets in a single ACID transaction
///
/// 🔴 FINANCIAL CODE — All monetary values are BIGINT cents. No floats.
/// 🔴 All payout execution happens inside a DB transaction.
/// 🔴 Balance should never go negative — CHECK constraints enforce this.
pub mod service;
