//! Shared multipart upload helpers.
//!
//! Centralises the "read a multipart field with a hard byte cap" pattern so
//! callers cannot accidentally call `field.bytes().await` (which buffers the
//! entire payload first, letting a malicious client burn up to the request
//! body limit in RAM before any size check runs).
//!
//! Mirrors the chunked-read loop used by the KYC upload at
//! `routes.rs::upload_kyc_document` — `field.chunk()` is awaited in a loop
//! and the running total is compared against `max_bytes` before each
//! `extend_from_slice` so an oversized upload is rejected as soon as the
//! threshold is crossed.

use crate::admin::extractors::ApiError;
use axum::extract::multipart::Field;

/// Read a multipart field chunk-by-chunk, rejecting as soon as the running
/// length exceeds `max_bytes`.
///
/// Returns `ApiError::BadRequest` on overflow (`"<label> must be ≤ <N> bytes"`)
/// or on a chunk-read error (`"Invalid <label> upload body"`). Both message
/// shapes are intentionally kept generic so they can be displayed to the
/// client without leaking implementation detail.
pub async fn read_field_capped(
    field: &mut Field<'_>,
    max_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, ApiError> {
    let mut bytes: Vec<u8> = Vec::with_capacity(8 * 1024);
    loop {
        match field.chunk().await {
            Ok(Some(chunk)) => {
                if bytes.len().saturating_add(chunk.len()) > max_bytes {
                    return Err(ApiError::BadRequest(format!(
                        "{} must be ≤ {} bytes",
                        label, max_bytes
                    )));
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(_) => {
                return Err(ApiError::BadRequest(format!(
                    "Invalid {} upload body",
                    label
                )));
            }
        }
    }
    Ok(bytes)
}
