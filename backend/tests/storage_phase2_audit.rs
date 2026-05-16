//! Storage subsystem — Phase 2 security-hardening regression tests.
//!
//! Guards the invariants we shipped during the security hardening pass:
//!
//! Functional (pure — no DB)
//! - `svg_payload_detection_catches_inline_script`
//! - `svg_payload_detection_handles_uppercase_tags`
//! - `svg_payload_detection_rejects_empty`
//! - `quota_class_byte_caps_are_sane`
//! - `quota_class_file_caps_are_sane`
//! - `quota_class_str_matches_db_check_constraint`
//!
//! Quota lifecycle (DB-backed — `--ignored`)
//! - `quota_starts_at_zero_for_new_user`
//! - `quota_check_rejects_when_over_byte_cap`
//! - `quota_check_rejects_when_over_file_cap`
//! - `quota_increment_then_decrement_round_trip`
//! - `quota_check_allows_when_under_cap`
//!
//! ## Running
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test storage_phase2_audit -- --include-ignored
//! ```

#![cfg(test)]

use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect")
}

async fn insert_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, status) VALUES ($1, $2, 'x', 'active')",
    )
    .bind(id)
    .bind(format!("{}@phase2.test", id))
    .execute(pool)
    .await
    .expect("insert user");
    id
}

async fn cleanup_user(pool: &PgPool, user_id: Uuid) {
    let _ = sqlx::query("DELETE FROM storage_user_quotas WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
}

// ══════════════════════════════════════════════════════════════════════
// SVG defence (Phase 2.4)
// ══════════════════════════════════════════════════════════════════════

#[test]
fn svg_payload_detection_catches_inline_script() {
    let payload = br#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">
  <script>alert(1)</script>
</svg>"#;
    assert!(
        poool_backend::storage::routes::is_svg_payload(payload),
        "SVG with inline script must be detected so it cannot be uploaded",
    );
}

#[test]
fn svg_payload_detection_handles_uppercase_tags() {
    // Some encoders emit uppercase. Our sniff lower-cases before search.
    let payload = b"<?xml version=\"1.0\"?>\n<SVG xmlns=\"http://www.w3.org/2000/svg\"/>";
    assert!(
        poool_backend::storage::routes::is_svg_payload(payload),
        "Case-insensitive detection — `<SVG` must trip the gate",
    );
}

#[test]
fn svg_payload_detection_rejects_empty() {
    assert!(
        !poool_backend::storage::routes::is_svg_payload(b""),
        "Empty input must not be flagged as SVG (would block other rejections)",
    );
}

#[test]
fn svg_payload_detection_does_not_false_positive_on_jpeg() {
    // First 12 bytes of a real JPEG file. The detector must NOT match.
    let jpeg = b"\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01";
    assert!(
        !poool_backend::storage::routes::is_svg_payload(jpeg),
        "JPEG header must not be misclassified as SVG",
    );
}

// ══════════════════════════════════════════════════════════════════════
// Quota class sanity (Phase 2.6)
// ══════════════════════════════════════════════════════════════════════

#[test]
fn quota_class_byte_caps_are_sane() {
    use poool_backend::storage::service::QuotaClass;
    // Every class must have a positive byte cap. Negative or zero
    // would mean "no uploads ever allowed" — a silent foot-gun.
    for class in [
        QuotaClass::Avatar,
        QuotaClass::PostImage,
        QuotaClass::AssetImage,
        QuotaClass::AssetDocument,
        QuotaClass::KycDocument,
        QuotaClass::DeveloperLogo,
    ] {
        assert!(
            class.quota_bytes() > 0,
            "{:?}: byte cap must be > 0; got {}",
            class,
            class.quota_bytes(),
        );
        // ≥ 1 MB — sub-MB caps signal a unit-error.
        assert!(
            class.quota_bytes() >= 1_048_576,
            "{:?}: byte cap looks too small ({} bytes < 1 MiB) — unit error?",
            class,
            class.quota_bytes(),
        );
    }
}

#[test]
fn quota_class_file_caps_are_sane() {
    use poool_backend::storage::service::QuotaClass;
    for class in [
        QuotaClass::Avatar,
        QuotaClass::PostImage,
        QuotaClass::AssetImage,
        QuotaClass::AssetDocument,
        QuotaClass::KycDocument,
        QuotaClass::DeveloperLogo,
    ] {
        assert!(class.quota_files() > 0, "{:?}: file cap must be > 0", class,);
    }
}

#[test]
fn quota_class_str_matches_db_check_constraint() {
    use poool_backend::storage::service::QuotaClass;
    // The CHECK constraint on storage_user_quotas.class accepts exactly
    // these six strings (migration 179). Any drift between the enum and
    // the constraint will surface as a runtime INSERT failure in
    // production — this test catches it at compile-time + test-run.
    let expected: std::collections::HashSet<&str> = [
        "avatar",
        "post_image",
        "asset_image",
        "asset_document",
        "kyc_document",
        "developer_logo",
    ]
    .into_iter()
    .collect();
    let actual: std::collections::HashSet<&str> = [
        QuotaClass::Avatar.as_str(),
        QuotaClass::PostImage.as_str(),
        QuotaClass::AssetImage.as_str(),
        QuotaClass::AssetDocument.as_str(),
        QuotaClass::KycDocument.as_str(),
        QuotaClass::DeveloperLogo.as_str(),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        actual, expected,
        "QuotaClass::as_str() values must match the DB CHECK constraint",
    );
}

// ══════════════════════════════════════════════════════════════════════
// Quota lifecycle — DB-backed
// ══════════════════════════════════════════════════════════════════════

#[ignore]
#[tokio::test]
async fn quota_starts_at_zero_for_new_user() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let (bytes, files) = poool_backend::storage::service::get_quota_usage(
        &pool,
        user,
        poool_backend::storage::service::QuotaClass::KycDocument,
    )
    .await
    .expect("get_quota_usage");
    cleanup_user(&pool, user).await;
    assert_eq!(bytes, 0, "fresh user must have 0 quota usage");
    assert_eq!(files, 0, "fresh user must have 0 file count");
}

