import re
import uuid
from playwright.sync_api import expect
import os
import json
import psycopg2

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
COMMUNITY_DB_URL = os.environ.get("COMMUNITY_DATABASE_URL", "postgres://martin@localhost/poool_community")


def get_core_db_connection():
    return psycopg2.connect(DB_URL)


def get_community_db_connection():
    return psycopg2.connect(COMMUNITY_DB_URL)


def test_community_feed_load(authenticated_user_page):
    page, tracker, current_user = authenticated_user_page

    # Community is a single page with tabs — default tab is Feed
    page.goto(f"{BASE_URL}/community")
    page.wait_for_load_state("networkidle")

    # Assert that the current community shell and HTMX content host are visible.
    expect(page.locator("#community-body .lb-container").first).to_be_visible(timeout=10000)
    expect(page.locator("#community-content-area").first).to_be_visible(timeout=10000)

    # Verify the Feed tab is active by default
    feed_tab = page.locator("button.community-tab-btn.active", has_text="Feed")
    expect(feed_tab).to_be_visible(timeout=5000)

    # Attempt to create a post if the text area exists
    post_textarea = page.locator("textarea[name='content']").first
    if post_textarea.is_visible():
        post_textarea.fill("Hello from Playwright E2E!")
        post_button = page.locator("button[type='submit']").first
        if post_button.is_visible():
            post_button.click()
            # Wait for success UI indication
            expect(post_textarea).to_have_value("")

    tracker.assert_no_critical_errors()


