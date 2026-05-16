"""
E2E Playwright test: Community DM flow between two members.

Pre-seeds Alice + Bob + an initial thread directly in the DB (skips the
"find a recipient" search step, which depends on display-name autocomplete
that isn't the focus of this test). Then drives the browser as Alice:
   1. Navigate to /community?tab=dms
   2. Confirm the thread shows up in the left rail
   3. Open the thread → confirm initial message renders
   4. Reply via the compose form
   5. Verify the reply appears in the message list within 5s
   6. Switch browser context to Bob → load page → confirm Alice's reply
      is visible AND marked as read (read_at_recipient flips)

Negative paths still owned by the bash e2e (scripts/test_dm_e2e.sh) —
this test only validates the UI rendering + interactivity layer.

Run:
    pytest tests/e2e/test_community_dm_flow.py -v
or
    pytest tests/e2e/ -m community -k dm_flow
"""

import os
import time
import uuid
import psycopg2
import pytest
from playwright.sync_api import expect

# Conftest functions live in tests/e2e/conftest.py — pytest auto-loads them
# but they're not import-able as a regular module. Re-declare the few
# helpers we need from there (matches the pattern in test_community.py).

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
COMMUNITY_DB_URL = os.environ.get(
    "COMMUNITY_DATABASE_URL",
    "postgres://martin@localhost/poool_community",
)


def _community_conn():
    return psycopg2.connect(COMMUNITY_DB_URL)


def _core_conn():
    return psycopg2.connect(DB_URL)


def _create_user(prefix, display_name):
    """Mint a fresh user + verified session directly in DB."""
    user_id = str(uuid.uuid4())
    session_token = str(uuid.uuid4())
    email = f"{prefix}-{user_id[:8]}@poool.test"
    conn = _core_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            """
            INSERT INTO users (id, email, password_hash, status, email_verified)
            VALUES (%s, %s, 'x', 'active', TRUE)
            """,
            (user_id, email),
        )
        cur.execute(
            """
            INSERT INTO user_profiles (user_id, first_name, last_name, display_name, annual_income_cents)
            VALUES (%s, %s, %s, %s, 100000000)
            ON CONFLICT (user_id) DO NOTHING
            """,
            (user_id, display_name.split(" ")[0], "DM", display_name, ),
        )
        cur.execute(
            """
            INSERT INTO user_sessions (user_id, session_token, expires_at, is_2fa_verified)
            VALUES (%s, %s, NOW() + INTERVAL '1 hour', TRUE)
            """,
            (user_id, session_token),
        )
        conn.commit()
        return {"user_id": user_id, "session_token": session_token, "email": email}
    finally:
        conn.close()


def _cleanup_user(user_id):
    uid = str(user_id)
    conn = _core_conn()
    try:
        cur = conn.cursor()
        for sql in [
            "DELETE FROM user_sessions WHERE user_id = %s",
            "DELETE FROM user_profiles WHERE user_id = %s",
            "DELETE FROM users WHERE id = %s",
        ]:
            try:
                cur.execute(sql, (uid,))
            except Exception:
                conn.rollback()
        conn.commit()
    finally:
        conn.close()


def _attach_session(context, session_token):
    context.add_cookies([
        {"name": "poool_session", "value": session_token, "url": BASE_URL}
    ])


def _ensure_community_profile(user_id):
    """Idempotent seed of community_profiles for an E2E test user."""
    conn = _community_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            "INSERT INTO community_profiles (user_id) VALUES (%s) "
            "ON CONFLICT (user_id) DO NOTHING",
            (str(user_id),),
        )
        conn.commit()
    finally:
        conn.close()


def _seed_dm_thread(sender_id, recipient_id, content):
    """Insert a thread + first message. Mirrors `create_dm_thread()` Rust handler."""
    a, b = sorted([str(sender_id), str(recipient_id)])
    conn = _community_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            """
            INSERT INTO dm_threads (participant_a_id, participant_b_id, last_message_at)
            VALUES (%s, %s, NOW())
            ON CONFLICT (participant_a_id, participant_b_id)
            DO UPDATE SET last_message_at = NOW()
            RETURNING id
            """,
            (a, b),
        )
        thread_id = cur.fetchone()[0]
        cur.execute(
            "INSERT INTO dm_messages (thread_id, sender_id, content) VALUES (%s, %s, %s) RETURNING id",
            (str(thread_id), str(sender_id), content),
        )
        msg_id = cur.fetchone()[0]
        conn.commit()
        return thread_id, msg_id
    finally:
        conn.close()


