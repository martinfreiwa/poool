pub mod event_indexer;
pub mod gas_monitor;
pub mod kyc_whitelist;
pub mod reconciler;
/// Blockchain integration module — on-chain settlement via POOOLProperty1155 contract.
///
/// This module is the bridge between the off-chain trading engine and the
/// on-chain ERC-1155 settlement layer on Polygon.
///
/// Architecture:
/// - Trades are matched and settled in PostgreSQL first (instant, off-chain)
/// - A background worker batches confirmed trades and calls `settleBatch()`
/// - The contract enforces KYC whitelist + transfer restrictions on-chain
/// - An event indexer caches on-chain balances for fast reads
/// - A KYC sync worker auto-whitelists approved users
/// - Trade records are updated with tx hash and confirmation status
///
/// Workers (spawned in main.rs):
/// - `service::run_settlement_worker` — Batch settlement (every 5 min)
/// - `event_indexer::run_event_indexer` — Balance sync (every 5 sec)
/// - `kyc_whitelist::run_kyc_whitelist_worker` — KYC→Whitelist (every 60 sec)
/// - `reconciler::run_reconciler` — Recovers trades stuck in 'submitted' (every 2 min)
///
/// 🔴 FINANCIAL CODE — All monetary logic uses integer math (cents/wei).
///    No floating-point math anywhere in this module.
pub mod service;
/// In-process Ethereum signing + EIP-191 verification (replaces `cast` subprocess).
pub mod signing;
/// Pluggable signer abstraction — local raw key vs HSM-backed GCP KMS.
pub mod signer;