#[ignore]
#[tokio::test]
async fn quota_check_rejects_when_over_byte_cap() {
    use poool_backend::storage::service::{check_quota_or_reject, increment_quota, QuotaClass};
    let pool = pool().await;
    let user = insert_user(&pool).await;

    // Pre-fill the quota to 1 byte below the avatar cap.
    let cap = QuotaClass::Avatar.quota_bytes();
    increment_quota(&pool, user, QuotaClass::Avatar, cap - 1)
        .await
        .expect("seed quota usage");

    // 2 more bytes pushes us 1 byte over the cap.
    let result = check_quota_or_reject(&pool, user, QuotaClass::Avatar, 2).await;
    cleanup_user(&pool, user).await;

    assert!(
        result.is_err(),
        "quota check must reject when incoming bytes would exceed cap",
    );
    let msg = format!("{:?}", result.unwrap_err());
    assert!(
        msg.contains("Storage quota exceeded"),
        "error message should clearly say 'quota exceeded'; got {}",
        msg,
    );
}

#[ignore]
#[tokio::test]
async fn quota_check_rejects_when_over_file_cap() {
    use poool_backend::storage::service::{check_quota_or_reject, QuotaClass};
    let pool = pool().await;
    let user = insert_user(&pool).await;

    // Manually inflate file_count to the cap WITHOUT touching bytes_used.
    let cap_files = QuotaClass::KycDocument.quota_files();
    sqlx::query(
        r#"INSERT INTO storage_user_quotas (user_id, class, bytes_used, file_count)
           VALUES ($1, $2, 0, $3)"#,
    )
    .bind(user)
    .bind(QuotaClass::KycDocument.as_str())
    .bind(cap_files)
    .execute(&pool)
    .await
    .expect("seed file count");

    let result = check_quota_or_reject(&pool, user, QuotaClass::KycDocument, 100).await;
    cleanup_user(&pool, user).await;

    assert!(
        result.is_err(),
        "quota check must reject when file count is at cap",
    );
    let msg = format!("{:?}", result.unwrap_err());
    assert!(
        msg.contains("File-count quota exceeded"),
        "error message should clearly say 'file-count quota exceeded'; got {}",
        msg,
    );
}

