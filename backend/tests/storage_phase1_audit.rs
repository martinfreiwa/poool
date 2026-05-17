//! Storage subsystem — Phase 1 production-readiness regression tests.
//!
//! Guards the invariants we shipped during the storage hardening pass:
//!
//! Functional
//! - `sha256_hex_is_stable_and_canonical`
//! - `sha256_hex_handles_empty_input`
//! - `is_local_fallback_disabled_in_production`
//! - `is_local_fallback_enabled_in_development`
//! - `upload_local_refuses_in_production_env`
//!
//! Schema (audit migration 178)
//! - `kyc_documents_has_content_sha256_column`
//! - `kyc_documents_has_uploaded_ip_column`
//! - `asset_documents_has_content_sha256_column`
//! - `asset_documents_has_uploaded_by_user_id_column`
//! - `sha256_dedup_index_exists`
//!
//! ## Running
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test storage_phase1_audit -- --ignored
//! ```
//!
//! Pure-function tests (no DB) run without `--ignored`.

#![cfg(test)]

use sqlx::{postgres::PgPoolOptions, PgPool};

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect")
}

// ══════════════════════════════════════════════════════════════════════
// Functional — pure functions, no DB
// ══════════════════════════════════════════════════════════════════════

/// SHA-256 of known input must produce the canonical RFC 6234 vector so
/// callers (DB INSERT, integrity-check on download) get the same hash
/// across processes, languages, and Rust versions.
#[test]
fn sha256_hex_is_stable_and_canonical() {
    let h = poool_backend::storage::service::sha256_hex(b"abc");
    assert_eq!(
        h, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "SHA-256(\"abc\") must match RFC 6234 test vector",
    );
}

/// Empty input is a valid edge case (e.g. zero-byte placeholder uploads
/// during dev). Must not panic and must produce the known empty hash.
#[test]
fn sha256_hex_handles_empty_input() {
    let h = poool_backend::storage::service::sha256_hex(b"");
    assert_eq!(
        h, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "SHA-256(\"\") must match the canonical empty-string hash",
    );
}

// Process-wide env vars (POOOL_ENV here) race when cargo runs unit tests
// concurrently. Wrap every test that flips POOOL_ENV in this mutex so
// the get/set/restore sequence is atomic.
static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Production safety: with `POOOL_ENV=production` (or unset), the local-FS
/// fallback gate must return false. Failure here means uploads silently
/// degrade to ephemeral container-FS on Cloud Run → data loss on restart.
#[test]
fn is_local_fallback_disabled_in_production() {
    let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::var("POOOL_ENV").ok();
    std::env::set_var("POOOL_ENV", "production");
    let allowed = poool_backend::storage::service::is_local_fallback_allowed();
    if let Some(p) = prev {
        std::env::set_var("POOOL_ENV", p);
    } else {
        std::env::remove_var("POOOL_ENV");
    }
    assert!(
        !allowed,
        "Local-FS fallback MUST be disabled when POOOL_ENV=production — \
         otherwise Cloud Run uploads silently degrade to ephemeral FS.",
    );
}

/// Dev convenience: with `POOOL_ENV=development|dev|local` the gate
/// returns true. Failure here means dev environments lose the working
/// upload path when GCS credentials are absent.
#[test]
fn is_local_fallback_enabled_in_development() {
    let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    for env_val in ["development", "dev", "local"] {
        let prev = std::env::var("POOOL_ENV").ok();
        std::env::set_var("POOOL_ENV", env_val);
        let allowed = poool_backend::storage::service::is_local_fallback_allowed();
        if let Some(p) = prev {
            std::env::set_var("POOOL_ENV", p);
        } else {
            std::env::remove_var("POOOL_ENV");
        }
        assert!(
            allowed,
            "Local-FS fallback MUST be enabled when POOOL_ENV={:?}",
            env_val,
        );
    }
}

