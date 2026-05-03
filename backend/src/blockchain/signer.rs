//! Pluggable Ethereum signer abstraction.
//!
//! Two implementations:
//!
//! - [`LocalKeySigner`] — wraps a raw secp256k1 private key held in process
//!   memory. Used for local dev and as a fallback when KMS is not configured.
//!
//! - [`KmsSigner`] — delegates the raw ECDSA operation to a Google Cloud KMS
//!   asymmetric-signing key (HSM-protected, key material never extractable).
//!   Selected when `CHAIN_KMS_KEY` env var is set to a key version resource
//!   name, e.g.
//!   `projects/<p>/locations/europe-west1/keyRings/poool-blockchain/cryptoKeys/chain-settler/cryptoKeyVersions/1`.
//!
//! Both produce `(Signature, RecoveryId)` from a 32-byte prehash. The legacy
//! transaction encoder in [`super::signing::sign_legacy_transaction_with`]
//! handles RLP + EIP-155 v computation on top.

use async_trait::async_trait;
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};

use super::signing::{keccak256, parse_hex_pub};

/// Outcome of a sign request: low-s normalised signature + the recovery id
/// (0 or 1) that recovers to the signer's address.
#[derive(Debug, Clone)]
pub struct SignedDigest {
    pub signature: Signature,
    pub recovery_id: RecoveryId,
}

#[async_trait]
pub trait Signer: Send + Sync + std::fmt::Debug {
    /// Ethereum address (20 bytes) controlled by this signer.
    fn address(&self) -> [u8; 20];

    /// Produce an ECDSA signature over a 32-byte prehash. Implementations
    /// MUST return a low-s signature with the matching recovery id so that
    /// callers can encode (v, r, s) directly into Ethereum tx form.
    async fn sign_prehash(&self, hash: &[u8; 32]) -> Result<SignedDigest, String>;
}

// ───────────────────────── LocalKeySigner ─────────────────────────

pub struct LocalKeySigner {
    key: SigningKey,
    address: [u8; 20],
}

impl std::fmt::Debug for LocalKeySigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalKeySigner")
            .field("address", &hex::encode(self.address))
            .finish_non_exhaustive()
    }
}

impl LocalKeySigner {
    pub fn from_hex(private_key_hex: &str) -> Result<Self, String> {
        let bytes = parse_hex_pub(private_key_hex)?;
        if bytes.len() != 32 {
            return Err(format!("private key must be 32 bytes, got {}", bytes.len()));
        }
        let key = SigningKey::from_slice(&bytes)
            .map_err(|e| format!("invalid secp256k1 private key: {}", e))?;
        let address = pubkey_to_address(key.verifying_key());
        Ok(Self { key, address })
    }
}

#[async_trait]
impl Signer for LocalKeySigner {
    fn address(&self) -> [u8; 20] {
        self.address
    }

    async fn sign_prehash(&self, hash: &[u8; 32]) -> Result<SignedDigest, String> {
        let (signature, recovery_id) = self
            .key
            .sign_prehash_recoverable(hash)
            .map_err(|e| format!("local sign failed: {}", e))?;
        // k256's `sign_prehash_recoverable` already returns a low-s
        // signature, so no normalisation needed here.
        Ok(SignedDigest {
            signature,
            recovery_id,
        })
    }
}

// ─────────────────────────── KmsSigner ───────────────────────────

/// HSM-backed signer. Holds the resolved Ethereum address (derived once at
/// init from the KMS public key) plus the KMS key resource name used for
/// every subsequent sign request.
pub struct KmsSigner {
    key_name: String,
    address: [u8; 20],
    client: google_cloud_kms::client::Client,
}

impl std::fmt::Debug for KmsSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KmsSigner")
            .field("key_name", &self.key_name)
            .field("address", &hex::encode(self.address))
            .finish_non_exhaustive()
    }
}

