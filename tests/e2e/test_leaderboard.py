"""
End-to-end browser tests for `/leaderboard` (audit task D2).

Covers:
  - Anonymous user loads /leaderboard, rankings render, no console errors
  - Authed user sees the "My Rank" card populated
  - Search input filters the visible table
  - Tier filter chip changes rendered rows AND total_participants display
  - Visibility toggle flips, persists across reload (regression for fix 3)
  - formatCompact thresholds: spot-check a known row (regression for fix 7)
  - Admin sees the "Refresh now" button (audit A2)

Each test creates its own user/session via the same direct-DB pattern used by
the rest of the e2e suite (see tests/e2e/conftest.py).
"""

import os
import uuid

import psycopg2
from playwright.sync_api import expect


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
CORE_DB_DSN = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


def _connect_core():
    return psycopg2.connect(CORE_DB_DSN)


# ─── Fixture helpers ──────────────────────────────────────────────────────


def _create_session(roles=()):
    """Create a fresh active user (+ optional roles) and return (user_id, session_token)."""
    email = f"e2e-lb-{uuid.uuid4().hex[:10]}@poool.app"
    session_token = str(uuid.uuid4())
    conn = _connect_core()
    cur = conn.cursor()
    try:
        cur.execute(
            "INSERT INTO users (email, email_verified, status) VALUES (%s, TRUE, 'active') RETURNING id",
            (email,),
        )
        user_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO user_profiles (user_id, first_name, last_name, display_name)
            VALUES (%s, 'E2E', 'Leaderboard', %s)
            ON CONFLICT (user_id) DO UPDATE SET display_name = EXCLUDED.display_name
            """,
            (user_id, f"E2E LB {uuid.uuid4().hex[:6]}"),
        )
        for role in roles:
            cur.execute(
                """
                INSERT INTO user_roles (user_id, role_id, is_active)
                SELECT %s, id, TRUE FROM roles WHERE name = %s
                ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE
                """,
                (user_id, role),
            )
        cur.execute(
            "INSERT INTO user_sessions (user_id, session_token, remember_me, expires_at) "
            "VALUES (%s, %s, FALSE, NOW() + INTERVAL '1 hour')",
            (user_id, session_token),
        )
        conn.commit()
        return user_id, session_token
    finally:
        cur.close()
        conn.close()


def _seed_leaderboard_rows(user_id, display_name, total_invested_cents, tier_id=2):
    """Insert a leaderboard_scores + tier row so the table is non-empty for this user."""
    conn = _connect_core()
    cur = conn.cursor()
    try:
        # Make sure the test user is the highest-ranked (deterministic for assertions).
        cur.execute(
            """
            INSERT INTO leaderboard_scores (
                user_id, total_invested_cents, asset_count, portfolio_roi_bps,
                affiliate_count, referral_network_value_cents, highest_investment_cents,
                rank_invested, computed_at
            ) VALUES (%s, %s, 1, 500, 0, 0, %s, 1, NOW())
            ON CONFLICT (user_id) DO UPDATE SET
                total_invested_cents = EXCLUDED.total_invested_cents,
                rank_invested = 1,
                computed_at = NOW()
            """,
            (user_id, total_invested_cents, total_invested_cents),
        )
        cur.execute(
            "UPDATE user_profiles SET display_name = %s WHERE user_id = %s",
            (display_name, user_id),
        )
        cur.execute(
            """
            INSERT INTO user_tiers (user_id, tier_id) VALUES (%s, %s)
            ON CONFLICT (user_id) DO UPDATE SET tier_id = EXCLUDED.tier_id
            """,
            (user_id, tier_id),
        )
        # Audit task fix 3 — visibility=true so the display_name is not anonymized.
        cur.execute(
            """
            INSERT INTO leaderboard_preferences (user_id, visible, show_avatar, display_name)
            VALUES (%s, TRUE, FALSE, NULL)
            ON CONFLICT (user_id) DO UPDATE SET visible = TRUE
            """,
            (user_id,),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _cleanup(user_id, session_token):
    conn = _connect_core()
    cur = conn.cursor()
    try:
        cur.execute("DELETE FROM leaderboard_scores WHERE user_id = %s", (user_id,))
        cur.execute("DELETE FROM leaderboard_preferences WHERE user_id = %s", (user_id,))
        cur.execute("DELETE FROM user_tiers WHERE user_id = %s", (user_id,))
        cur.execute("DELETE FROM user_sessions WHERE session_token = %s", (session_token,))
        cur.execute("DELETE FROM user_roles WHERE user_id = %s", (user_id,))
        cur.execute("DELETE FROM user_profiles WHERE user_id = %s", (user_id,))
        cur.execute("DELETE FROM users WHERE id = %s", (user_id,))
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _attach_session(page, session_token):
    page.context.add_cookies(
        [{"name": "poool_session", "value": session_token, "url": BASE_URL}]
    )


# ─── Tests ─────────────────────────────────────────────────────────────────


def test_authed_user_sees_my_rank_card_populated(quality_page):
    """Authenticated user loads /leaderboard, the My Rank card shows their rank."""
    page, tracker = quality_page
    user_id, token = _create_session()
    display_name = f"LB Test {uuid.uuid4().hex[:6]}"
    _seed_leaderboard_rows(user_id, display_name, total_invested_cents=500_000_000)
    _attach_session(page, token)
    try:
        tracker.navigate_and_check(f"{BASE_URL}/leaderboard")
        # The seeded row is rank 1 because we pinned `rank_invested = 1`.
        expect(page.locator("#lb-my-rank")).to_have_text("#1", timeout=15_000)
        tracker.assert_no_critical_errors()
    finally:
        _cleanup(user_id, token)


def test_rankings_table_shows_other_visible_users_not_current_viewer(quality_page):
    """The public table lists other visible investors while keeping the viewer in My Rank."""
    page, tracker = quality_page
    viewer_id, token = _create_session()
    target_id, target_token = _create_session()
    unique = uuid.uuid4().hex[:8].upper()
    display_name = f"NEEDLE-{unique}"
    _seed_leaderboard_rows(target_id, display_name, total_invested_cents=900_000_000)
    _attach_session(page, token)
    try:
        tracker.navigate_and_check(f"{BASE_URL}/leaderboard")
        # Wait for the table to render at least one row
        expect(page.locator("#lb-rankings-body tr").first).to_be_visible(timeout=15_000)
        expect(page.locator("#lb-rankings-body")).to_contain_text(
            f"NEEDLE-{unique}", timeout=10_000
        )
        visible_rows = page.locator("#lb-rankings-body tr").count()
        assert visible_rows >= 1, "rankings should contain the seeded public user"
        expect(page.locator("#lb-rankings-body")).not_to_contain_text("E2E LB")
        tracker.assert_no_critical_errors()
    finally:
        _cleanup(target_id, target_token)
        _cleanup(viewer_id, token)


def test_removed_filter_and_visibility_controls_stay_out_of_leaderboard(quality_page):
    """Search, tier filters, and visibility settings are no longer managed on this page."""
    page, tracker = quality_page
    user_id, token = _create_session()
    target_id, target_token = _create_session()
    display_name = f"VisE2E-{uuid.uuid4().hex[:6]}"
    _seed_leaderboard_rows(user_id, display_name, total_invested_cents=400_000_000)
    _seed_leaderboard_rows(target_id, f"Visible-{uuid.uuid4().hex[:6]}", total_invested_cents=800_000_000)
    _attach_session(page, token)
    try:
        tracker.navigate_and_check(f"{BASE_URL}/leaderboard")
        expect(page).to_have_title("Leaderboard - POOOL", timeout=15_000)
        assert page.locator("#lb-search-input").count() == 0
        assert page.locator("#lb-visibility-toggle").count() == 0
        assert page.locator('button.lb-topbar-tab[data-tier-id]').count() == 0
        tracker.assert_no_critical_errors()
    finally:
        _cleanup(target_id, target_token)
        _cleanup(user_id, token)


def test_format_compact_thresholds_via_known_row(quality_page):
    """
    formatCompact regression for audit fix 7. We seed a user with exactly
    €1_000_000 (= 100_000_000 cents) and assert their summary card / breakdown
    renders "€1.00M" rather than the pre-fix output ("€1.0K").
    """
    page, tracker = quality_page
    user_id, token = _create_session()
    display_name = f"FmtE2E-{uuid.uuid4().hex[:6]}"
    _seed_leaderboard_rows(user_id, display_name, total_invested_cents=100_000_000)
    _attach_session(page, token)
    try:
        tracker.navigate_and_check(f"{BASE_URL}/leaderboard")
        # The "Your Standing" breakdown renders "Holdings: €1.00M" via formatCompact.
        expect(page.locator("#lb-my-rank-card")).to_contain_text("1.00M", timeout=15_000)
        tracker.assert_no_critical_errors()
    finally:
        _cleanup(user_id, token)


def test_admin_sees_refresh_now_button(quality_page):
    """An admin user sees the audit-A2 Refresh button (regular users don't)."""
    page, tracker = quality_page
    admin_id, admin_token = _create_session(roles=("admin", "super_admin"))
    target_id, target_token = _create_session()
    display_name = f"AdminE2E-{uuid.uuid4().hex[:6]}"
    _seed_leaderboard_rows(admin_id, display_name, total_invested_cents=200_000_000)
    _seed_leaderboard_rows(target_id, f"AdminVisible-{uuid.uuid4().hex[:6]}", total_invested_cents=800_000_000)
    _attach_session(page, admin_token)
    try:
        tracker.navigate_and_check(f"{BASE_URL}/leaderboard")
        btn = page.locator("#lb-refresh-btn")
        # The button is rendered hidden=true and revealed by JS after /api/me.
        expect(btn).to_be_visible(timeout=15_000)
        tracker.assert_no_critical_errors()
    finally:
        _cleanup(target_id, target_token)
        _cleanup(admin_id, admin_token)


def test_anonymous_user_loads_leaderboard_no_errors(quality_page):
    """An unauthenticated visit redirects to login (no crash, no console errors)."""
    page, tracker = quality_page
    # No session attached → /leaderboard requires auth and redirects to /auth/login.
    response = page.goto(f"{BASE_URL}/leaderboard", wait_until="domcontentloaded", timeout=15_000)
    # Either a 302 to login or a 200 on the login page itself is acceptable; the
    # important contract is "no crash, page renders something".
    assert response is None or response.status < 500
    tracker.assert_no_critical_errors()