/// Calling `upload_local` directly while POOOL_ENV is unset/production
/// returns an `AppError::Internal` with the production-safety message —
/// the silent-data-loss path is closed at the function boundary, not
/// only at the caller.
#[tokio::test]
async fn upload_local_refuses_in_production_env() {
    let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::var("POOOL_ENV").ok();
    std::env::set_var("POOOL_ENV", "production");

    let result = poool_backend::storage::service::upload_local(
        "test/refuses.txt",
        b"production-bytes".to_vec(),
    )
    .await;

    if let Some(p) = prev {
        std::env::set_var("POOOL_ENV", p);
    } else {
        std::env::remove_var("POOOL_ENV");
    }

    assert!(
        result.is_err(),
        "upload_local must return Err in production env; got {:?}",
        result,
    );
    let msg = format!("{:?}", result.unwrap_err());
    assert!(
        msg.contains("Local-FS upload-fallback disabled"),
        "error message should mention production safety; got {}",
        msg,
    );
}

// ══════════════════════════════════════════════════════════════════════
// Schema (migration 178) — DB connection required, --ignored
// ══════════════════════════════════════════════════════════════════════

async fn assert_column_exists(pool: &PgPool, table: &str, column: &str) {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_name = $1 AND column_name = $2
         )",
    )
    .bind(table)
    .bind(column)
    .fetch_one(pool)
    .await
    .expect("information_schema query");
    assert!(
        exists,
        "Column `{}.{}` must exist (migration 178)",
        table, column,
    );
}

#[ignore]
#[tokio::test]
async fn kyc_documents_has_content_sha256_column() {
    let pool = pool().await;
    assert_column_exists(&pool, "kyc_documents", "content_sha256").await;
    // Also check data type — CHAR(64) is the SHA-256 hex length.
    let data_type: String = sqlx::query_scalar(
        "SELECT data_type FROM information_schema.columns
         WHERE table_name = 'kyc_documents' AND column_name = 'content_sha256'",
    )
    .fetch_one(&pool)
    .await
    .expect("data_type");
    assert!(
        data_type.contains("character"),
        "content_sha256 should be CHAR(64); got {}",
        data_type,
    );
}

#[ignore]
#[tokio::test]
async fn kyc_documents_has_uploaded_ip_column() {
    let pool = pool().await;
    assert_column_exists(&pool, "kyc_documents", "uploaded_ip").await;
    assert_column_exists(&pool, "kyc_documents", "uploaded_user_agent").await;
    assert_column_exists(&pool, "kyc_documents", "content_size_bytes").await;
}

#[ignore]
#[tokio::test]
async fn asset_documents_has_content_sha256_column() {
    let pool = pool().await;
    assert_column_exists(&pool, "asset_documents", "content_sha256").await;
    assert_column_exists(&pool, "asset_documents", "uploaded_ip").await;
    assert_column_exists(&pool, "asset_documents", "uploaded_user_agent").await;
}

#[ignore]
#[tokio::test]
async fn asset_documents_has_uploaded_by_user_id_column() {
    let pool = pool().await;
    assert_column_exists(&pool, "asset_documents", "uploaded_by_user_id").await;
    // Verify the SET NULL FK constraint is in place — soft FK preserves
    // audit row after user-delete instead of cascading the doc away.
    let on_delete: Option<String> = sqlx::query_scalar(
        r#"SELECT rc.delete_rule
           FROM information_schema.referential_constraints rc
           JOIN information_schema.key_column_usage kcu
             ON kcu.constraint_name = rc.constraint_name
           WHERE kcu.table_name = 'asset_documents'
             AND kcu.column_name = 'uploaded_by_user_id'
           LIMIT 1"#,
    )
    .fetch_optional(&pool)
    .await
    .expect("delete_rule lookup")
    .flatten();
    assert_eq!(
        on_delete.as_deref(),
        Some("SET NULL"),
        "uploaded_by_user_id should use ON DELETE SET NULL to preserve audit trail",
    );
}

#[ignore]
#[tokio::test]
async fn sha256_dedup_index_exists() {
    let pool = pool().await;
    for (table, idx) in [
        ("kyc_documents", "idx_kyc_documents_sha256"),
        ("asset_documents", "idx_asset_documents_sha256"),
    ] {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM pg_indexes
                WHERE tablename = $1 AND indexname = $2
             )",
        )
        .bind(table)
        .bind(idx)
        .fetch_one(&pool)
        .await
        .expect("pg_indexes query");
        assert!(
            exists,
            "Index `{}` on `{}` must exist for SHA-256 dedup queries",
            idx, table,
        );
    }
}

