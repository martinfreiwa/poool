//! Storage Phase 4 audit tests — Compliance + Retention invariants.
//!
//! Schema-introspection tests don't need a live GCS bucket. The
//! end-to-end retention worker test against a real bucket is gated
//! behind `#[ignore]` because it requires production-ish creds.

use poool_backend::storage::service::PiiClass;

// ─── Pure: PiiClass canonicalization ──────────────────────────────

#[test]
fn pii_class_as_str_matches_gcs_metadata_contract() {
    // These are the literal values stored in `x-goog-meta-pii-class`.
    // Changing them is a breaking change — the reconciler + retention
    // worker query on these strings.
    assert_eq!(PiiClass::A.as_str(), "A");
    assert_eq!(PiiClass::B.as_str(), "B");
    assert_eq!(PiiClass::C.as_str(), "C");
    assert_eq!(PiiClass::None.as_str(), "none");
}

#[test]
fn pii_class_retention_trigger_defaults_match_stakeholder_doc_q7() {
    // Q7 decision: "5 Jahre nach Business-Beziehungs-Ende" → both
    // A and B map to business_end+5y. C + None have no retention.
    assert_eq!(PiiClass::A.default_retention_trigger(), "business_end+5y");
    assert_eq!(PiiClass::B.default_retention_trigger(), "business_end+5y");
    assert_eq!(PiiClass::C.default_retention_trigger(), "none");
    assert_eq!(PiiClass::None.default_retention_trigger(), "none");
}

// ─── DB-backed: migration 200 schema ──────────────────────────────

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn users_table_has_business_relationship_ended_at() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    let (exists,): (bool,) = sqlx::query_as(
        "SELECT EXISTS(
            SELECT 1 FROM information_schema.columns
            WHERE table_name='users' AND column_name='business_relationship_ended_at'
         )",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        exists,
        "users.business_relationship_ended_at missing (migration 200 not applied?)"
    );
}

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn kyc_documents_has_retention_columns() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    for col in ["retention_until", "deleted_at", "deletion_reason"] {
        let (exists,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(
                SELECT 1 FROM information_schema.columns
                WHERE table_name='kyc_documents' AND column_name=$1
             )",
        )
        .bind(col)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(exists, "kyc_documents.{} missing", col);
    }
}

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn kyc_retention_runs_table_exists_with_status_check() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    // Insert with bogus status should be rejected by the CHECK constraint.
    let bad: Result<_, sqlx::Error> =
        sqlx::query("INSERT INTO kyc_retention_runs (status) VALUES ('doomsday')")
            .execute(&pool)
            .await;
    assert!(bad.is_err(), "status CHECK must reject 'doomsday'");

    // Insert with a valid status should succeed (and we clean up).
    let row: (uuid::Uuid,) =
        sqlx::query_as("INSERT INTO kyc_retention_runs (status) VALUES ('success') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("DELETE FROM kyc_retention_runs WHERE id = $1")
        .bind(row.0)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn arm_kyc_retention_for_user_function_exists_and_is_idempotent() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    // Create a throwaway user.
    let email = format!("retention-test-{}@example.invalid", uuid::Uuid::new_v4());
    let uid: (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO users (email, email_verified, status)
         VALUES ($1, false, 'active') RETURNING id",
    )
    .bind(&email)
    .fetch_one(&pool)
    .await
    .unwrap();

    // First call: sets business_relationship_ended_at.
    let n1: (i32,) = sqlx::query_as("SELECT arm_kyc_retention_for_user($1, 5)")
        .bind(uid.0)
        .fetch_one(&pool)
        .await
        .unwrap();
    // No KYC docs exist for this user yet, so 0 updates expected.
    assert_eq!(n1.0, 0);

    let (end_after_first,): (Option<chrono::DateTime<chrono::Utc>>,) =
        sqlx::query_as("SELECT business_relationship_ended_at FROM users WHERE id = $1")
            .bind(uid.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        end_after_first.is_some(),
        "first call must set business_relationship_ended_at"
    );

    // Second call: idempotent — must NOT overwrite the original end-date.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let _: (i32,) = sqlx::query_as("SELECT arm_kyc_retention_for_user($1, 5)")
        .bind(uid.0)
        .fetch_one(&pool)
        .await
        .unwrap();

    let (end_after_second,): (Option<chrono::DateTime<chrono::Utc>>,) =
        sqlx::query_as("SELECT business_relationship_ended_at FROM users WHERE id = $1")
            .bind(uid.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        end_after_first, end_after_second,
        "second call must not overwrite the original end-date"
    );

    // Cleanup.
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(uid.0)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn retention_until_index_used_for_worker_scan() {
    // The migration 200 partial index is shaped exactly for the
    // worker's hot query:
    //   WHERE retention_until IS NOT NULL AND deleted_at IS NULL
    //   ORDER BY retention_until ASC LIMIT 1000
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    sqlx::query("ANALYZE kyc_documents")
        .execute(&pool)
        .await
        .ok();

    let plan: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN SELECT id, gcs_path FROM kyc_documents
         WHERE retention_until IS NOT NULL
           AND retention_until <= NOW()
           AND deleted_at IS NULL
         ORDER BY retention_until ASC
         LIMIT 1000",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let plan_text = plan
        .iter()
        .map(|p| p.0.clone())
        .collect::<Vec<_>>()
        .join("\n");

    // Either the partial index or another index is acceptable; what we
    // refuse is a bare Seq Scan over the whole table once there's real
    // data. For an empty/tiny dataset planner may pick Seq Scan; we
    // only assert "no full seq-scan WHEN table is non-trivial". Skip
    // the assertion gracefully if the table is below the threshold.
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM kyc_documents")
        .fetch_one(&pool)
        .await
        .unwrap();
    if count < 500 {
        eprintln!(
            "skipping index assertion (kyc_documents has {} rows, need >=500)",
            count
        );
        return;
    }
    assert!(
        plan_text.contains("Index") || plan_text.contains("index"),
        "expected an Index plan, got:\n{}",
        plan_text
    );
}

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn retention_partial_index_actually_exists() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    let (exists,): (bool,) = sqlx::query_as(
        "SELECT EXISTS(
            SELECT 1 FROM pg_indexes
            WHERE tablename = 'kyc_documents'
              AND indexname = 'idx_kyc_documents_retention_due'
         )",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        exists,
        "idx_kyc_documents_retention_due index missing (migration 200 not applied?)"
    );
}