def _read_receipts_count(thread_id, sender_id):
    """Count unread messages from `sender_id` in `thread_id`."""
    conn = _community_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            """
            SELECT COUNT(*) FROM dm_messages
            WHERE thread_id = %s AND sender_id = %s AND read_at_recipient IS NULL
            """,
            (str(thread_id), str(sender_id)),
        )
        return cur.fetchone()[0]
    finally:
        conn.close()


def _cleanup_dm_artifacts(*user_ids):
    """Remove DM threads + messages + community profile for the given users."""
    if not user_ids:
        return
    ids = [str(u) for u in user_ids]
    conn = _community_conn()
    try:
        cur = conn.cursor()
        for tbl, col in [
            ("dm_messages", "sender_id"),
            ("dm_threads", "participant_a_id"),
            ("dm_threads", "participant_b_id"),
            ("block_relationships", "actor_user_id"),
            ("block_relationships", "target_user_id"),
            ("community_profiles", "user_id"),
        ]:
            for uid in ids:
                try:
                    cur.execute(
                        f"DELETE FROM {tbl} WHERE {col} = %s", (uid,)
                    )
                except Exception:
                    conn.rollback()
        conn.commit()
    finally:
        conn.close()


# ─── Fixtures ──────────────────────────────────────────────────────────

@pytest.fixture(scope="function")
def dm_pair(playwright_session, request):
    """
    Seeds 2 fresh users + community profiles + an initial DM thread
    started by Alice → Bob. Yields:
        (alice, bob, thread_id, first_message_id)
    each `*_session_token` resolvable via attach_session_cookie.

    Cleans up community + core rows on teardown.
    """
    alice = _create_user("e2e-dm-alice", "Alice DM")
    bob = _create_user("e2e-dm-bob", "Bob DM")
    _ensure_community_profile(alice["user_id"])
    _ensure_community_profile(bob["user_id"])

    thread_id, msg_id = _seed_dm_thread(
        alice["user_id"], bob["user_id"], "Hey Bob 👋 first DM (from fixture)"
    )

    yield alice, bob, str(thread_id), str(msg_id)

    _cleanup_dm_artifacts(alice["user_id"], bob["user_id"])
    _cleanup_user(alice["user_id"])
    _cleanup_user(bob["user_id"])


# ─── Tests ─────────────────────────────────────────────────────────────

@pytest.mark.community
def test_dm_thread_renders_in_ui(playwright_session, dm_pair, request):
    """Alice loads /community?tab=dms — the seeded thread shows + first message renders."""
    alice, bob, thread_id, _ = dm_pair

    context = playwright_session.new_context()
    context.add_init_script(
        "localStorage.setItem('poool_cookie_consent', "
        "JSON.stringify({granted_at:'2026-01-01T00:00:00.000Z',"
        "preferences:{essential:true,analytics:true,marketing:true}}));"
    )
    _attach_session(context, alice["session_token"])
    page = context.new_page()
    js_errors = []
    page.on("pageerror", lambda e: js_errors.append(str(e)))

    try:
        page.goto(f"{BASE_URL}/community?tab=dms", wait_until="domcontentloaded", timeout=15000)
        # Thread list populated (the seed message is "Hey Bob 👋 first DM…")
        thread_list = page.locator("#community-dm-thread-list")
        expect(thread_list).to_be_visible(timeout=10000)
        # Wait until the loading placeholder is replaced.
        page.wait_for_function(
            "() => !document.getElementById('community-dm-thread-list')?.textContent?.includes('Loading')",
            timeout=10000,
        )
        # At least one thread row contains Bob's display name.
        bob_row = page.locator("#community-dm-thread-list").get_by_text("Bob DM", exact=False)
        expect(bob_row.first).to_be_visible(timeout=5000)
        assert not js_errors, f"JS errors: {js_errors[:5]}"
    finally:
        context.close()