def test_community_feed_reaction_comment_accessibility(authenticated_user_page):
    page, tracker, current_user = authenticated_user_page
    post_content = f"E2E community feed regression {uuid.uuid4().hex}"
    initial_comment = f"E2E comment {uuid.uuid4().hex}"
    csrf_token = uuid.uuid4().hex
    post_id = None

    core_conn = get_core_db_connection()
    core_cur = core_conn.cursor()
    community_conn = get_community_db_connection()
    community_cur = community_conn.cursor()
    try:
        core_cur.execute(
            """
            INSERT INTO user_settings (user_id, totp_enabled)
            VALUES (%s, FALSE)
            ON CONFLICT (user_id) DO UPDATE SET totp_enabled = FALSE
            """,
            (current_user["user_id"],),
        )
        core_cur.execute(
            "UPDATE user_sessions SET is_2fa_verified = TRUE WHERE user_id = %s",
            (current_user["user_id"],),
        )
        core_conn.commit()

        community_cur.execute(
            "INSERT INTO community_profiles (user_id) VALUES (%s) ON CONFLICT (user_id) DO NOTHING",
            (current_user["user_id"],),
        )
        community_cur.execute(
            """
            INSERT INTO posts (user_id, post_type, content, content_sanitized)
            VALUES (%s, 'general', %s, %s)
            RETURNING id
            """,
            (current_user["user_id"], post_content, post_content),
        )
        post_id = community_cur.fetchone()[0]
        community_cur.execute(
            """
            INSERT INTO reactions (post_id, user_id, reaction_type)
            VALUES (%s, %s, 'fire')
            """,
            (post_id, current_user["user_id"]),
        )
        community_conn.commit()

        page.context.add_cookies([
            {"name": "csrf_token", "value": csrf_token, "url": BASE_URL},
        ])
        page.goto(f"{BASE_URL}/community")
        page.wait_for_load_state("networkidle")

        post = page.locator(f"#post-{post_id}")
        expect(post).to_be_visible(timeout=10000)
        expect(post).to_contain_text(post_content)

        reaction_btn = post.locator("button.feed-reaction-btn[aria-label='React to post']")
        expect(reaction_btn).to_have_attribute("aria-pressed", "true")
        expect(reaction_btn).to_have_class(re.compile(r"\bactive\b"))
        expect(reaction_btn.locator("span")).to_have_text("1")

        with page.expect_response(
            lambda response: response.url.endswith(f"/api/community/posts/{post_id}/reactions")
            and response.request.method == "POST"
        ) as reaction_response:
            reaction_btn.click()
        assert reaction_response.value.status == 200
        reaction_payload = reaction_response.value.json()
        assert reaction_payload["added"] is False
        assert reaction_payload["reaction_count"] == 0
        expect(reaction_btn).to_have_attribute("aria-pressed", "false")
        expect(reaction_btn.locator("span")).to_have_text("0")

        # Two buttons share aria-controls=comments-section-… (the small
        # reaction-bar icon button + the larger "X reactions · Y comments"
        # text button). Narrow to the icon button via class to keep the
        # selector unambiguous.
        comment_btn = post.locator(
            f"button.feed-reaction-btn[aria-controls='comments-section-{post_id}']"
        )
        expect(comment_btn).to_have_attribute("aria-label", "Show comments")
        expect(comment_btn).to_have_attribute("aria-expanded", "false")
        comment_btn.click()
        expect(comment_btn).to_have_attribute("aria-expanded", "true")
        expect(page.locator(f"#comments-section-{post_id}")).to_be_visible()

        expect(post.locator(f"label[for='comment-input-{post_id}']")).to_have_text("Write a comment")
        comment_input = page.locator(f"#comment-input-{post_id}")
        comment_input.fill(initial_comment)
        with page.expect_response(
            lambda response: response.url.endswith(f"/api/community/posts/{post_id}/comments")
            and response.request.method == "POST"
        ) as comment_response:
            post.locator("#comments-section-%s button" % post_id, has_text="Post").click()
        assert comment_response.value.status == 200
        expect(page.locator(f"#comments-list-{post_id}")).to_contain_text(initial_comment)

        expect(post.locator(f"#bookmark-btn-{post_id}")).to_have_attribute("aria-pressed", re.compile("true|false"))
        expect(post.locator("button[aria-label='Report post']")).to_be_visible()

        community_cur.execute("SELECT comment_count FROM posts WHERE id = %s", (post_id,))
        assert community_cur.fetchone()[0] == 1

        tracker.assert_no_critical_errors()
    finally:
        if post_id is not None:
            community_cur.execute("DELETE FROM posts WHERE id = %s", (post_id,))
        community_cur.execute(
            "DELETE FROM community_profiles WHERE user_id = %s",
            (current_user["user_id"],),
        )
        community_conn.commit()
        community_cur.close()
        community_conn.close()
        core_cur.close()
        core_conn.close()


def test_community_announcements(authenticated_user_page):
    page, tracker, current_user = authenticated_user_page

    # Navigate to community page
    page.goto(f"{BASE_URL}/community")
    page.wait_for_load_state("networkidle")

    # Click the "Announcements" tab
    announcements_tab = page.locator(
        "button.community-tab-btn", has_text=re.compile(r"Announcements", re.IGNORECASE)
    )
    expect(announcements_tab).to_be_visible(timeout=10000)
    announcements_tab.click()
    page.wait_for_timeout(500)

    # Verify the announcements panel is visible
    announcements_panel = page.locator("#community-announcements-tab")
    expect(announcements_panel).to_be_visible(timeout=5000)

    tracker.assert_no_critical_errors()


def test_community_dynamic_tabs_load_without_console_errors(authenticated_user_page):
    page, tracker, current_user = authenticated_user_page

    page.goto(f"{BASE_URL}/community")
    page.wait_for_load_state("networkidle")

    tab_expectations = [
        ("My Circle", "#community-circle-tab"),
        ("Expert AMAs", "#community-ama-tab"),
        ("Challenges", "#community-challenges-tab"),
    ]

    for label, selector in tab_expectations:
        tab = page.locator("button.community-tab-btn", has_text=re.compile(label, re.IGNORECASE))
        expect(tab).to_be_visible(timeout=10000)
        tab.click()
        expect(page.locator(selector)).to_be_visible(timeout=10000)
        page.wait_for_timeout(500)

    expect(page.locator("#ama-loading")).not_to_be_visible(timeout=10000)
    expect(page.locator("#community-challenges-tab")).to_be_visible(timeout=5000)

    tracker.assert_no_critical_errors()


