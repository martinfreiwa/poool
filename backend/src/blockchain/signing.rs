//! In-process Ethereum signing + verification.
//!
//! Replaces the `cast` CLI subprocess (which leaked the private key via
//! `ps aux` and depended on Foundry being installed). Two responsibilities:
//!
//! 1. **Settlement signer** — derive the platform signer's address from
//!    its private key and produce signed legacy transactions for the
//!    settlement worker.
//! 2. **SIWE verifier** — verify EIP-191 personal_sign messages so users
//!    can prove ownership of an Ethereum address during KYC, binding a
//!    real wallet to their account.
//!
//! 🔴 SECURITY: the settlement private key never leaves this process
//! address space. It is read once from `CHAIN_SETTLEMENT_PRIVATE_KEY` and
//! held in memory. No subprocess args, no logs, no error messages.

use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use sha3::{Digest, Keccak256};

/// Compute keccak256(input) → 32 bytes.
pub fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(input);
    let out = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}

/// Derive an Ethereum address (last 20 bytes of keccak256(pubkey)) from a
/// secp256k1 public key.
fn pubkey_to_address(verifying_key: &VerifyingKey) -> [u8; 20] {
    // Uncompressed pubkey is 65 bytes: 0x04 || X || Y. Drop the prefix
    // byte and hash X || Y. The Ethereum address is the last 20 bytes
    // of that hash.
    let encoded = verifying_key.to_encoded_point(false);
    let pubkey_bytes = encoded.as_bytes();
    debug_assert_eq!(pubkey_bytes.len(), 65);
    let hash = keccak256(&pubkey_bytes[1..]);
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);
    addr
}

/// Parse a 0x-prefixed (or unprefixed) hex string into bytes.
fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    let trimmed = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(trimmed).map_err(|e| format!("invalid hex: {}", e))
}

/// Public re-export of `parse_hex` for sibling modules (signer.rs).
pub fn parse_hex_pub(s: &str) -> Result<Vec<u8>, String> {
    parse_hex(s)
}

/// Format 20 bytes as a 0x-prefixed lowercase Ethereum address.
pub fn format_address(addr: &[u8; 20]) -> String {
    format!("0x{}", hex::encode(addr))
}

/// Derive the Ethereum address controlled by a private key.
///
/// Replaces the placeholder `derive_address_from_private_key` in
/// `service.rs` — that one was a stub that returned an env var.
pub fn address_from_private_key(private_key_hex: &str) -> Result<String, String> {
    let key_bytes = parse_hex(private_key_hex)?;
    if key_bytes.len() != 32 {
        return Err(format!(
            "private key must be 32 bytes, got {}",
            key_bytes.len()
        ));
    }
    let signing_key = SigningKey::from_slice(&key_bytes)
        .map_err(|e| format!("invalid secp256k1 private key: {}", e))?;
    let verifying_key = signing_key.verifying_key();
    let addr = pubkey_to_address(verifying_key);
    Ok(format_address(&addr))
}

// ═══════════════════════════════════════════════════════════════
// ── EIP-191 SIWE VERIFICATION ────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Wrap a message as defined by EIP-191 / personal_sign:
///   "\x19Ethereum Signed Message:\n" || len(message) || message
fn eip191_hash(message: &[u8]) -> [u8; 32] {
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let mut buf = Vec::with_capacity(prefix.len() + message.len());
    buf.extend_from_slice(prefix.as_bytes());
    buf.extend_from_slice(message);
    keccak256(&buf)
}

