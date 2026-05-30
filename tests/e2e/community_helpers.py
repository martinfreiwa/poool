"""Shared helpers for community-feature Playwright tests.

Lives next to conftest.py so any test_*.py in this directory can:
    from community_helpers import mint_user, attach_session, cleanup_user, …

Pytest auto-adds tests/e2e/ to sys.path via conftest.
"""

import os
import uuid
import psycopg2

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
COMMUNITY_DB_URL = os.environ.get(
    "COMMUNITY_DATABASE_URL",
    "postgres://martin@localhost/poool_community",
)


def core_conn():
    return psycopg2.connect(DB_URL)


def comm_conn():
    return psycopg2.connect(COMMUNITY_DB_URL)


def mint_user(prefix="e2e-ui", display_name="UI Tester"):
    """Create a verified user + session + community profile. Returns dict."""
    uid = str(uuid.uuid4())
    tok = str(uuid.uuid4())
    email = f"{prefix}-{uid[:8]}@poool.test"
    conn = core_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            "INSERT INTO users (id, email, password_hash, status, email_verified) "
            "VALUES (%s, %s, 'x', 'active', TRUE)",
            (uid, email),
        )
        cur.execute(
            """
            INSERT INTO user_profiles (user_id, first_name, last_name, display_name, annual_income_cents)
            VALUES (%s, %s, 'UI', %s, 100000000)
            ON CONFLICT (user_id) DO NOTHING
            """,
            (uid, display_name.split(" ")[0], display_name),
        )
        cur.execute(
            "INSERT INTO user_sessions (user_id, session_token, expires_at, is_2fa_verified) "
            "VALUES (%s, %s, NOW() + INTERVAL '1 hour', TRUE)",
            (uid, tok),
        )
        conn.commit()
    finally:
        conn.close()
    # community profile (separate DB)
    conn = comm_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            "INSERT INTO community_profiles (user_id) VALUES (%s) ON CONFLICT (user_id) DO NOTHING",
            (uid,),
        )
        conn.commit()
    finally:
        conn.close()
    return {"user_id": uid, "session_token": tok, "email": email,
            "display_name": display_name}


def mint_admin(prefix="e2e-ui-admin", display_name="UI Admin"):
    """Like mint_user, but also grants the admin + super_admin roles so the
    returned user can reach `/admin/*` routes. Same dict shape as mint_user."""
    user = mint_user(prefix=prefix, display_name=display_name)
    conn = core_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            """
            INSERT INTO user_roles (user_id, role_id, is_active)
            SELECT %s, id, TRUE
            FROM roles
            WHERE name IN ('admin', 'super_admin')
            ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE
            """,
            (user["user_id"],),
        )
        conn.commit()
    finally:
        conn.close()
    return user


def attach_session(context, session_token):
    """Add the poool_session cookie to a Playwright context."""
    context.add_cookies([
        {"name": "poool_session", "value": session_token, "url": BASE_URL}
    ])


def install_cookie_consent(context):
    """Pre-set the cookie-consent localStorage so banners don't obscure tests."""
    context.add_init_script(
        "localStorage.setItem('poool_cookie_consent', "
        "JSON.stringify({granted_at:'2026-01-01T00:00:00.000Z',"
        "preferences:{essential:true,analytics:true,marketing:true}}));"
    )


def make_context(browser, user, viewport="desktop"):
    """Helper: new browser context wired with cookie + session + JS-error tracking.

    Pass `viewport="mobile"` to get an iPhone-sized (375x812, touch, mobile UA)
    context — used by the Wave F mobile-regression tests.
    """
    if viewport == "mobile":
        ctx = browser.new_context(
            viewport={"width": 375, "height": 812},
            has_touch=True,
            is_mobile=True,
            service_workers="block",
            user_agent=(
                "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) "
                "AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 "
                "Mobile/15E148 Safari/604.1"
            ),
        )
    else:
        ctx = browser.new_context(service_workers="block")
    install_cookie_consent(ctx)
    attach_session(ctx, user["session_token"])
    page = ctx.new_page()
    errors = []
    page.on("pageerror", lambda e: errors.append(str(e)))
    return ctx, page, errors