def test_community_invalid_partial_returns_404(authenticated_user_page):
    page, tracker, current_user = authenticated_user_page

    response = page.request.get(f"{BASE_URL}/community/partials/not-a-tab")
    assert response.status == 404


def test_community_partial_requires_auth(page):
    response = page.request.get(
        f"{BASE_URL}/community/partials/challenges",
        max_redirects=0,
    )

    assert response.status in (302, 303, 307, 308)
    assert response.headers["location"].endswith("/auth/login")


def test_circle_settings_modal_keyboard_and_mobile(authenticated_user_page):
    page, tracker, current_user = authenticated_user_page
    page.set_viewport_size({"width": 390, "height": 844})

    def fulfill_json(route, payload, status=200):
        route.fulfill(
            status=status,
            content_type="application/json",
            body=json.dumps(payload),
        )

    circle_payload = {
        "circle": {
            "id": "circle-1",
            "name": "E2E Circle",
            "description": "Seeded circle for modal coverage",
            "avatar_emoji": "G",
            "member_count": 1,
            "total_xp": 1500,
            "level": 2,
            "level_name": "Sprout",
            "owner_id": current_user["user_id"],
            "is_public": True,
        },
        "members": [
            {
                "user_id": current_user["user_id"],
                "role": "owner",
                "joined_at": "2026-04-01T00:00:00Z",
            }
        ],
    }

    page.route("**/api/community/xp", lambda route: fulfill_json(route, {
        "level_icon": "G",
        "level_name": "Sprout",
        "level": 2,
        "xp_total": 1500,
        "progress_pct": 40,
        "xp_to_next": 500,
        "login_streak": 3,
    }))
    page.route("**/api/community/xp/history?page=1", lambda route: fulfill_json(route, {"entries": []}))
    page.route("**/api/community/circles/me", lambda route: fulfill_json(route, circle_payload))
    page.route("**/api/community/circles/requests/mine", lambda route: fulfill_json(route, {"requests": []}))
    page.route("**/api/community/circles/leaderboard", lambda route: fulfill_json(route, {
        "circles": [
            {
                "id": "circle-1",
                "name": "E2E Circle",
                "avatar_emoji": "G",
                "is_public": True,
                "member_count": 1,
                "level": 2,
                "total_xp": 1500,
            }
        ]
    }))
    page.route("**/api/community/invites", lambda route: fulfill_json(route, {"invites": []}))
    page.route("**/api/community/circles/circle-1/requests", lambda route: fulfill_json(route, {"requests": []}))

    page.goto(f"{BASE_URL}/community")
    page.wait_for_load_state("networkidle")

    circle_tab = page.locator(
        "button.community-tab-btn", has_text=re.compile(r"My Circle", re.IGNORECASE)
    )
    # The mobile topbar (WS1.5) puts tabs in a horizontally scrolling row;
    # the "My Circle" tab can be off-screen at 390px width. Scroll it in,
    # then click programmatically — the mobile-header overlay intercepts
    # pointer events on the tab's actual coords.
    circle_tab.scroll_into_view_if_needed()
    expect(circle_tab).to_be_visible(timeout=10000)
    circle_tab.evaluate("el => el.click()")

    settings_button = page.locator("button", has_text=re.compile(r"Settings", re.IGNORECASE)).first
    expect(settings_button).to_be_visible(timeout=10000)
    settings_button.click()

    modal = page.locator("#circle-settings-modal")
    expect(modal).to_be_visible(timeout=5000)
    expect(modal).to_have_attribute("role", "dialog")
    expect(modal).to_have_attribute("aria-modal", "true")
    expect(modal).to_have_attribute("aria-hidden", "false")
    expect(page.locator("button[aria-label='Close circle settings']")).to_be_focused()

    page.keyboard.press("Shift+Tab")
    expect(page.locator("#settings-save-btn")).to_be_focused()

    panel_box = modal.locator(".ds-card").bounding_box()
    assert panel_box
    assert panel_box["x"] >= 0
    assert panel_box["y"] >= 0
    assert panel_box["x"] + panel_box["width"] <= 390
    assert panel_box["y"] + panel_box["height"] <= 844

    page.keyboard.press("Escape")
    expect(modal).not_to_be_visible(timeout=3000)
    expect(settings_button).to_be_focused()

    tracker.assert_no_critical_errors()


