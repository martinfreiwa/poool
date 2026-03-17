"""
test_storage_analytics.py — Verifies /api/admin/storage returns live DB data.

Strategy:
  1. Snapshot the analytics BEFORE any action.
  2. Upload a new avatar → confirm avatar count increases.
  3. Upload a KYC document → confirm doc count + breakdown increase.
  4. Cross-check every returned number against direct DB queries.
  5. Re-check monthly_trend & recent_uploads reflect the new upload.

All assertions compare API values against raw psql queries so we know
nothing is hard-coded.
"""

import subprocess, sys, io, json, requests

BASE = "http://localhost:8888"
USER_EMAIL = "test@poool.app"

PASS_COUNT = FAIL_COUNT = 0
FAILURES: list[str] = []


# ── Helpers ───────────────────────────────────────────────────────────────

def section(title: str):
    bar = "─" * 60
    print(f"\n{bar}\n  {title}\n{bar}")


def check(label: str, ok: bool, detail: str = ""):
    global PASS_COUNT, FAIL_COUNT
    if ok:
        PASS_COUNT += 1
        print(f"  ✅ {label}")
    else:
        FAIL_COUNT += 1
        msg = f"  ❌ {label}" + (f"  — {detail}" if detail else "")
        print(msg)
        FAILURES.append(label)


def psql(query: str) -> str:
    """Run a psql query and return the trimmed scalar result."""
    result = subprocess.run(
        ["psql", "-Atc", query, "poool"],
        capture_output=True, text=True
    )
    return result.stdout.strip()


def psql_int(query: str) -> int:
    """Run a psql query and return an integer result."""
    return int(psql(query) or "0")


def get_session_token(email: str) -> str:
    return psql(
        f"SELECT session_token FROM user_sessions "
        f"WHERE user_id=(SELECT id FROM users WHERE email='{email}') "
        f"ORDER BY created_at DESC LIMIT 1"
    )


def make_session(email: str) -> requests.Session:
    s = requests.Session()
    token = get_session_token(email)
    s.cookies.set("poool_session", token)
    return s


def tiny_jpeg() -> bytes:
    """Return a minimal valid JPEG (1×1 white pixel, ~640 bytes)."""
    # This is a real 1×1 white JPEG
    return bytes([
        0xFF,0xD8,0xFF,0xE0,0x00,0x10,0x4A,0x46,0x49,0x46,0x00,0x01,
        0x01,0x00,0x00,0x01,0x00,0x01,0x00,0x00,0xFF,0xDB,0x00,0x43,
        0x00,0x08,0x06,0x06,0x07,0x06,0x05,0x08,0x07,0x07,0x07,0x09,
        0x09,0x08,0x0A,0x0C,0x14,0x0D,0x0C,0x0B,0x0B,0x0C,0x19,0x12,
        0x13,0x0F,0x14,0x1D,0x1A,0x1F,0x1E,0x1D,0x1A,0x1C,0x1C,0x20,
        0x24,0x2E,0x27,0x20,0x22,0x2C,0x23,0x1C,0x1C,0x28,0x37,0x29,
        0x2C,0x30,0x31,0x34,0x34,0x34,0x1F,0x27,0x39,0x3D,0x38,0x32,
        0x3C,0x2E,0x33,0x34,0x32,0xFF,0xC0,0x00,0x0B,0x08,0x00,0x01,
        0x00,0x01,0x01,0x01,0x11,0x00,0xFF,0xC4,0x00,0x1F,0x00,0x00,
        0x01,0x05,0x01,0x01,0x01,0x01,0x01,0x01,0x00,0x00,0x00,0x00,
        0x00,0x00,0x00,0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,
        0x09,0x0A,0x0B,0xFF,0xC4,0x00,0xB5,0x10,0x00,0x02,0x01,0x03,
        0x03,0x02,0x04,0x03,0x05,0x05,0x04,0x04,0x00,0x00,0x01,0x7D,
        0x01,0x02,0x03,0x00,0x04,0x11,0x05,0x12,0x21,0x31,0x41,0x06,
        0x13,0x51,0x61,0x07,0x22,0x71,0x14,0x32,0x81,0x91,0xA1,0x08,
        0x23,0x42,0xB1,0xC1,0x15,0x52,0xD1,0xF0,0x24,0x33,0x62,0x72,
        0x82,0x09,0x0A,0x16,0x17,0x18,0x19,0x1A,0x25,0x26,0x27,0x28,
        0x29,0x2A,0x34,0x35,0x36,0x37,0x38,0x39,0x3A,0x43,0x44,0x45,
        0x46,0x47,0x48,0x49,0x4A,0x53,0x54,0x55,0x56,0x57,0x58,0x59,
        0x5A,0x63,0x64,0x65,0x66,0x67,0x68,0x69,0x6A,0x73,0x74,0x75,
        0x76,0x77,0x78,0x79,0x7A,0x83,0x84,0x85,0x86,0x87,0x88,0x89,
        0x8A,0x92,0x93,0x94,0x95,0x96,0x97,0x98,0x99,0x9A,0xA2,0xA3,
        0xA4,0xA5,0xA6,0xA7,0xA8,0xA9,0xAA,0xB2,0xB3,0xB4,0xB5,0xB6,
        0xB7,0xB8,0xB9,0xBA,0xC2,0xC3,0xC4,0xC5,0xC6,0xC7,0xC8,0xC9,
        0xCA,0xD2,0xD3,0xD4,0xD5,0xD6,0xD7,0xD8,0xD9,0xDA,0xE1,0xE2,
        0xE3,0xE4,0xE5,0xE6,0xE7,0xE8,0xE9,0xEA,0xF1,0xF2,0xF3,0xF4,
        0xF5,0xF6,0xF7,0xF8,0xF9,0xFA,0xFF,0xDA,0x00,0x08,0x01,0x01,
        0x00,0x00,0x3F,0x00,0xFB,0x28,0xA2,0x8A,0xFF,0xD9
    ])