// ══════════════════════════════════════════════════════════════════════
// Integration — write/read round-trip through real DB schema
// ══════════════════════════════════════════════════════════════════════

/// Round-trip: insert a kyc_documents row with the new audit columns,
/// read it back, confirm values survive. Verifies the migration types
/// accept what production handlers will write.
#[ignore]
#[tokio::test]
async fn kyc_documents_round_trip_with_audit_columns() {
    let pool = pool().await;
    let user_id = uuid::Uuid::new_v4();

    // Need a user row to satisfy the FK.
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, status) VALUES ($1, $2, 'x', 'active')",
    )
    .bind(user_id)
    .bind(format!("{}@phase1.test", user_id))
    .execute(&pool)
    .await
    .expect("insert user");

    let sha = poool_backend::storage::service::sha256_hex(b"hello world");
    let inserted_id: uuid::Uuid = sqlx::query_scalar(
        r#"INSERT INTO kyc_documents
             (user_id, document_type, gcs_path, status,
              content_sha256, content_size_bytes, uploaded_ip, uploaded_user_agent)
           VALUES ($1, 'passport', 'gs://test/path', 'pending',
                   $2, 11, '203.0.113.42'::inet, 'curl/8.0')
           RETURNING id"#,
    )
    .bind(user_id)
    .bind(&sha)
    .fetch_one(&pool)
    .await
    .expect("insert kyc_document with audit columns");

    let row: (Option<String>, Option<i64>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT content_sha256, content_size_bytes, host(uploaded_ip), uploaded_user_agent
         FROM kyc_documents WHERE id = $1",
    )
    .bind(inserted_id)
    .fetch_one(&pool)
    .await
    .expect("read back");

    // Cleanup before assert
    let _ = sqlx::query("DELETE FROM kyc_documents WHERE id = $1")
        .bind(inserted_id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await;

    assert_eq!(row.0.as_deref(), Some(sha.as_str()), "sha256 round-trip");
    assert_eq!(row.1, Some(11), "size round-trip");
    assert_eq!(row.2.as_deref(), Some("203.0.113.42"), "IP round-trip");
    assert_eq!(row.3.as_deref(), Some("curl/8.0"), "UA round-trip");
}

/// Dedup index is *usable*: planner picks Index/Bitmap when the table
/// has enough rows. Small tables prefer Seq Scan (cheaper at low cardinality),
/// so we seed a synthetic block of rows before EXPLAIN to force the
/// planner past the seq-scan threshold.
#[ignore]
#[tokio::test]
async fn sha256_dedup_query_uses_index() {
    let pool = pool().await;

    // Seed a temporary user + 250 kyc_documents rows so the planner has
    // enough cardinality to prefer the partial index over a seq-scan.
    let user_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, status) VALUES ($1, $2, 'x', 'active')",
    )
    .bind(user_id)
    .bind(format!("{}@dedup.test", user_id))
    .execute(&pool)
    .await
    .expect("seed user");

    // Bulk insert via generate_series for speed.
    sqlx::query(
        r#"INSERT INTO kyc_documents
             (user_id, document_type, gcs_path, status, content_sha256, content_size_bytes)
           SELECT $1, 'passport', 'gs://test/' || gs::text, 'pending',
                  md5(gs::text) || md5(gs::text), 100
           FROM generate_series(1, 250) AS gs"#,
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("bulk seed kyc rows");

    // ANALYZE so the planner sees the new stats.
    sqlx::query("ANALYZE kyc_documents")
        .execute(&pool)
        .await
        .expect("analyze");

    let plan: String = sqlx::query_scalar(
        "EXPLAIN (FORMAT TEXT) SELECT 1 FROM kyc_documents WHERE content_sha256 = 'abc'",
    )
    .fetch_one(&pool)
    .await
    .expect("explain");

    // Cleanup
    let _ = sqlx::query("DELETE FROM kyc_documents WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await;

    // Either Index-Scan or Bitmap-Index-Scan is acceptable evidence that
    // the partial index was picked up by the planner.
    assert!(
        plan.contains("Index")
            || plan.contains("Bitmap")
            || plan.contains("idx_kyc_documents_sha256"),
        "SHA-256 lookup should use the partial index after seeding 250 rows; plan was:\n{}",
        plan,
    );
}