def _mark_user_community_banned(user_id, reason="Repeat policy violations."):
    """14.8.1 helper — upserts a community_profiles row with the user marked banned."""
    conn = get_community_db_connection()
    try:
        cur = conn.cursor()
        cur.execute(
            """
            INSERT INTO community_profiles (user_id, is_community_banned, ban_reason)
            VALUES (%s, TRUE, %s)
            ON CONFLICT (user_id) DO UPDATE SET
                is_community_banned = TRUE,
                ban_reason = EXCLUDED.ban_reason
            """,
            (str(user_id), reason),
        )
        # Wipe any prior appeals for a clean slate.
        cur.execute("DELETE FROM ban_appeals WHERE user_id = %s", (str(user_id),))
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _fetch_ban_appeals(user_id):
    conn = get_community_db_connection()
    try:
        cur = conn.cursor()
        cur.execute(
            "SELECT status, appeal_text FROM ban_appeals WHERE user_id = %s",
            (str(user_id),),
        )
        rows = cur.fetchall()
        return rows
    finally:
        cur.close()
        conn.close()


def test_community_ban_appeal_banner_and_submission(authenticated_user_page):
    """14.8.1 — banned user sees banner, can submit appeal, state flips to pending.

    Covers: backend ban-state exposure on /api/community/profile/me, banner
    render branch, submit POST /api/community/appeals, post-submit UI swap.
    """
    page, tracker, current_user = authenticated_user_page
    user_id = current_user["user_id"]

    _mark_user_community_banned(user_id)

    page.goto(f"{BASE_URL}/community")
    page.wait_for_load_state("networkidle")

    banner = page.locator("#community-ban-banner")
    expect(banner).to_be_visible(timeout=5000)
    expect(banner.locator("#community-ban-banner-reason")).to_contain_text(
        "Repeat policy violations."
    )

    submit_btn = page.locator("#community-ban-appeal-btn")
    pending_pill = page.locator("#community-ban-pending-state")
    expect(submit_btn).to_be_visible()
    expect(pending_pill).not_to_be_visible()

    submit_btn.click()
    modal = page.locator("#ban-appeal-modal")
    expect(modal).to_be_visible(timeout=3000)
    expect(modal).to_have_attribute("role", "dialog")

    textarea = page.locator("#ban-appeal-text")
    textarea.fill(
        "I understand the policy violation and am requesting a careful review. "
        "I will not repeat the behaviour."
    )

    page.locator("#ban-appeal-submit-btn").click()
    expect(modal).not_to_be_visible(timeout=5000)
    expect(pending_pill).to_be_visible(timeout=5000)
    expect(submit_btn).not_to_be_visible()

    appeals = _fetch_ban_appeals(user_id)
    assert len(appeals) == 1, f"expected exactly one appeal row, got {appeals!r}"
    assert appeals[0][0] == "pending"
    assert "policy violation" in appeals[0][1].lower()

    tracker.assert_no_critical_errors()


def test_community_ban_appeal_banner_hidden_for_unbanned_user(authenticated_user_page):
    """14.8.1 — banner stays hidden for the happy path so non-banned users
    never see the warning."""
    page, tracker, current_user = authenticated_user_page

    page.goto(f"{BASE_URL}/community")
    page.wait_for_load_state("networkidle")

    banner = page.locator("#community-ban-banner")
    expect(banner).to_have_count(1)
    expect(banner).not_to_be_visible()

    tracker.assert_no_critical_errors()