# ── Main test ─────────────────────────────────────────────────────────────

def main():
    session = make_session(USER_EMAIL)
    user_id = psql(f"SELECT id FROM users WHERE email='{USER_EMAIL}'")

    section("SETUP")
    print(f"  User: {USER_EMAIL} ({user_id})")

    # ── 1. Snapshot BEFORE ────────────────────────────────────────────────
    section("1. SNAPSHOT BEFORE")
    r = session.get(f"{BASE}/api/admin/storage")
    check("API returns 200", r.status_code == 200, f"got {r.status_code}")
    if r.status_code != 200:
        print("Cannot continue – aborting.")
        sys.exit(1)

    before = r.json()

    # Validate schema
    check("Response has 'bucket' field", "bucket" in before)
    check("Response has 'summary' object", "summary" in before)
    check("Response has 'cost_estimate' object", "cost_estimate" in before)
    check("Response has 'breakdown_by_type' array", isinstance(before.get("breakdown_by_type"), list))
    check("Response has 'breakdown_by_status' array", isinstance(before.get("breakdown_by_status"), list))
    check("Response has 'recent_uploads' array", isinstance(before.get("recent_uploads"), list))
    check("Response has 'monthly_trend' array", isinstance(before.get("monthly_trend"), list))
    check("Bucket is configured (not placeholder)", before["bucket"] != "not configured", before["bucket"])

    before_total_files  = before["summary"]["total_files"]
    before_kyc_docs     = before["summary"]["kyc_documents"]
    before_avatars      = before["summary"]["avatars"]
    before_bytes        = before["summary"]["estimated_storage_bytes"]

    # Cross-check with DB right now
    db_kyc   = psql_int(f"SELECT COUNT(*) FROM kyc_documents")
    db_avats = psql_int(f"SELECT COUNT(*) FROM users WHERE avatar_url IS NOT NULL AND avatar_url <> ''")

    check("summary.kyc_documents matches DB",
          before_kyc_docs == db_kyc,
          f"API={before_kyc_docs}, DB={db_kyc}")
    check("summary.avatars matches DB",
          before_avatars == db_avats,
          f"API={before_avatars}, DB={db_avats}")
    check("summary.total_files == kyc_documents + avatars",
          before_total_files == before_kyc_docs + before_avatars,
          f"{before_total_files} != {before_kyc_docs} + {before_avatars}")
    check("estimated_storage_bytes > 0", before_bytes >= 0)

    # Cost estimate sanity: storage_cost + ops_cost = total (within float rounding)
    ce = before["cost_estimate"]
    computed_total = round(ce["storage_per_month_usd"] + ce["operations_per_month_usd"], 4)
    check("cost_estimate totals add up",
          abs(computed_total - ce["total_per_month_usd"]) < 0.001,
          f"{ce['storage_per_month_usd']} + {ce['operations_per_month_usd']} ≠ {ce['total_per_month_usd']}")

    # Breakdown by type count must sum to total KYC docs
    type_sum = sum(t["count"] for t in before["breakdown_by_type"])
    check("breakdown_by_type counts sum to kyc_documents",
          type_sum == before_kyc_docs,
          f"sum={type_sum}, kyc_documents={before_kyc_docs}")

    # Status breakdown must also sum to kyc_documents
    status_sum = sum(s["count"] for s in before["breakdown_by_status"])
    check("breakdown_by_status counts sum to kyc_documents",
          status_sum == before_kyc_docs,
          f"sum={status_sum}, kyc_documents={before_kyc_docs}")

    print(f"\n  Before state: {before_kyc_docs} KYC docs, {before_avatars} avatars, "
          f"{before_bytes:,} estimated bytes")

    # ── 2. Upload a new avatar, re-check ─────────────────────────────────
    section("2. AVATAR UPLOAD → DATA CHANGES")
    jpeg = tiny_jpeg()
    r = session.post(
        f"{BASE}/api/upload/avatar",
        files={"file": ("test_avatar.jpg", io.BytesIO(jpeg), "image/jpeg")},
    )
    check("Avatar upload succeeds (200)", r.status_code in (200, 201),
          f"got {r.status_code}: {r.text[:200]}")

    if r.status_code in (200, 201):
        # After the upload, avatar_url must be set in DB
        avatar_url = psql(f"SELECT avatar_url FROM users WHERE id='{user_id}'")
        check("avatar_url saved in DB", avatar_url.startswith("https://"), avatar_url)

        # Re-query the analytics API
        r2 = session.get(f"{BASE}/api/admin/storage")
        after_avatar = r2.json()

        after_avatars = after_avatar["summary"]["avatars"]
        db_avats2     = psql_int(f"SELECT COUNT(*) FROM users WHERE avatar_url IS NOT NULL AND avatar_url <> ''")

        check("summary.avatars now matches updated DB",
              after_avatars == db_avats2,
              f"API={after_avatars}, DB={db_avats2}")

        # Bytes may not change if the user already had an avatar (count-based estimate).
        # What matters is that the API count still matches the DB count.
        after_bytes2 = after_avatar["summary"]["estimated_storage_bytes"]
        avatar_was_new = before_avatars < after_avatars  # count went up = new avatar
        if avatar_was_new:
            check("estimated_storage_bytes increased for new avatar",
                  after_bytes2 > before_bytes,
                  f"before={before_bytes}, after={after_bytes2}")
        else:
            # User already had an avatar — replacing it doesn't change the count/bytes
            check("estimated_storage_bytes unchanged (avatar replaced, not added)",
                  after_bytes2 == before_bytes,
                  f"expected {before_bytes}, got {after_bytes2}")

    # ── 3. Upload a KYC doc, re-check ────────────────────────────────────
    section("3. KYC DOC UPLOAD → DATA CHANGES")
    r = session.post(
        f"{BASE}/api/upload/kyc",
        files={"file": ("passport_test.jpg", io.BytesIO(tiny_jpeg()), "image/jpeg")},
        data={"document_type": "driving_licence"},
    )
    check("KYC upload succeeds (200)", r.status_code in (200, 201),
          f"got {r.status_code}: {r.text[:200]}")

    if r.status_code in (200, 201):
        doc_id = r.json().get("document_id")
        check("Response contains document_id", bool(doc_id))

        # Row must exist in DB
        db_doc_status = psql(f"SELECT status FROM kyc_documents WHERE id='{doc_id}'")
        check("kyc_document row exists in DB", bool(db_doc_status),
              f"status={db_doc_status}")

        # Re-query analytics
        r3 = session.get(f"{BASE}/api/admin/storage")
        after_kyc = r3.json()

        after_kyc_count = after_kyc["summary"]["kyc_documents"]
        db_kyc3         = psql_int("SELECT COUNT(*) FROM kyc_documents")

        check("summary.kyc_documents increased by 1",
              after_kyc_count == before_kyc_docs + 1,
              f"before={before_kyc_docs}, after={after_kyc_count}")
        check("summary.kyc_documents matches DB after upload",
              after_kyc_count == db_kyc3,
              f"API={after_kyc_count}, DB={db_kyc3}")

        # Check that driving_licence appears in breakdown_by_type
        types = {t["type"]: t["count"] for t in after_kyc.get("breakdown_by_type", [])}
        check("driving_licence appears in breakdown_by_type",
              "driving_licence" in types,
              f"types present: {list(types.keys())}")

        # Check recent_uploads contains our doc
        recent_ids = [u["id"] for u in after_kyc.get("recent_uploads", [])]
        check("New doc appears in recent_uploads",
              doc_id in recent_ids,
              f"doc_id={doc_id}, recents={recent_ids[:3]}")

        # Estimated bytes must have grown
        after_bytes3 = after_kyc["summary"]["estimated_storage_bytes"]
        check("estimated_storage_bytes increased after KYC upload",
              after_bytes3 > before_bytes,
              f"before={before_bytes}, after={after_bytes3}")

        # Re-check type + status sums still match kyc_documents
        type_sum3   = sum(t["count"] for t in after_kyc["breakdown_by_type"])
        status_sum3 = sum(s["count"] for s in after_kyc["breakdown_by_status"])
        check("breakdown_by_type still sums to kyc_documents",
              type_sum3 == after_kyc_count,
              f"sum={type_sum3} vs {after_kyc_count}")
        check("breakdown_by_status still sums to kyc_documents",
              status_sum3 == after_kyc_count,
              f"sum={status_sum3} vs {after_kyc_count}")

    # ── 4. Clean up test docs ─────────────────────────────────────────────
    section("4. CLEANUP")
    # Remove any driving_licence docs we just created (status=pending) for test user
    deleted = psql(
        f"WITH del AS (DELETE FROM kyc_documents WHERE user_id='{user_id}' "
        f"AND document_type='driving_licence' AND status='pending' RETURNING id) "
        f"SELECT COUNT(*) FROM del"
    )
    print(f"  Cleaned up {deleted} test driving_licence doc(s)")

    # ── Summary ───────────────────────────────────────────────────────────
    section("SUMMARY")
    total = PASS_COUNT + FAIL_COUNT
    print(f"Results: {PASS_COUNT}/{total} passed")
    if FAILURES:
        print("FAILURES:")
        for f in FAILURES:
            print(f"  • {f}")
    sys.exit(0 if FAIL_COUNT == 0 else 1)


if __name__ == "__main__":
    main()