/// Verify that `signature_hex` (65 bytes: r || s || v) was produced by
/// the holder of `claimed_address` over `message` (using personal_sign).
///
/// On success returns `Ok(())`. On any failure (bad hex, bad signature,
/// wrong recovered address) returns `Err(...)`.
///
/// This is the gate the SIWE wallet-binding endpoint uses to confirm a
/// user actually controls the address they're claiming.
pub fn verify_personal_sign(
    message: &[u8],
    signature_hex: &str,
    claimed_address: &str,
) -> Result<(), String> {
    let sig_bytes = parse_hex(signature_hex)?;
    if sig_bytes.len() != 65 {
        return Err(format!(
            "signature must be 65 bytes, got {}",
            sig_bytes.len()
        ));
    }
    // r (32) || s (32) || v (1). Ethereum encodes v as 27/28 (or 0/1
    // post-EIP-155 + chain shifts). For personal_sign it's 27/28.
    let v_raw = sig_bytes[64];
    let recovery_id_byte = match v_raw {
        27 | 28 => v_raw - 27,
        0 | 1 => v_raw,
        _ => return Err(format!("invalid recovery byte v={}", v_raw)),
    };

    let signature = Signature::from_slice(&sig_bytes[..64])
        .map_err(|e| format!("invalid signature bytes: {}", e))?;
    let recovery_id = RecoveryId::from_byte(recovery_id_byte)
        .ok_or_else(|| format!("invalid recovery id {}", recovery_id_byte))?;

    let digest = eip191_hash(message);
    let recovered = VerifyingKey::recover_from_prehash(&digest, &signature, recovery_id)
        .map_err(|e| format!("signature recovery failed: {}", e))?;
    let recovered_addr = pubkey_to_address(&recovered);

    let claimed_clean = claimed_address
        .strip_prefix("0x")
        .unwrap_or(claimed_address)
        .to_lowercase();
    let recovered_hex = hex::encode(recovered_addr);
    if claimed_clean != recovered_hex {
        return Err(format!(
            "signature does not match claimed address (recovered=0x{})",
            recovered_hex
        ));
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── LEGACY TRANSACTION SIGNING (EIP-155) ─────────────────────
// ═══════════════════════════════════════════════════════════════

/// Sign a legacy (pre-EIP-1559) transaction with EIP-155 chain-ID
/// replay protection. Returns the raw RLP-encoded signed transaction
/// bytes as `0x...` hex, ready for `eth_sendRawTransaction`.
///
/// We use legacy txs (not EIP-1559) because Polygon RPCs accept both and
/// legacy is one fewer moving piece (no priority fee). If/when we move
/// to EIP-1559 we'd add a parallel function.
#[allow(clippy::too_many_arguments)]
pub fn sign_legacy_transaction(
    private_key_hex: &str,
    chain_id: u64,
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
    to: &str,
    value_wei: u128,
    data: &str,
) -> Result<String, String> {
    let key_bytes = parse_hex(private_key_hex)?;
    if key_bytes.len() != 32 {
        return Err("private key must be 32 bytes".to_string());
    }
    let signing_key =
        SigningKey::from_slice(&key_bytes).map_err(|e| format!("invalid private key: {}", e))?;

    let to_bytes = parse_hex(to)?;
    if to_bytes.len() != 20 {
        return Err(format!(
            "to-address must be 20 bytes, got {}",
            to_bytes.len()
        ));
    }
    let data_bytes = parse_hex(data)?;

    // Step 1: RLP-encode the unsigned tx with EIP-155 (chain_id, 0, 0)
    // appended in place of (v, r, s). Hash with keccak256.
    let unsigned_rlp = encode_legacy_tx_rlp(
        nonce,
        gas_price,
        gas_limit,
        &to_bytes,
        value_wei,
        &data_bytes,
        Some(chain_id),
        None,
    );
    let digest = keccak256(&unsigned_rlp);

    // Step 2: secp256k1 sign with recovery.
    let (signature, recovery_id) = signing_key
        .sign_prehash_recoverable(&digest)
        .map_err(|e| format!("signing failed: {}", e))?;

    // Step 3: Build EIP-155 v: v = chain_id*2 + 35 + recid.
    let v = chain_id
        .checked_mul(2)
        .and_then(|x| x.checked_add(35))
        .and_then(|x| x.checked_add(recovery_id.to_byte() as u64))
        .ok_or_else(|| "v overflow".to_string())?;

    let r_bytes = signature.r().to_bytes();
    let s_bytes = signature.s().to_bytes();

    // Step 4: RLP-encode the signed tx (same fields + actual v, r, s).
    let signed_rlp = encode_legacy_tx_rlp(
        nonce,
        gas_price,
        gas_limit,
        &to_bytes,
        value_wei,
        &data_bytes,
        None,
        Some((v, r_bytes.as_slice(), s_bytes.as_slice())),
    );

    Ok(format!("0x{}", hex::encode(signed_rlp)))
}

/// Sign a legacy (EIP-155) transaction using any [`Signer`]
/// (local key OR KMS). Same wire output as `sign_legacy_transaction` —
/// callers should prefer this variant going forward.
#[allow(clippy::too_many_arguments)]
pub async fn sign_legacy_transaction_with(
    signer: &dyn super::signer::Signer,
    chain_id: u64,
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
    to: &str,
    value_wei: u128,
    data: &str,
) -> Result<String, String> {
    let to_bytes = parse_hex(to)?;
    if to_bytes.len() != 20 {
        return Err(format!(
            "to-address must be 20 bytes, got {}",
            to_bytes.len()
        ));
    }
    let data_bytes = parse_hex(data)?;

    let unsigned_rlp = encode_legacy_tx_rlp(
        nonce,
        gas_price,
        gas_limit,
        &to_bytes,
        value_wei,
        &data_bytes,
        Some(chain_id),
        None,
    );
    let digest = keccak256(&unsigned_rlp);

    let signed = signer.sign_prehash(&digest).await?;

    let v = chain_id
        .checked_mul(2)
        .and_then(|x| x.checked_add(35))
        .and_then(|x| x.checked_add(signed.recovery_id.to_byte() as u64))
        .ok_or_else(|| "v overflow".to_string())?;
    let r_bytes = signed.signature.r().to_bytes();
    let s_bytes = signed.signature.s().to_bytes();

    let signed_rlp = encode_legacy_tx_rlp(
        nonce,
        gas_price,
        gas_limit,
        &to_bytes,
        value_wei,
        &data_bytes,
        None,
        Some((v, r_bytes.as_slice(), s_bytes.as_slice())),
    );
    Ok(format!("0x{}", hex::encode(signed_rlp)))
}

/// RLP-encode a legacy transaction. Two modes:
/// - Unsigned (EIP-155 form): pass `chain_id`, leave `signature` None.
/// - Signed: leave `chain_id` None, pass `(v, r, s)`.
fn encode_legacy_tx_rlp(
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
    to: &[u8],
    value: u128,
    data: &[u8],
    chain_id: Option<u64>,
    signature: Option<(u64, &[u8], &[u8])>,
) -> Vec<u8> {
    let mut stream = rlp::RlpStream::new_list(9);
    stream.append(&nonce);
    stream.append(&gas_price);
    stream.append(&gas_limit);
    stream.append(&to);
    // u128 value: serialize as big-endian, leading zeros stripped, so
    // small values encode compactly.
    stream.append(&value_to_bytes(value).as_slice());
    stream.append(&data);
    match (chain_id, signature) {
        (Some(cid), None) => {
            stream.append(&cid);
            stream.append(&0u8);
            stream.append(&0u8);
        }
        (None, Some((v, r, s))) => {
            stream.append(&v);
            stream.append(&trim_leading_zeros(r));
            stream.append(&trim_leading_zeros(s));
        }
        _ => unreachable!("encode_legacy_tx_rlp: one of chain_id/signature must be set"),
    }
    stream.out().to_vec()
}

fn value_to_bytes(value: u128) -> Vec<u8> {
    if value == 0 {
        return Vec::new();
    }
    let raw = value.to_be_bytes();
    let first_nonzero = raw.iter().position(|&b| b != 0).unwrap_or(15);
    raw[first_nonzero..].to_vec()
}

fn trim_leading_zeros(bytes: &[u8]) -> &[u8] {
    let i = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    &bytes[i..]
}

// ═══════════════════════════════════════════════════════════════
// ── TESTS ─────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// Well-known test vector from the Ethereum yellow paper / common
    /// fixtures: this private key derives this address.
    const TEST_KEY: &str = "0x4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318";
    const TEST_ADDR: &str = "0x2c7536e3605d9c16a7a3d7b1898e529396a65c23";

    #[test]
    fn address_derivation_matches_known_vector() {
        let addr = address_from_private_key(TEST_KEY).unwrap();
        assert_eq!(addr, TEST_ADDR);
    }

    #[test]
    fn address_derivation_rejects_short_key() {
        assert!(address_from_private_key("0x1234").is_err());
    }

    #[test]
    fn personal_sign_round_trip() {
        // Sign a message with our key, then verify recovers our address.
        let key_bytes = parse_hex(TEST_KEY).unwrap();
        let signing_key = SigningKey::from_slice(&key_bytes).unwrap();
        let message = b"Bind wallet to account abc123";
        let digest = eip191_hash(message);
        let (sig, recid) = signing_key.sign_prehash_recoverable(&digest).unwrap();
        let mut sig_bytes = sig.to_bytes().to_vec();
        sig_bytes.push(recid.to_byte() + 27);
        let sig_hex = format!("0x{}", hex::encode(&sig_bytes));
        verify_personal_sign(message, &sig_hex, TEST_ADDR).unwrap();
    }

    #[test]
    fn personal_sign_rejects_wrong_address() {
        let key_bytes = parse_hex(TEST_KEY).unwrap();
        let signing_key = SigningKey::from_slice(&key_bytes).unwrap();
        let message = b"hi";
        let digest = eip191_hash(message);
        let (sig, recid) = signing_key.sign_prehash_recoverable(&digest).unwrap();
        let mut sig_bytes = sig.to_bytes().to_vec();
        sig_bytes.push(recid.to_byte() + 27);
        let sig_hex = format!("0x{}", hex::encode(&sig_bytes));
        // Wrong address — should fail.
        assert!(verify_personal_sign(
            message,
            &sig_hex,
            "0x0000000000000000000000000000000000000000"
        )
        .is_err());
    }

    #[test]
    fn legacy_tx_signs_without_panic() {
        let signed = sign_legacy_transaction(
            TEST_KEY,
            137, // Polygon mainnet
            42,
            30_000_000_000,
            21_000,
            "0x5555555555555555555555555555555555555555",
            1_000_000_000_000_000, // 0.001 ETH
            "0x",
        )
        .unwrap();
        assert!(signed.starts_with("0x"));
        assert!(signed.len() > 100); // Reasonable lower bound
    }
}