# ─── 14.8.2 — block / mute self-service ──────────────────────────────────


def _seed_community_post(author_user_id, content):
    """14.8.2 helper — inserts a post directly into the community DB so block
    filtering can be asserted against a known author."""
    conn = get_community_db_connection()
    try:
        cur = conn.cursor()
        # Ensure a community_profiles row exists so the feed query JOIN doesn't
        # drop the post.
        cur.execute(
            """
            INSERT INTO community_profiles (user_id) VALUES (%s)
            ON CONFLICT (user_id) DO NOTHING
            """,
            (str(author_user_id),),
        )
        cur.execute(
            """
            INSERT INTO posts (user_id, content, post_type)
            VALUES (%s, %s, 'general')
            RETURNING id
            """,
            (str(author_user_id), content),
        )
        post_id = cur.fetchone()[0]
        conn.commit()
        return post_id
    finally:
        cur.close()
        conn.close()


def _make_e2e_user_via_db():
    """Reuse the e2e user creation helper from conftest without going through
    a browser context — useful when we just need a second user id to seed."""
    from tests.e2e.conftest import create_e2e_user

    return create_e2e_user(email_prefix="e2e-target", display_name="Block Target")


def _csrf_headers(page):
    """Read csrf_token cookie from page context and return matching header dict."""
    cookies = page.context.cookies(BASE_URL)
    token = next((c["value"] for c in cookies if c["name"] == "csrf_token"), None)
    return {"X-CSRF-Token": token} if token else {}


def _block_via_api(page, target_user_id):
    """Issues POST /api/community/users/:id/block from the page's auth context."""
    return page.request.post(
        f"{BASE_URL}/api/community/users/{target_user_id}/block",
        headers={**_csrf_headers(page), "Content-Type": "application/json"},
    )


def test_community_block_unblock_and_feed_hides_blocked_author(authenticated_user_page):
    """14.8.2 — actor blocks target, target's post vanishes from actor's
    feed; unblock restores it."""
    page, tracker, current_user = authenticated_user_page
    actor_id = current_user["user_id"]
    target = _make_e2e_user_via_db()
    target_id = target["user_id"]

    needle = f"block-fixture-{uuid.uuid4().hex}"
    post_id = _seed_community_post(target_id, needle)

    try:
        # Confirm feed shows the target's post before any block.
        page.goto(f"{BASE_URL}/community")
        page.wait_for_load_state("networkidle")
        page.wait_for_selector("#community-feed-container .feed-post", timeout=10000)
        feed_text_before = page.locator("#community-feed-container").inner_text()
        assert needle in feed_text_before, "expected target's post in actor feed before block"

        # Block via API call (uses page context so cookies + CSRF apply).
        res = _block_via_api(page, target_id)
        assert res.status == 200, f"block POST returned {res.status}: {res.text()}"
        payload = res.json()
        assert payload.get("blocked") is True

        # List endpoint surfaces the target.
        list_res = page.request.get(f"{BASE_URL}/api/community/blocks")
        assert list_res.status == 200
        blocks = list_res.json()["blocks"]
        assert any(b["target_user_id"] == target_id for b in blocks)

        # Reload feed; target's post should be filtered out.
        page.reload()
        page.wait_for_load_state("networkidle")
        feed_after = page.locator("#community-feed-container").inner_text()
        assert needle not in feed_after, "blocked author's post still visible in feed"

        # Self-block rejected with 400.
        self_block = page.request.post(
            f"{BASE_URL}/api/community/users/{actor_id}/block",
            headers=_csrf_headers(page),
        )
        assert self_block.status == 400

        # Unblock restores visibility.
        unblock = page.request.delete(
            f"{BASE_URL}/api/community/users/{target_id}/block",
            headers=_csrf_headers(page),
        )
        assert unblock.status == 200
        page.reload()
        page.wait_for_load_state("networkidle")
        page.wait_for_selector("#community-feed-container .feed-post", timeout=10000)
        feed_restored = page.locator("#community-feed-container").inner_text()
        assert needle in feed_restored, "post did not reappear after unblock"

        tracker.assert_no_critical_errors()
    finally:
        conn = get_community_db_connection()
        try:
            cur = conn.cursor()
            cur.execute("DELETE FROM posts WHERE id = %s", (post_id,))
            cur.execute(
                "DELETE FROM block_relationships WHERE actor_user_id = %s",
                (str(actor_id),),
            )
            conn.commit()
        finally:
            cur.close()
            conn.close()


