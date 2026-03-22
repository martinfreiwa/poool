/// IPFS integration module — metadata storage via Pinata for ERC-1155 asset tokens.
///
/// This module handles:
/// 1. Building ERC-1155 compliant metadata JSON for each tokenized asset
/// 2. Pinning metadata (and optionally legal documents) to IPFS via Pinata
/// 3. Serving metadata at `/api/assets/{id}/metadata.json` for smart contracts
///
/// The metadata follows the ERC-1155 Metadata JSON standard and includes
/// SPV details, property information, offering terms, and document hashes.
///
/// 🔴 SECURITY: Never include PII (names, emails, KYC docs) in on-chain metadata.
pub mod metadata;
pub mod service;