def cleanup_user(user_id):
    """Remove every community + core trace of the user."""
    uid = str(user_id)
    # Community DB
    conn = comm_conn()
    try:
        cur = conn.cursor()
        for sql in [
            "DELETE FROM dm_messages WHERE sender_id = %s "
            "OR thread_id IN (SELECT id FROM dm_threads WHERE participant_a_id = %s OR participant_b_id = %s)",
            "DELETE FROM dm_threads  WHERE participant_a_id = %s OR participant_b_id = %s",
            "DELETE FROM block_relationships WHERE actor_user_id = %s OR target_user_id = %s",
            "DELETE FROM mute_relationships  WHERE actor_user_id = %s OR target_user_id = %s",
            "DELETE FROM reactions WHERE user_id = %s",
            "DELETE FROM post_bookmarks WHERE user_id = %s",
            "DELETE FROM comments WHERE user_id = %s",
            # Polls cascade from posts, but delete vote rows the user cast
            # in OTHER people's polls explicitly. Polls/options for own posts
            # get cleaned via the posts cascade.
            "DELETE FROM poll_votes WHERE user_id = %s",
            "DELETE FROM posts WHERE user_id = %s",
            "DELETE FROM follows WHERE follower_id = %s OR followee_id = %s",
            "DELETE FROM notifications WHERE user_id = %s OR actor_id = %s",
            "DELETE FROM circle_members WHERE user_id = %s",
            "DELETE FROM circle_bans WHERE banned_user_id = %s OR banned_by = %s",
            "DELETE FROM circles WHERE owner_id = %s",
            "DELETE FROM verified_owner_requests WHERE user_id = %s",
            "DELETE FROM xp_ledger WHERE user_id = %s",
            "DELETE FROM community_profiles WHERE user_id = %s",
        ]:
            try:
                # Substitute %s placeholders with the right count.
                cur.execute(sql, (uid,) * sql.count("%s"))
            except Exception:
                conn.rollback()
        conn.commit()
    finally:
        conn.close()
    # Core DB
    conn = core_conn()
    try:
        cur = conn.cursor()
        for sql in [
            "DELETE FROM notifications WHERE user_id = %s OR actor_id = %s",
            "DELETE FROM user_roles WHERE user_id = %s",
            "DELETE FROM user_sessions WHERE user_id = %s",
            "DELETE FROM user_profiles WHERE user_id = %s",
            "DELETE FROM users WHERE id = %s",
        ]:
            try:
                cur.execute(sql, (uid,) * sql.count("%s"))
            except Exception:
                conn.rollback()
        conn.commit()
    finally:
        conn.close()


def seed_post(user_id, content="UI test post", post_type="general"):
    """Insert a post directly into community DB. Returns post_id."""
    conn = comm_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            "INSERT INTO posts (user_id, content, post_type, is_hidden) "
            "VALUES (%s, %s, %s, FALSE) RETURNING id",
            (str(user_id), content, post_type),
        )
        pid = cur.fetchone()[0]
        conn.commit()
        return str(pid)
    finally:
        conn.close()


def seed_circle(owner_id, name="UI Test Circle", slug=None):
    """Insert a public circle. Returns circle_id."""
    if not slug:
        slug = "ui-" + str(uuid.uuid4())[:8]
    conn = comm_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            "INSERT INTO circles (owner_id, name, slug, is_public, max_members, member_count) "
            "VALUES (%s, %s, %s, TRUE, 50, 1) RETURNING id",
            (str(owner_id), name, slug),
        )
        cid = cur.fetchone()[0]
        cur.execute(
            "INSERT INTO circle_members (circle_id, user_id, role) VALUES (%s, %s, 'owner')",
            (str(cid), str(owner_id)),
        )
        conn.commit()
        return {"id": str(cid), "slug": slug, "name": name}
    finally:
        conn.close()


def seed_notification(user_id, title="UI test notification",
                      message="Hello from the helper",
                      ntype="system"):
    """Insert a notification row in the CORE DB. Returns notification_id."""
    conn = core_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            "INSERT INTO notifications (user_id, title, message, type, is_read) "
            "VALUES (%s, %s, %s, %s, FALSE) RETURNING id",
            (str(user_id), title, message, ntype),
        )
        nid = cur.fetchone()[0]
        conn.commit()
        return str(nid)
    finally:
        conn.close()


def seed_poll(user_id, question="Best chain for RWA?", options=None):
    """Insert a post + attached poll with options. Returns dict:
    {post_id, poll_id, option_ids: [id, ...]}.
    """
    opts = options or ["Ethereum", "Solana", "Polygon"]
    conn = comm_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            "INSERT INTO posts (user_id, content, post_type, is_hidden) "
            "VALUES (%s, %s, 'general', FALSE) RETURNING id",
            (str(user_id), question),
        )
        pid = cur.fetchone()[0]
        cur.execute(
            "INSERT INTO polls (post_id, question) VALUES (%s, %s) RETURNING id",
            (str(pid), question),
        )
        poll_id = cur.fetchone()[0]
        opt_ids = []
        for idx, label in enumerate(opts):
            cur.execute(
                "INSERT INTO poll_options (poll_id, label, sort_order) "
                "VALUES (%s, %s, %s) RETURNING id",
                (str(poll_id), label, idx),
            )
            opt_ids.append(cur.fetchone()[0])
        conn.commit()
        return {
            "post_id": str(pid),
            "poll_id": str(poll_id),
            "option_ids": [str(i) for i in opt_ids],
        }
    finally:
        conn.close()


def seed_dm_thread(sender_id, recipient_id, content="Hi from helper"):
    """Insert a DM thread + first message. Returns (thread_id, message_id)."""
    a, b = sorted([str(sender_id), str(recipient_id)])
    conn = comm_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            "INSERT INTO dm_threads (participant_a_id, participant_b_id, last_message_at) "
            "VALUES (%s, %s, NOW()) "
            "ON CONFLICT (participant_a_id, participant_b_id) "
            "DO UPDATE SET last_message_at = NOW() RETURNING id",
            (a, b),
        )
        tid = cur.fetchone()[0]
        cur.execute(
            "INSERT INTO dm_messages (thread_id, sender_id, content) "
            "VALUES (%s, %s, %s) RETURNING id",
            (str(tid), str(sender_id), content),
        )
        mid = cur.fetchone()[0]
        conn.commit()
        return str(tid), str(mid)
    finally:
        conn.close()