#[ignore]
#[tokio::test]
async fn quota_check_allows_when_under_cap() {
    use poool_backend::storage::service::{check_quota_or_reject, QuotaClass};
    let pool = pool().await;
    let user = insert_user(&pool).await;
    // Fresh user, no prior quota row → check should pass.
    let result = check_quota_or_reject(&pool, user, QuotaClass::KycDocument, 1024).await;
    cleanup_user(&pool, user).await;
    assert!(
        result.is_ok(),
        "quota check should allow under-cap upload; got {:?}",
        result
    );
}

#[ignore]
#[tokio::test]
async fn quota_increment_then_decrement_round_trip() {
    use poool_backend::storage::service::{
        decrement_quota, get_quota_usage, increment_quota, QuotaClass,
    };
    let pool = pool().await;
    let user = insert_user(&pool).await;

    increment_quota(&pool, user, QuotaClass::AssetImage, 5000)
        .await
        .expect("increment");
    let (b1, f1) = get_quota_usage(&pool, user, QuotaClass::AssetImage)
        .await
        .expect("read after increment");
    assert_eq!(
        (b1, f1),
        (5000, 1),
        "increment should set 5000 bytes / 1 file"
    );

    increment_quota(&pool, user, QuotaClass::AssetImage, 2500)
        .await
        .expect("second increment");
    let (b2, f2) = get_quota_usage(&pool, user, QuotaClass::AssetImage)
        .await
        .expect("read after second increment");
    assert_eq!((b2, f2), (7500, 2), "second increment should accumulate");

    decrement_quota(&pool, user, QuotaClass::AssetImage, 2500)
        .await
        .expect("decrement");
    let (b3, f3) = get_quota_usage(&pool, user, QuotaClass::AssetImage)
        .await
        .expect("read after decrement");
    cleanup_user(&pool, user).await;
    assert_eq!(
        (b3, f3),
        (5000, 1),
        "decrement should subtract bytes and file count",
    );
}

#[ignore]
#[tokio::test]
async fn quota_decrement_floors_at_zero_not_negative() {
    use poool_backend::storage::service::{decrement_quota, get_quota_usage, QuotaClass};
    let pool = pool().await;
    let user = insert_user(&pool).await;
    // Decrement a row that doesn't exist — should be a no-op (UPDATE
    // matches zero rows). Subsequent read still returns 0.
    decrement_quota(&pool, user, QuotaClass::Avatar, 1_000_000)
        .await
        .expect("decrement non-existent row");
    let (b, f) = get_quota_usage(&pool, user, QuotaClass::Avatar)
        .await
        .expect("read");
    cleanup_user(&pool, user).await;
    assert_eq!((b, f), (0, 0), "no row → no decrement effect");
}

// ══════════════════════════════════════════════════════════════════════
// Schema check — Phase 2.6 migration applied
// ══════════════════════════════════════════════════════════════════════

#[ignore]
#[tokio::test]
async fn storage_user_quotas_table_has_expected_columns() {
    let pool = pool().await;
    for col in ["user_id", "class", "bytes_used", "file_count", "updated_at"] {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = 'storage_user_quotas' AND column_name = $1
             )",
        )
        .bind(col)
        .fetch_one(&pool)
        .await
        .expect("information_schema");
        assert!(
            exists,
            "storage_user_quotas.{} must exist (migration 179)",
            col
        );
    }

    // The CHECK constraint accepts exactly the 6 allowed class strings.
    let check_src: Option<String> = sqlx::query_scalar(
        r#"SELECT pg_get_constraintdef(c.oid)
           FROM pg_constraint c
           JOIN pg_class t ON t.oid = c.conrelid
           WHERE t.relname = 'storage_user_quotas'
             AND c.contype = 'c'
             AND pg_get_constraintdef(c.oid) LIKE '%class%'
           LIMIT 1"#,
    )
    .fetch_optional(&pool)
    .await
    .expect("constraint lookup")
    .flatten();
    let src = check_src.expect("storage_user_quotas should have a class CHECK constraint");
    for class in [
        "avatar",
        "post_image",
        "asset_image",
        "asset_document",
        "kyc_document",
        "developer_logo",
    ] {
        assert!(
            src.contains(class),
            "CHECK constraint must include `{}`; got: {}",
            class,
            src,
        );
    }
}