impl KmsSigner {
    /// Build a KMS signer from a key version resource name. Calls
    /// `GetPublicKey` once to derive the Ethereum address.
    pub async fn new(key_name: String) -> Result<Self, String> {
        use google_cloud_kms::client::{Client, ClientConfig};
        use google_cloud_kms::grpc::kms::v1::GetPublicKeyRequest;

        let config = ClientConfig::default()
            .with_auth()
            .await
            .map_err(|e| format!("KMS auth failed: {}", e))?;
        let client = Client::new(config)
            .await
            .map_err(|e| format!("KMS client init failed: {}", e))?;

        let pubkey = client
            .get_public_key(
                GetPublicKeyRequest {
                    name: key_name.clone(),
                },
                None,
            )
            .await
            .map_err(|e| format!("KMS GetPublicKey failed: {}", e))?;

        let address = pem_to_eth_address(&pubkey.pem)?;
        Ok(Self {
            key_name,
            address,
            client,
        })
    }
}

#[async_trait]
impl Signer for KmsSigner {
    fn address(&self) -> [u8; 20] {
        self.address
    }

    async fn sign_prehash(&self, hash: &[u8; 32]) -> Result<SignedDigest, String> {
        use google_cloud_kms::grpc::kms::v1::{digest, AsymmetricSignRequest, Digest};

        let req = AsymmetricSignRequest {
            name: self.key_name.clone(),
            // KMS verifies digest length matches the algorithm's hash (32B
            // for secp256k1+SHA256). We pass keccak256(payload) — KMS does
            // not recompute or verify the hash function used.
            digest: Some(Digest {
                digest: Some(digest::Digest::Sha256(hash.to_vec())),
            }),
            ..Default::default()
        };

        let resp = self
            .client
            .asymmetric_sign(req, None)
            .await
            .map_err(|e| format!("KMS AsymmetricSign failed: {}", e))?;

        let (r, s) = parse_der_signature(&resp.signature)?;
        let (signature, recovery_id) = recover_signature(hash, &r, &s, &self.address)?;
        Ok(SignedDigest {
            signature,
            recovery_id,
        })
    }
}

// ───────────────────────── helpers ─────────────────────────

fn pubkey_to_address(verifying_key: &VerifyingKey) -> [u8; 20] {
    let encoded = verifying_key.to_encoded_point(false);
    let bytes = encoded.as_bytes();
    debug_assert_eq!(bytes.len(), 65);
    let hash = keccak256(&bytes[1..]);
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);
    addr
}

/// Derive an Ethereum address from a SubjectPublicKeyInfo PEM. KMS returns
/// the public key in this format; the trailing 65 bytes of the DER-decoded
/// SPKI are the uncompressed secp256k1 point (`0x04 || X || Y`).
fn pem_to_eth_address(pem: &str) -> Result<[u8; 20], String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let b64: String = pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");
    let der = STANDARD
        .decode(b64.trim())
        .map_err(|e| format!("invalid PEM base64: {}", e))?;
    if der.len() < 65 {
        return Err(format!("SPKI DER too short: {} bytes", der.len()));
    }
    let pub_uncompressed = &der[der.len() - 65..];
    if pub_uncompressed[0] != 0x04 {
        return Err(format!(
            "expected uncompressed point prefix 0x04, got {:#x}",
            pub_uncompressed[0]
        ));
    }
    let xy = &pub_uncompressed[1..];
    let hash = keccak256(xy);
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);
    Ok(addr)
}