def test_community_own_comment_edit_updates_db_and_shows_edited(authenticated_user_page):
    """14.8.5 — owner edits their comment via PUT /api/community/comments/:id;
    the row in DB gets edited_at + content; original_content captured on first
    edit only."""
    page, tracker, current_user = authenticated_user_page
    actor_id = current_user["user_id"]

    page.goto(f"{BASE_URL}/community")
    page.wait_for_load_state("networkidle")

    post_id = _seed_community_post(actor_id, f"edit-fixture-post-{uuid.uuid4().hex}")
    conn = get_community_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO comments (post_id, user_id, content)
            VALUES (%s, %s, %s)
            RETURNING id
            """,
            (post_id, str(actor_id), "Original comment body."),
        )
        comment_id = cur.fetchone()[0]
        conn.commit()
    finally:
        cur.close()
        conn.close()

    try:
        # Edit via API.
        res = page.request.put(
            f"{BASE_URL}/api/community/comments/{comment_id}",
            headers={**_csrf_headers(page), "Content-Type": "application/json"},
            data='{"content": "Edited comment body after review."}',
        )
        assert res.status == 200, f"PUT returned {res.status}: {res.text()}"

        # Verify DB row updated.
        conn = get_community_db_connection()
        cur = conn.cursor()
        try:
            cur.execute(
                "SELECT content, original_content, edited_at FROM comments WHERE id = %s",
                (str(comment_id),),
            )
            row = cur.fetchone()
            assert row is not None
            assert row[0] == "Edited comment body after review."
            assert row[1] == "Original comment body."
            assert row[2] is not None, "edited_at should be set after edit"
        finally:
            cur.close()
            conn.close()

        # Editing someone else's comment is rejected with 403.
        other = _make_e2e_user_via_db()
        conn = get_community_db_connection()
        cur = conn.cursor()
        try:
            cur.execute(
                """
                INSERT INTO comments (post_id, user_id, content)
                VALUES (%s, %s, %s)
                RETURNING id
                """,
                (post_id, str(other["user_id"]), "Other user comment."),
            )
            other_comment_id = cur.fetchone()[0]
            conn.commit()
        finally:
            cur.close()
            conn.close()

        forbidden = page.request.put(
            f"{BASE_URL}/api/community/comments/{other_comment_id}",
            headers={**_csrf_headers(page), "Content-Type": "application/json"},
            data='{"content": "Hijack attempt"}',
        )
        assert forbidden.status == 403, f"expected 403, got {forbidden.status}"

        tracker.assert_no_critical_errors()
    finally:
        conn = get_community_db_connection()
        try:
            cur = conn.cursor()
            cur.execute("DELETE FROM comments WHERE post_id = %s", (post_id,))
            cur.execute("DELETE FROM posts WHERE id = %s", (post_id,))
            conn.commit()
        finally:
            cur.close()
            conn.close()


def test_community_comment_reaction_toggle(authenticated_user_page):
    """14.8.6 — reacting to a comment toggles a comment_reactions row and
    bumps comments.reaction_count via trigger."""
    page, tracker, current_user = authenticated_user_page
    actor_id = current_user["user_id"]

    page.goto(f"{BASE_URL}/community")
    page.wait_for_load_state("networkidle")

    post_id = _seed_community_post(actor_id, f"comment-reaction-fixture-{uuid.uuid4().hex}")
    conn = get_community_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO comments (post_id, user_id, content)
            VALUES (%s, %s, %s)
            RETURNING id
            """,
            (post_id, str(actor_id), "Reactable comment."),
        )
        comment_id = cur.fetchone()[0]
        conn.commit()
    finally:
        cur.close()
        conn.close()

    try:
        # First POST adds the reaction.
        first = page.request.post(
            f"{BASE_URL}/api/community/comments/{comment_id}/reactions",
            headers={**_csrf_headers(page), "Content-Type": "application/json"},
            data='{"reaction_type": "fire"}',
        )
        assert first.status == 200, f"first reaction: {first.status}: {first.text()}"
        body = first.json()
        assert body["added"] is True
        assert body["reaction_count"] == 1

        # Same POST toggles it off.
        second = page.request.post(
            f"{BASE_URL}/api/community/comments/{comment_id}/reactions",
            headers={**_csrf_headers(page), "Content-Type": "application/json"},
            data='{"reaction_type": "fire"}',
        )
        assert second.status == 200
        body2 = second.json()
        assert body2["added"] is False
        assert body2["reaction_count"] == 0

        # Invalid reaction type is rejected.
        bad = page.request.post(
            f"{BASE_URL}/api/community/comments/{comment_id}/reactions",
            headers={**_csrf_headers(page), "Content-Type": "application/json"},
            data='{"reaction_type": "like"}',
        )
        assert bad.status == 400

        tracker.assert_no_critical_errors()
    finally:
        conn = get_community_db_connection()
        try:
            cur = conn.cursor()
            cur.execute("DELETE FROM comments WHERE post_id = %s", (post_id,))
            cur.execute("DELETE FROM posts WHERE id = %s", (post_id,))
            conn.commit()
        finally:
            cur.close()
            conn.close()