@pytest.mark.community
def test_dm_reply_appears_in_message_list(playwright_session, dm_pair, request):
    """Alice opens the thread + posts a reply — it renders in the message list."""
    alice, bob, thread_id, _ = dm_pair

    context = playwright_session.new_context()
    context.add_init_script(
        "localStorage.setItem('poool_cookie_consent', "
        "JSON.stringify({granted_at:'2026-01-01T00:00:00.000Z',"
        "preferences:{essential:true,analytics:true,marketing:true}}));"
    )
    _attach_session(context, alice["session_token"])
    page = context.new_page()
    js_errors = []
    page.on("pageerror", lambda e: js_errors.append(str(e)))

    try:
        page.goto(f"{BASE_URL}/community?tab=dms", wait_until="domcontentloaded", timeout=15000)

        # Wait for thread list load + click the thread containing Bob.
        page.wait_for_function(
            "() => !document.getElementById('community-dm-thread-list')?.textContent?.includes('Loading')",
            timeout=10000,
        )
        bob_row = page.locator("#community-dm-thread-list").get_by_text("Bob DM", exact=False).first
        bob_row.click()

        # Wait for active conversation panel.
        active = page.locator("#community-dm-active")
        expect(active).to_be_visible(timeout=5000)

        # Seed message should be visible.
        expect(page.locator("#community-dm-message-list")).to_contain_text(
            "Hey Bob", timeout=5000
        )

        # Compose + send a reply.
        reply_text = f"UI-test reply {uuid.uuid4().hex[:6]}"
        page.fill("#community-dm-input", reply_text)
        page.click("#community-dm-compose button[type='submit']")

        # Reply renders.
        expect(page.locator("#community-dm-message-list")).to_contain_text(
            reply_text, timeout=5000
        )
        assert not js_errors, f"JS errors: {js_errors[:5]}"
    finally:
        context.close()


@pytest.mark.community
def test_dm_read_receipt_marks_seen_for_recipient(playwright_session, dm_pair, request):
    """Bob loading the thread flips Alice's unread messages to read."""
    alice, bob, thread_id, _ = dm_pair

    # Pre-condition: Alice's seed message is unread for Bob.
    unread_before = _read_receipts_count(thread_id, alice["user_id"])
    assert unread_before >= 1, f"Expected ≥1 unread, got {unread_before}"

    context = playwright_session.new_context()
    context.add_init_script(
        "localStorage.setItem('poool_cookie_consent', "
        "JSON.stringify({granted_at:'2026-01-01T00:00:00.000Z',"
        "preferences:{essential:true,analytics:true,marketing:true}}));"
    )
    _attach_session(context, bob["session_token"])
    page = context.new_page()
    js_errors = []
    page.on("pageerror", lambda e: js_errors.append(str(e)))

    try:
        page.goto(f"{BASE_URL}/community?tab=dms", wait_until="domcontentloaded", timeout=15000)
        page.wait_for_function(
            "() => !document.getElementById('community-dm-thread-list')?.textContent?.includes('Loading')",
            timeout=10000,
        )
        # Open thread with Alice.
        alice_row = page.locator("#community-dm-thread-list").get_by_text("Alice DM", exact=False).first
        alice_row.click()
        expect(page.locator("#community-dm-active")).to_be_visible(timeout=5000)
        expect(page.locator("#community-dm-message-list")).to_contain_text(
            "Hey Bob", timeout=5000
        )

        # Give the backend up to 2s to commit the read_at_recipient UPDATE.
        for _ in range(20):
            time.sleep(0.1)
            if _read_receipts_count(thread_id, alice["user_id"]) == 0:
                break

        unread_after = _read_receipts_count(thread_id, alice["user_id"])
        assert unread_after == 0, (
            f"Expected 0 unread after Bob viewed thread, still {unread_after}"
        )
        assert not js_errors, f"JS errors: {js_errors[:5]}"
    finally:
        context.close()