/// Parse a DER-encoded ECDSA signature into (r, s) as 32-byte big-endian
/// scalars. Handles the leading-0x00 padding byte that DER adds when the
/// high bit of an integer is set.
fn parse_der_signature(der: &[u8]) -> Result<([u8; 32], [u8; 32]), String> {
    // SEQUENCE { INTEGER r, INTEGER s }
    if der.len() < 8 || der[0] != 0x30 {
        return Err("DER: missing SEQUENCE tag".to_string());
    }
    let seq_len = der[1] as usize;
    if seq_len + 2 != der.len() {
        return Err(format!(
            "DER: SEQUENCE length mismatch ({} + 2 != {})",
            seq_len,
            der.len()
        ));
    }
    let body = &der[2..];

    let read_int = |buf: &[u8]| -> Result<(Vec<u8>, usize), String> {
        if buf.len() < 2 || buf[0] != 0x02 {
            return Err("DER: missing INTEGER tag".to_string());
        }
        let len = buf[1] as usize;
        if buf.len() < 2 + len {
            return Err("DER: INTEGER overrun".to_string());
        }
        let mut int = buf[2..2 + len].to_vec();
        // Strip leading 0x00 used to keep integers positive.
        if !int.is_empty() && int[0] == 0x00 {
            int.remove(0);
        }
        Ok((int, 2 + len))
    };

    let (r_bytes, r_consumed) = read_int(body)?;
    let (s_bytes, _) = read_int(&body[r_consumed..])?;

    let to_32 = |v: Vec<u8>| -> Result<[u8; 32], String> {
        if v.len() > 32 {
            return Err(format!("scalar too large: {} bytes", v.len()));
        }
        let mut out = [0u8; 32];
        out[32 - v.len()..].copy_from_slice(&v);
        Ok(out)
    };
    Ok((to_32(r_bytes)?, to_32(s_bytes)?))
}

/// Given (r, s), figure out which recovery id (0 or 1) yields the expected
/// Ethereum address. Also normalises s to low-s form (Ethereum requirement).
fn recover_signature(
    hash: &[u8; 32],
    r: &[u8; 32],
    s: &[u8; 32],
    expected_address: &[u8; 20],
) -> Result<(Signature, RecoveryId), String> {
    use k256::elliptic_curve::scalar::IsHigh;
    use k256::Scalar;

    // Build initial signature.
    let mut sig =
        Signature::from_scalars(*r, *s).map_err(|e| format!("invalid (r,s) scalars: {}", e))?;

    // Ethereum requires low-s. If KMS returned high-s, normalise:
    // s' = N - s, and flip the recovery id parity.
    let s_scalar = Scalar::from(*sig.s().as_ref());
    let mut flipped = false;
    if bool::from(s_scalar.is_high()) {
        sig = sig.normalize_s().unwrap_or(sig);
        flipped = true;
    }

    for candidate in [0u8, 1u8] {
        let recid_byte = if flipped { candidate ^ 1 } else { candidate };
        let recid = match RecoveryId::from_byte(recid_byte) {
            Some(r) => r,
            None => continue,
        };
        let recovered = match VerifyingKey::recover_from_prehash(hash, &sig, recid) {
            Ok(k) => k,
            Err(_) => continue,
        };
        if &pubkey_to_address(&recovered) == expected_address {
            return Ok((sig, recid));
        }
    }
    Err("signature does not recover to expected address".to_string())
}

// ───────────────────────── factory ─────────────────────────