def test_community_mute_filters_feed(authenticated_user_page):
    """14.8.2 — muting a user removes their posts from the actor's feed
    (one-directional)."""
    page, tracker, current_user = authenticated_user_page
    target = _make_e2e_user_via_db()
    target_id = target["user_id"]
    needle = f"mute-fixture-{uuid.uuid4().hex}"
    post_id = _seed_community_post(target_id, needle)

    try:
        # Visit /community first so the csrf_token cookie is set.
        page.goto(f"{BASE_URL}/community")
        page.wait_for_load_state("networkidle")
        # Mute via API.
        res = page.request.post(
            f"{BASE_URL}/api/community/users/{target_id}/mute",
            headers=_csrf_headers(page),
        )
        assert res.status == 200, f"mute POST returned {res.status}: {res.text()}"

        page.goto(f"{BASE_URL}/community")
        page.wait_for_load_state("networkidle")
        feed_after = page.locator("#community-feed-container").inner_text()
        assert needle not in feed_after, "muted author's post still visible"

        # Unmute restores visibility.
        page.request.delete(
            f"{BASE_URL}/api/community/users/{target_id}/mute",
            headers=_csrf_headers(page),
        )
        page.reload()
        page.wait_for_load_state("networkidle")
        page.wait_for_selector("#community-feed-container .feed-post", timeout=10000)
        feed_restored = page.locator("#community-feed-container").inner_text()
        assert needle in feed_restored, "post did not reappear after unmute"

        tracker.assert_no_critical_errors()
    finally:
        conn = get_community_db_connection()
        try:
            cur = conn.cursor()
            cur.execute("DELETE FROM posts WHERE id = %s", (post_id,))
            cur.execute(
                "DELETE FROM mute_relationships WHERE actor_user_id = %s",
                (str(current_user["user_id"]),),
            )
            conn.commit()
        finally:
            cur.close()
            conn.close()