/// Pick the right signer based on env. KMS takes precedence when both are
/// set (production path). Returns `None` when neither is configured (matches
/// the prior behaviour where the chain worker just stays idle).
pub async fn build_signer_from_env() -> Option<Result<Box<dyn Signer>, String>> {
    if let Ok(key_name) = std::env::var("CHAIN_KMS_KEY") {
        if !key_name.is_empty() {
            return Some(
                KmsSigner::new(key_name)
                    .await
                    .map(|s| Box::new(s) as Box<dyn Signer>),
            );
        }
    }
    if let Ok(pk) = std::env::var("CHAIN_SETTLEMENT_PRIVATE_KEY") {
        if !pk.is_empty() {
            return Some(
                LocalKeySigner::from_hex(&pk).map(|s| Box::new(s) as Box<dyn Signer>),
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: &str = "0x4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318";
    const TEST_ADDR: &str = "2c7536e3605d9c16a7a3d7b1898e529396a65c23";

    #[tokio::test]
    async fn local_signer_address_matches_known_vector() {
        let signer = LocalKeySigner::from_hex(TEST_KEY).unwrap();
        assert_eq!(hex::encode(signer.address()), TEST_ADDR);
    }

    /// Live test against a real GCP KMS key. Requires:
    ///   CHAIN_KMS_KEY=projects/.../cryptoKeyVersions/N
    ///   GOOGLE_APPLICATION_CREDENTIALS or `gcloud auth application-default login`
    /// Run with: `cargo test --bin poool-backend -- --ignored kms_signer`
    #[tokio::test]
    #[ignore = "requires CHAIN_KMS_KEY env + GCP auth"]
    async fn kms_signer_round_trip_against_real_key() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let key = std::env::var("CHAIN_KMS_KEY").expect("CHAIN_KMS_KEY not set");
        let signer = KmsSigner::new(key).await.expect("KmsSigner::new failed");
        let hash = keccak256(b"poool kms round trip test");
        let signed = signer.sign_prehash(&hash).await.expect("sign failed");
        let recovered = VerifyingKey::recover_from_prehash(
            &hash,
            &signed.signature,
            signed.recovery_id,
        )
        .expect("recover failed");
        assert_eq!(
            pubkey_to_address(&recovered),
            signer.address(),
            "recovered address must match KMS-derived address"
        );
    }

    /// Full pipeline: KMS sign a legacy tx, RLP-encode, then parse the
    /// signed RLP back and recover the sender. Proves the sign + encode
    /// path the settlement worker uses works end-to-end against a real
    /// HSM key without spending gas on chain.
    #[tokio::test]
    #[ignore = "requires CHAIN_KMS_KEY env + GCP auth"]
    async fn kms_signed_legacy_tx_recovers_to_kms_address() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let key = std::env::var("CHAIN_KMS_KEY").expect("CHAIN_KMS_KEY not set");
        let signer = KmsSigner::new(key).await.expect("KmsSigner::new failed");

        let signed_hex = super::super::signing::sign_legacy_transaction_with(
            &signer,
            80002,                    // Amoy chain id
            0,                        // nonce
            30_000_000_000,           // 30 gwei
            21_000,                   // simple transfer gas
            &super::super::signing::format_address(&signer.address()), // self-tx
            0,
            "0x",
        )
        .await
        .expect("sign failed");

        // Parse RLP, extract chain-id-aware (v, r, s), recompute the
        // unsigned digest, recover address.
        let raw = hex::decode(signed_hex.trim_start_matches("0x")).unwrap();
        let rlp = rlp::Rlp::new(&raw);
        let nonce: u64 = rlp.val_at(0).unwrap();
        let gas_price: u64 = rlp.val_at(1).unwrap();
        let gas_limit: u64 = rlp.val_at(2).unwrap();
        let to: Vec<u8> = rlp.val_at(3).unwrap();
        let value: Vec<u8> = rlp.val_at(4).unwrap();
        let data: Vec<u8> = rlp.val_at(5).unwrap();
        let v: u64 = rlp.val_at(6).unwrap();
        let r: Vec<u8> = rlp.val_at(7).unwrap();
        let s: Vec<u8> = rlp.val_at(8).unwrap();

        // EIP-155: v = chain_id*2 + 35 + recid → recid = v - 35 - chain_id*2
        let recid_byte = (v - 35 - 80002 * 2) as u8;

        // Re-encode the unsigned form (with chain_id, 0, 0) to recover digest.
        let mut us = rlp::RlpStream::new_list(9);
        us.append(&nonce);
        us.append(&gas_price);
        us.append(&gas_limit);
        us.append(&to);
        us.append(&value);
        us.append(&data);
        us.append(&80002u64);
        us.append(&0u8);
        us.append(&0u8);
        let unsigned = us.out().to_vec();
        let digest = keccak256(&unsigned);

        let mut r32 = [0u8; 32];
        r32[32 - r.len()..].copy_from_slice(&r);
        let mut s32 = [0u8; 32];
        s32[32 - s.len()..].copy_from_slice(&s);
        let sig = k256::ecdsa::Signature::from_scalars(r32, s32).unwrap();
        let recid = RecoveryId::from_byte(recid_byte).unwrap();
        let recovered = VerifyingKey::recover_from_prehash(&digest, &sig, recid).unwrap();
        assert_eq!(pubkey_to_address(&recovered), signer.address());
    }

    /// Synthetic test of the high-s recovery path: take a known low-s
    /// signature, flip s to high-s (s' = N - s), feed it through
    /// `recover_signature`, and verify normalisation + recid flip recover
    /// the correct signer address. This exercises the branch that fires
    /// when KMS returns a high-s signature.
    #[tokio::test]
    async fn recover_signature_handles_high_s() {
        use k256::elliptic_curve::PrimeField;

        // Sign a message with a known key — gives us a low-s baseline.
        let key_bytes = parse_hex_pub(TEST_KEY).unwrap();
        let key = SigningKey::from_slice(&key_bytes).unwrap();
        let expected_addr = pubkey_to_address(key.verifying_key());
        let hash = keccak256(b"high-s test payload");
        let (sig_low, _recid_low) = key.sign_prehash_recoverable(&hash).unwrap();

        // Construct the high-s mirror: s' = N - s.
        let r_bytes: [u8; 32] = sig_low.r().to_bytes().into();
        let s_low_bytes: [u8; 32] = sig_low.s().to_bytes().into();
        let s_low_scalar = k256::Scalar::from_repr(s_low_bytes.into()).unwrap();
        let s_high_scalar = -s_low_scalar;
        let s_high_bytes: [u8; 32] = s_high_scalar.to_repr().into();

        // Sanity: the flipped s really is high.
        use k256::elliptic_curve::scalar::IsHigh;
        assert!(bool::from(s_high_scalar.is_high()), "test setup: s should be high");

        let (recovered_sig, recovered_recid) =
            recover_signature(&hash, &r_bytes, &s_high_bytes, &expected_addr)
                .expect("recover_signature must handle high-s input");

        // Verify the returned (sig, recid) actually recovers correctly.
        let pk = VerifyingKey::recover_from_prehash(&hash, &recovered_sig, recovered_recid)
            .expect("recover from prehash");
        assert_eq!(pubkey_to_address(&pk), expected_addr);
    }

    #[tokio::test]
    async fn local_signer_round_trip_via_recovery() {
        let signer = LocalKeySigner::from_hex(TEST_KEY).unwrap();
        let hash = keccak256(b"test payload");
        let signed = signer.sign_prehash(&hash).await.unwrap();
        let recovered =
            VerifyingKey::recover_from_prehash(&hash, &signed.signature, signed.recovery_id)
                .unwrap();
        assert_eq!(pubkey_to_address(&recovered), signer.address());
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;
    use super::tests::*;

    /// Hammer KMS with 10 signs to exercise both low-s and high-s
    /// recovery paths (ECDSA k is random, ~50% chance of high-s per call).
    #[tokio::test]
    #[ignore = "requires CHAIN_KMS_KEY env + GCP auth, makes 10 KMS calls"]
    async fn kms_signer_hammer() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let key = std::env::var("CHAIN_KMS_KEY").expect("CHAIN_KMS_KEY not set");
        let signer = KmsSigner::new(key).await.unwrap();
        for i in 0..10 {
            let hash = keccak256(format!("kms hammer {}", i).as_bytes());
            let signed = signer.sign_prehash(&hash).await.unwrap();
            let pk = VerifyingKey::recover_from_prehash(&hash, &signed.signature, signed.recovery_id).unwrap();
            assert_eq!(pubkey_to_address(&pk), signer.address(), "iteration {}", i);
        }
    }
}
