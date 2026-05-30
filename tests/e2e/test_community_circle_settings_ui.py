"""
Wave B — Circle Settings Subpage UI tests.

Drives the browser against /community/circle/:slug/settings.

The page is owner-only; we mint a fresh user, seed a circle as that user
(so the user is the owner), and then verify the settings page:
  1. Page loads — back link + header card render
  2. Basic-info inputs populated from the seeded circle
  3. Privacy toggle reflects is_public=TRUE
  4. Members card renders the owner row
  5. Save button starts disabled; editing a field enables it

Run:
    pytest tests/e2e/test_community_circle_settings_ui.py -v
"""

import os
import uuid
from pathlib import Path

import pytest
import psycopg2
from playwright.sync_api import expect

from community_helpers import (
    BASE_URL,
    COMMUNITY_DB_URL,
    mint_user,
    make_context,
    cleanup_user,
    seed_circle,
)


# ─── Fixtures ──────────────────────────────────────────────────────────

@pytest.fixture(scope="function")
def circle_owner():
    """Mint a user, seed a circle they own. Yields (user, circle)."""
    user = mint_user(prefix="e2e-ccs-owner", display_name="CCS Owner")
    circle = seed_circle(user["user_id"], name=f"CCS Test {uuid.uuid4().hex[:6]}")
    yield user, circle
    cleanup_user(user["user_id"])


# ─── Helpers ───────────────────────────────────────────────────────────

def _open_settings(playwright_session, user, slug):
    ctx, page, errors = make_context(playwright_session, user)
    page.goto(
        f"{BASE_URL}/community/circle/{slug}/settings",
        wait_until="domcontentloaded",
        timeout=15000,
    )
    expect(page.locator("#ccs-root")).to_be_visible(timeout=10000)
    return ctx, page, errors


def _wait_for_hydration(page, slug):
    """Wait until the JS replaces the '—' placeholder in the header."""
    page.wait_for_function(
        "() => { const n = document.getElementById('ccs-name');"
        "  return n && n.textContent && n.textContent.trim() !== '—'; }",
        timeout=10000,
    )


def _seed_resource_review_fixture(circle_id, owner_id):
    """Seed a Circle resource with a pending current version for review."""
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        suffix = uuid.uuid4().hex[:8]
        title = f"E2E Resource Review {suffix}"
        current_url = f"https://example.com/resources/{suffix}/current.pdf"
        previous_url = f"https://example.com/resources/{suffix}/previous.pdf"
        cur.execute(
            """
            INSERT INTO circle_resources (
                circle_id,
                title,
                description,
                resource_type,
                access_scope,
                url,
                is_official,
                created_by,
                version_label,
                requires_download
            )
            VALUES (%s, %s, %s, 'official_document', 'member', %s, TRUE, %s, 'v2', FALSE)
            RETURNING id
            """,
            (
                circle_id,
                title,
                "Browser fixture for resource version review.",
                current_url,
                owner_id,
            ),
        )
        resource_id = str(cur.fetchone()[0])
        cur.execute(
            """
            INSERT INTO circle_resource_versions (
                resource_id,
                circle_id,
                version_label,
                url,
                requires_download,
                change_note,
                is_current,
                created_by,
                review_status
            )
            VALUES (%s, %s, 'v1', %s, FALSE, 'Previous browser fixture', FALSE, %s, 'superseded')
            """,
            (resource_id, circle_id, previous_url, owner_id),
        )
        cur.execute(
            """
            INSERT INTO circle_resource_versions (
                resource_id,
                circle_id,
                version_label,
                url,
                requires_download,
                change_note,
                is_current,
                created_by,
                review_status
            )
            VALUES (%s, %s, 'v2', %s, FALSE, 'Pending browser review', TRUE, %s, 'pending')
            RETURNING id
            """,
            (resource_id, circle_id, current_url, owner_id),
        )
        version_id = str(cur.fetchone()[0])
        conn.commit()
        return {
            "resource_id": resource_id,
            "version_id": version_id,
            "title": title,
            "current_url": current_url,
            "previous_url": previous_url,
        }
    finally:
        conn.close()


def _seed_ops_alert_fixture(circle_id):
    """Seed an active ops alert for Circle manager workflow actions."""
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        summary = f"E2E Workflow Alert {uuid.uuid4().hex[:8]}"
        cur.execute(
            """
            INSERT INTO circle_ops_alerts (
                circle_id,
                alert_type,
                severity,
                status,
                summary,
                details
            )
            VALUES (%s, 'moderation_sla', 'warning', 'open', %s, '{}'::jsonb)
            RETURNING id
            """,
            (circle_id, summary),
        )
        alert_id = str(cur.fetchone()[0])
        conn.commit()
        return {"alert_id": alert_id, "summary": summary}
    finally:
        conn.close()


def _seed_private_gcs_resource_fixture(circle_id, owner_id, fake_root, bucket):
    """Seed a private-GCS-backed Circle resource and create its fake object bytes."""
    object_path = f"community/circles/{circle_id}/resources/{uuid.uuid4().hex}.pdf"
    file_bytes = (
        b"%PDF-1.4\n"
        b"1 0 obj\n<< /Type /Catalog >>\nendobj\n"
        b"trailer\n<< /Root 1 0 R >>\n%%EOF\n"
    )
    target_path = Path(fake_root) / bucket / object_path
    target_path.parent.mkdir(parents=True, exist_ok=True)
    target_path.write_bytes(file_bytes)

    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        suffix = uuid.uuid4().hex[:8]
        title = f"E2E Private GCS Resource {suffix}"
        storage_path = f"gs://{bucket}/{object_path}"
        file_name = f"private-circle-report-{suffix}.pdf"
        sha256_hex = "0" * 64
        cur.execute(
            """
            INSERT INTO circle_resources (
                circle_id,
                title,
                description,
                resource_type,
                access_scope,
                storage_object_path,
                is_official,
                created_by,
                file_name,
                mime_type,
                file_size_bytes,
                sha256_hex,
                version_label,
                requires_download,
                upload_status
            )
            VALUES (
                %s, %s, 'Private GCS stream E2E fixture', 'official_document',
                'member', %s, TRUE, %s, %s, 'application/pdf', %s, %s, 'gcs-v1', TRUE, 'uploaded'
            )
            RETURNING id
            """,
            (
                circle_id,
                title,
                storage_path,
                owner_id,
                file_name,
                len(file_bytes),
                sha256_hex,
            ),
        )
        resource_id = str(cur.fetchone()[0])
        cur.execute(
            """
            INSERT INTO circle_resource_versions (
                resource_id,
                circle_id,
                version_label,
                storage_object_path,
                file_name,
                mime_type,
                file_size_bytes,
                sha256_hex,
                requires_download,
                upload_status,
                change_note,
                is_current,
                created_by
            )
            VALUES (%s, %s, 'gcs-v1', %s, %s, 'application/pdf', %s, %s, TRUE, 'uploaded',
                    'Private GCS stream E2E fixture', TRUE, %s)
            """,
            (
                resource_id,
                circle_id,
                storage_path,
                file_name,
                len(file_bytes),
                sha256_hex,
                owner_id,
            ),
        )
        conn.commit()
        return {
            "resource_id": resource_id,
            "title": title,
            "file_name": file_name,
            "storage_path": storage_path,
            "object_path": object_path,
            "bytes": file_bytes,
        }
    finally:
        conn.close()


def _set_circle_private(circle_id):
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        cur.execute(
            """
            UPDATE circles
               SET is_public = FALSE,
                   visibility = 'private',
                   join_policy = 'request',
                   join_approval_required = TRUE,
                   updated_at = NOW()
             WHERE id = %s
            """,
            (circle_id,),
        )
        conn.commit()
    finally:
        conn.close()


def _add_circle_member(circle_id, user_id, role):
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        cur.execute(
            """
            INSERT INTO circle_members (circle_id, user_id, role)
            VALUES (%s, %s, %s)
            ON CONFLICT (circle_id, user_id)
            DO UPDATE SET role = EXCLUDED.role, joined_at = NOW()
            """,
            (circle_id, user_id, role),
        )
        cur.execute(
            """
            UPDATE circles
               SET member_count = (
                   SELECT COUNT(*)::INT FROM circle_members WHERE circle_id = %s
               )
             WHERE id = %s
            """,
            (circle_id, circle_id),
        )
        conn.commit()
    finally:
        conn.close()


def _fetch_resource_version_review(version_id):
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        cur.execute(
            """
            SELECT review_status, review_note
              FROM circle_resource_versions
             WHERE id = %s
            """,
            (version_id,),
        )
        return cur.fetchone()
    finally:
        conn.close()


def _fetch_resource_upload_state(resource_id):
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        cur.execute(
            """
            SELECT title, version_label, upload_status, mime_type, file_size_bytes, sha256_hex
              FROM circle_resources
             WHERE id = %s
            """,
            (resource_id,),
        )
        return cur.fetchone()
    finally:
        conn.close()


def _fetch_resource_current_state(resource_id):
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        cur.execute(
            """
            SELECT r.version_label,
                   r.url,
                   ARRAY_AGG(v.version_label ORDER BY v.version_label) FILTER (WHERE v.is_current)
              FROM circle_resources r
              JOIN circle_resource_versions v ON v.resource_id = r.id
             WHERE r.id = %s
             GROUP BY r.id, r.version_label, r.url
            """,
            (resource_id,),
        )
        return cur.fetchone()
    finally:
        conn.close()


def _fetch_ops_alert_workflow(alert_id):
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        cur.execute(
            """
            SELECT status, workflow_state, workflow_note
              FROM circle_ops_alerts
             WHERE id = %s
            """,
            (alert_id,),
        )
        return cur.fetchone()
    finally:
        conn.close()


def _circle_exists(circle_id):
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        cur.execute("SELECT EXISTS(SELECT 1 FROM circles WHERE id = %s)", (circle_id,))
        return bool(cur.fetchone()[0])
    finally:
        conn.close()


def _csrf_header(context):
    token = next(
        (cookie["value"] for cookie in context.cookies() if cookie["name"] == "csrf_token"),
        "",
    )
    return {"X-CSRF-Token": token}


def _seed_circle_delete_dependency_fixture(circle_id, owner_id, member_id, requester_id):
    """Create representative Circle-owned rows that must disappear with the Circle."""
    resource = _seed_resource_review_fixture(circle_id, owner_id)
    alert = _seed_ops_alert_fixture(circle_id)
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        cur.execute(
            """
            INSERT INTO circle_join_requests (circle_id, user_id, status)
            VALUES (%s, %s, 'pending')
            ON CONFLICT DO NOTHING
            """,
            (circle_id, requester_id),
        )
        cur.execute(
            """
            INSERT INTO circle_bans (circle_id, banned_user_id, banned_by, reason)
            VALUES (%s, %s, %s, 'Delete cascade fixture')
            ON CONFLICT (circle_id, banned_user_id) DO NOTHING
            """,
            (circle_id, requester_id, owner_id),
        )
        cur.execute(
            """
            INSERT INTO posts (user_id, post_type, content, image_urls, circle_id)
            VALUES (%s, 'announcement', 'Circle delete cascade fixture post', %s, %s)
            RETURNING id
            """,
            (
                owner_id,
                ["https://example.com/circle-delete-fixture.png"],
                circle_id,
            ),
        )
        post_id = str(cur.fetchone()[0])
        cur.execute(
            """
            INSERT INTO content_reports (post_id, reporter_id, reason)
            VALUES (%s, %s, 'Circle delete cascade fixture report')
            """,
            (post_id, requester_id),
        )
        cur.execute(
            """
            INSERT INTO circle_ops_alert_notifications (
                alert_id,
                trigger_action,
                target_user_id,
                payload
            )
            VALUES (%s, 'auto_critical', %s, '{"fixture": true}'::jsonb)
            """,
            (alert["alert_id"], owner_id),
        )
        cur.execute(
            """
            INSERT INTO amas (
                title,
                description,
                expert_name,
                expert_title,
                scheduled_at,
                status,
                created_by,
                circle_id,
                rsvp_enabled
            )
            VALUES (
                'Circle delete cascade fixture AMA',
                'Fixture scoped to deleted Circle',
                'Workflow Expert',
                'Fixture',
                NOW() + INTERVAL '1 day',
                'scheduled',
                %s,
                %s,
                TRUE
            )
            RETURNING id
            """,
            (owner_id, circle_id),
        )
        ama_id = str(cur.fetchone()[0])
        cur.execute(
            """
            INSERT INTO circle_event_rsvps (ama_id, circle_id, user_id, status)
            VALUES (%s, %s, %s, 'going')
            """,
            (ama_id, circle_id, member_id),
        )
        cur.execute(
            """
            INSERT INTO challenges (
                title,
                description,
                xp_reward,
                requirement_type,
                requirement_value,
                frequency,
                circle_id,
                challenge_scope
            )
            VALUES (
                'Circle delete cascade fixture challenge',
                'Fixture scoped to deleted Circle',
                5,
                'circle_comment',
                1,
                'one_time',
                %s,
                'circle'
            )
            RETURNING id
            """,
            (circle_id,),
        )
        challenge_id = str(cur.fetchone()[0])
        cur.execute(
            """
            INSERT INTO circle_challenge_progress (
                circle_id,
                user_id,
                challenge_id,
                current_value
            )
            VALUES (%s, %s, %s, 1)
            """,
            (circle_id, member_id, challenge_id),
        )
        cur.execute(
            """
            INSERT INTO circle_onboarding_progress (
                circle_id,
                user_id,
                rules_read,
                introduced_self
            )
            VALUES (%s, %s, TRUE, TRUE)
            """,
            (circle_id, member_id),
        )
        cur.execute(
            """
            INSERT INTO circle_daily_analytics (
                circle_id,
                snapshot_date,
                member_count,
                active_members,
                posts_count,
                comments_count,
                reported_content_count
            )
            VALUES (%s, CURRENT_DATE, 2, 2, 1, 0, 1)
            ON CONFLICT (circle_id, snapshot_date) DO NOTHING
            """,
            (circle_id,),
        )
        conn.commit()
        return {
            "post_id": post_id,
            "resource_id": resource["resource_id"],
            "alert_id": alert["alert_id"],
            "ama_id": ama_id,
            "challenge_id": challenge_id,
        }
    finally:
        conn.close()


def _count_circle_delete_dependencies(circle_id, fixture):
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        checks = {
            "circles": ("SELECT COUNT(*) FROM circles WHERE id = %s", (circle_id,)),
            "circle_members": ("SELECT COUNT(*) FROM circle_members WHERE circle_id = %s", (circle_id,)),
            "circle_join_requests": (
                "SELECT COUNT(*) FROM circle_join_requests WHERE circle_id = %s",
                (circle_id,),
            ),
            "circle_bans": ("SELECT COUNT(*) FROM circle_bans WHERE circle_id = %s", (circle_id,)),
            "posts": ("SELECT COUNT(*) FROM posts WHERE circle_id = %s", (circle_id,)),
            "content_reports": (
                "SELECT COUNT(*) FROM content_reports WHERE post_id = %s",
                (fixture["post_id"],),
            ),
            "circle_resources": (
                "SELECT COUNT(*) FROM circle_resources WHERE circle_id = %s",
                (circle_id,),
            ),
            "circle_resource_versions": (
                "SELECT COUNT(*) FROM circle_resource_versions WHERE circle_id = %s",
                (circle_id,),
            ),
            "circle_ops_alerts": (
                "SELECT COUNT(*) FROM circle_ops_alerts WHERE circle_id = %s",
                (circle_id,),
            ),
            "circle_ops_alert_notifications": (
                "SELECT COUNT(*) FROM circle_ops_alert_notifications WHERE alert_id = %s",
                (fixture["alert_id"],),
            ),
            "amas": ("SELECT COUNT(*) FROM amas WHERE circle_id = %s", (circle_id,)),
            "circle_event_rsvps": (
                "SELECT COUNT(*) FROM circle_event_rsvps WHERE circle_id = %s",
                (circle_id,),
            ),
            "challenges": ("SELECT COUNT(*) FROM challenges WHERE circle_id = %s", (circle_id,)),
            "circle_challenge_progress": (
                "SELECT COUNT(*) FROM circle_challenge_progress WHERE circle_id = %s",
                (circle_id,),
            ),
            "circle_onboarding_progress": (
                "SELECT COUNT(*) FROM circle_onboarding_progress WHERE circle_id = %s",
                (circle_id,),
            ),
            "circle_daily_analytics": (
                "SELECT COUNT(*) FROM circle_daily_analytics WHERE circle_id = %s",
                (circle_id,),
            ),
        }
        counts = {}
        for name, (sql, params) in checks.items():
            cur.execute(sql, params)
            counts[name] = int(cur.fetchone()[0])
        return counts
    finally:
        conn.close()


# ─── Tests ─────────────────────────────────────────────────────────────

@pytest.mark.community
def test_circle_settings_page_loads_with_back_link(playwright_session, circle_owner):
    """Page renders with back-link + all cards present in DOM."""
    user, circle = circle_owner
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        expect(page.locator(".ccs-back")).to_be_visible()
        expect(page.locator(".ccs-back")).to_have_attribute("href", "/community/circles")

        # All major cards exist in DOM (even if some are hidden).
        for card_title in ["Basic Info", "Privacy", "Members"]:
            expect(page.get_by_role("heading", name=card_title)).to_be_visible()

        # Sticky footer exists.
        expect(page.locator("#ccs-footer")).to_be_attached()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_settings_header_populates_from_api(playwright_session, circle_owner):
    """After JS hydration, header shows the circle name + 1 member + role badge."""
    user, circle = circle_owner
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])

        expect(page.locator("#ccs-name")).to_have_text(circle["name"], timeout=5000)
        # The seeded circle starts with member_count = 1 (just the owner).
        expect(page.locator("#ccs-meta-members")).to_contain_text("member")
        # Owner badge.
        role = page.locator("#ccs-meta-role").text_content() or ""
        assert "owner" in role.lower(), f"Expected owner role, got '{role}'"

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_settings_basic_info_inputs_populated(playwright_session, circle_owner):
    """Name + slug input mirror the seeded values."""
    user, circle = circle_owner
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])

        expect(page.locator("#ccs-input-name")).to_have_value(circle["name"], timeout=5000)
        expect(page.locator("#ccs-input-slug")).to_have_value(circle["slug"])

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_settings_privacy_toggle_reflects_public_state(playwright_session, circle_owner):
    """seed_circle creates with is_public=TRUE → checkbox should be checked."""
    user, circle = circle_owner
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])

        # Give JS one tick to wire the checkbox.
        page.wait_for_timeout(300)
        is_checked = page.locator("#ccs-input-public").is_checked()
        assert is_checked, "Public toggle should be ON for a seeded public circle"

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_settings_save_btn_enables_after_edit(playwright_session, circle_owner):
    """Save button starts disabled; typing a new name enables it."""
    user, circle = circle_owner
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])

        save_btn = page.locator("#ccs-save-btn")
        # Disabled on pristine load.
        expect(save_btn).to_be_disabled()

        # Edit the name → save button should enable.
        page.fill("#ccs-input-name", circle["name"] + " edited")
        # The JS uses an input listener to flip data-state to dirty.
        page.wait_for_function(
            "() => !document.getElementById('ccs-save-btn').disabled",
            timeout=3000,
        )
        expect(save_btn).to_be_enabled()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_resource_file_upload_and_delivery_redirect_from_browser(
    playwright_session,
    circle_owner,
    tmp_path,
):
    """Owner uploads a PDF resource and the authenticated delivery endpoint is usable."""
    user, circle = circle_owner
    upload_path = tmp_path / "circle-report.pdf"
    upload_path.write_bytes(
        b"%PDF-1.4\n"
        b"1 0 obj\n<< /Type /Catalog >>\nendobj\n"
        b"trailer\n<< /Root 1 0 R >>\n%%EOF\n"
    )
    title = f"E2E Uploaded Resource {uuid.uuid4().hex[:8]}"
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])
        page.fill("#ccs-resource-title", title)
        page.fill("#ccs-resource-version-label", "upload-v1")
        page.set_input_files("#ccs-resource-file", str(upload_path))

        with page.expect_response(
            lambda response: (
                f"/api/community/circles/{circle['id']}/resources/upload" in response.url
                and response.request.method == "POST"
            ),
            timeout=20000,
        ) as response_info:
            page.locator("#ccs-resource-form").get_by_role("button", name="Add Resource").click()

        response = response_info.value
        assert response.ok
        payload = response.json()
        resource_id = payload["resource_id"]
        delivery_url = payload["delivery_url"]

        row = page.locator(".ccs-resource-row").filter(has_text=title)
        expect(row).to_be_visible(timeout=10000)
        expect(row).to_contain_text("Current version: upload-v1")
        expect(row.get_by_role("link", name="Open")).to_have_attribute("href", delivery_url)

        uploaded = _fetch_resource_upload_state(resource_id)
        assert uploaded is not None
        assert uploaded[0] == title
        assert uploaded[1] == "upload-v1"
        assert uploaded[2] == "uploaded"
        assert uploaded[3] == "application/pdf"
        assert uploaded[4] and uploaded[4] > 0
        assert uploaded[5] and len(uploaded[5]) == 64

        delivery = page.request.get(f"{BASE_URL}{delivery_url}", max_redirects=0)
        if delivery.status in (301, 302, 303, 307, 308):
            assert delivery.headers.get("location"), "Delivery redirect must include Location"
        else:
            assert delivery.status == 200
            assert "application/pdf" in delivery.headers.get("content-type", "")
            assert "no-store" in delivery.headers.get("cache-control", "")

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_resource_private_gcs_stream_requires_access_and_safe_headers(
    playwright_session,
    circle_owner,
):
    """Private GCS-backed resources stream through the app and stay access-controlled."""
    fake_root = os.environ.get("POOOL_GCS_DOWNLOAD_FAKE_ROOT")
    bucket = os.environ.get("GCS_BUCKET_NAME")
    if not fake_root or not bucket:
        pytest.skip("Private GCS stream E2E requires GCS_BUCKET_NAME and POOOL_GCS_DOWNLOAD_FAKE_ROOT")

    user, circle = circle_owner
    fixture = _seed_private_gcs_resource_fixture(
        circle["id"],
        user["user_id"],
        fake_root,
        bucket,
    )
    nonmember = mint_user(prefix="e2e-ccs-outsider", display_name="CCS Outsider")
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    outsider_ctx = None
    try:
        _wait_for_hydration(page, circle["slug"])
        delivery_url = f"/api/community/circles/{circle['id']}/resources/{fixture['resource_id']}/access"

        resources = page.request.get(f"{BASE_URL}/api/community/circles/{circle['id']}/resources")
        assert resources.ok
        resources_text = resources.text()
        assert fixture["title"] in resources_text
        assert fixture["storage_path"] not in resources_text
        assert fixture["object_path"] not in resources_text
        resources_payload = resources.json()
        row = next(
            item for item in resources_payload["resources"]
            if item["id"] == fixture["resource_id"]
        )
        assert row["delivery_mode"] == "api_stream"
        assert row["delivery_url"] == delivery_url
        assert row["has_private_file"] is True

        delivery = page.request.get(f"{BASE_URL}{delivery_url}", max_redirects=0)
        assert delivery.status == 200
        assert "application/pdf" in delivery.headers.get("content-type", "")
        assert "no-store" in delivery.headers.get("cache-control", "")
        assert delivery.headers.get("x-content-type-options") == "nosniff"
        disposition = delivery.headers.get("content-disposition", "")
        assert "attachment" in disposition
        assert fixture["file_name"] in disposition
        assert delivery.body() == fixture["bytes"]

        outsider_ctx, outsider_page, outsider_errors = make_context(playwright_session, nonmember)
        denied = outsider_page.request.get(f"{BASE_URL}{delivery_url}", max_redirects=0)
        assert denied.status in (403, 404)
        assert not outsider_errors, f"Outsider JS errors: {outsider_errors[:5]}"
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        if outsider_ctx:
            outsider_ctx.close()
        ctx.close()
        cleanup_user(nonmember["user_id"])


@pytest.mark.community
def test_circle_multi_user_manage_and_resource_access_matrix(
    playwright_session,
    circle_owner,
):
    """Owner/mod/member/outsider sessions hit the same private Circle gates."""
    owner, circle = circle_owner
    _set_circle_private(circle["id"])
    moderator = mint_user(prefix="e2e-ccs-mod", display_name="CCS Moderator")
    member = mint_user(prefix="e2e-ccs-member", display_name="CCS Member")
    outsider = mint_user(prefix="e2e-ccs-outsider", display_name="CCS Outsider")
    _add_circle_member(circle["id"], moderator["user_id"], "moderator")
    _add_circle_member(circle["id"], member["user_id"], "member")
    fixture = _seed_resource_review_fixture(circle["id"], owner["user_id"])

    contexts = []
    try:
        owner_ctx, owner_page, owner_errors = make_context(playwright_session, owner)
        mod_ctx, mod_page, mod_errors = make_context(playwright_session, moderator)
        member_ctx, member_page, member_errors = make_context(playwright_session, member)
        outsider_ctx, outsider_page, outsider_errors = make_context(playwright_session, outsider)
        contexts.extend([owner_ctx, mod_ctx, member_ctx, outsider_ctx])

        manage_url = f"{BASE_URL}/api/community/circles/{circle['id']}/manage"
        resource_admin_url = f"{BASE_URL}/api/community/circles/{circle['id']}/resources/manage"
        resources_url = f"{BASE_URL}/api/community/circles/{circle['id']}/resources"
        delivery_url = (
            f"{BASE_URL}/api/community/circles/{circle['id']}"
            f"/resources/{fixture['resource_id']}/access"
        )

        owner_manage = owner_page.request.get(manage_url)
        assert owner_manage.status == 200
        assert owner_manage.json()["role"] == "owner"

        moderator_manage = mod_page.request.get(manage_url)
        assert moderator_manage.status == 200
        assert moderator_manage.json()["role"] == "moderator"

        member_manage = member_page.request.get(manage_url)
        assert member_manage.status == 403

        outsider_manage = outsider_page.request.get(manage_url)
        assert outsider_manage.status == 403

        owner_resource_admin = owner_page.request.get(resource_admin_url)
        assert owner_resource_admin.status == 200

        moderator_resource_admin = mod_page.request.get(resource_admin_url)
        assert moderator_resource_admin.status == 403

        member_resources = member_page.request.get(resources_url)
        assert member_resources.status == 200
        assert fixture["title"] in member_resources.text()

        member_delivery = member_page.request.get(delivery_url, max_redirects=0)
        assert member_delivery.status in (301, 302, 303, 307, 308)
        assert member_delivery.headers.get("location") == fixture["current_url"]

        outsider_resources = outsider_page.request.get(resources_url)
        assert outsider_resources.status == 403

        outsider_delivery = outsider_page.request.get(delivery_url, max_redirects=0)
        assert outsider_delivery.status in (403, 404)

        for label, errors in [
            ("owner", owner_errors),
            ("moderator", mod_errors),
            ("member", member_errors),
            ("outsider", outsider_errors),
        ]:
            assert not errors, f"{label} JS errors: {errors[:5]}"
    finally:
        for ctx in contexts:
            ctx.close()
        cleanup_user(moderator["user_id"])
        cleanup_user(member["user_id"])
        cleanup_user(outsider["user_id"])


@pytest.mark.community
def test_circle_settings_non_owners_cannot_use_or_call_danger_zone(
    playwright_session,
    circle_owner,
):
    """Circle admin/mod/member can open settings but cannot see or call owner-only deletion."""
    owner, circle = circle_owner
    non_owner_roles = [
        ("admin", mint_user(prefix="e2e-ccs-admin", display_name="CCS Admin")),
        ("moderator", mint_user(prefix="e2e-ccs-mod", display_name="CCS Moderator")),
        ("member", mint_user(prefix="e2e-ccs-member", display_name="CCS Member")),
    ]
    contexts = []
    try:
        for role, user in non_owner_roles:
            _add_circle_member(circle["id"], user["user_id"], role)
            ctx, page, errors = make_context(playwright_session, user)
            contexts.append(ctx)
            page.goto(
                f"{BASE_URL}/community/circle/{circle['slug']}/settings",
                wait_until="domcontentloaded",
                timeout=15000,
            )
            expect(page.locator("#ccs-root")).to_be_visible(timeout=10000)
            _wait_for_hydration(page, circle["slug"])

            expect(page.locator("#ccs-danger-card")).to_be_hidden()
            expect(page.locator("#ccs-nav-danger")).to_be_hidden()
            expect(page.locator("#ccs-delete-confirm-modal")).to_be_hidden()

            denied = page.request.delete(
                f"{BASE_URL}/api/community/circles/{circle['id']}",
                headers=_csrf_header(ctx),
            )
            assert denied.status == 403, f"{role} delete should be forbidden"
            assert _circle_exists(circle["id"]), f"{role} delete attempt removed the circle"
            assert not errors, f"{role} JS errors: {errors[:5]}"

        owner_ctx, owner_page, owner_errors = make_context(playwright_session, owner)
        contexts.append(owner_ctx)
        still_present = owner_page.request.get(
            f"{BASE_URL}/api/community/circles/by-slug/{circle['slug']}"
        )
        assert still_present.status == 200
        assert still_present.json()["circle"]["id"] == circle["id"]
        assert not owner_errors, f"owner JS errors: {owner_errors[:5]}"
    finally:
        for ctx in contexts:
            ctx.close()
        for _, user in non_owner_roles:
            cleanup_user(user["user_id"])


@pytest.mark.community
def test_circle_owner_delete_cascades_representative_dependent_data(
    playwright_session,
    circle_owner,
):
    """Deleting a disposable Circle removes the major Circle-scoped records it owns."""
    owner, circle = circle_owner
    member = mint_user(prefix="e2e-ccs-delete-member", display_name="CCS Delete Member")
    requester = mint_user(prefix="e2e-ccs-delete-requester", display_name="CCS Delete Requester")
    ctx = None
    try:
        _add_circle_member(circle["id"], member["user_id"], "member")
        fixture = _seed_circle_delete_dependency_fixture(
            circle["id"],
            owner["user_id"],
            member["user_id"],
            requester["user_id"],
        )
        before = _count_circle_delete_dependencies(circle["id"], fixture)
        missing = {name: count for name, count in before.items() if count <= 0}
        assert not missing, f"Dependency fixture did not seed rows: {missing}"

        ctx, page, errors = make_context(playwright_session, owner)
        page.goto(
            f"{BASE_URL}/community/circle/{circle['slug']}/settings",
            wait_until="domcontentloaded",
            timeout=15000,
        )
        expect(page.locator("#ccs-root")).to_be_visible(timeout=10000)
        delete_response = page.request.delete(
            f"{BASE_URL}/api/community/circles/{circle['id']}",
            headers=_csrf_header(ctx),
        )
        assert delete_response.status == 200
        assert delete_response.json()["success"] is True

        after = _count_circle_delete_dependencies(circle["id"], fixture)
        remaining = {name: count for name, count in after.items() if count != 0}
        assert not remaining, f"Circle delete left dependent rows behind: {remaining}"

        by_slug = page.request.get(f"{BASE_URL}/api/community/circles/by-slug/{circle['slug']}")
        assert by_slug.status == 404
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        if ctx:
            ctx.close()
        cleanup_user(member["user_id"])
        cleanup_user(requester["user_id"])


@pytest.mark.community
def test_circle_resource_version_review_rejects_from_browser(playwright_session, circle_owner):
    """Owner opens version history, rejects a pending version, and DB records the review."""
    user, circle = circle_owner
    fixture = _seed_resource_review_fixture(circle["id"], user["user_id"])
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])
        row = page.locator(".ccs-resource-row").filter(has_text=fixture["title"])
        expect(row).to_be_visible(timeout=10000)

        row.get_by_role("button", name="Versions").click()
        expect(row.locator(".ccs-resource-version-compare")).to_be_visible(timeout=10000)
        version_row = row.locator(".ccs-resource-version-row").filter(has_text="v2").first
        expect(version_row).to_contain_text("Pending", timeout=5000)

        def accept_rejection_note(dialog):
            if "Rejection note" in dialog.message:
                dialog.accept("Needs updated legal copy")
            else:
                dialog.accept()

        page.once("dialog", accept_rejection_note)
        with page.expect_response(
            lambda response: (
                f"/resources/{fixture['resource_id']}/versions/{fixture['version_id']}/review"
                in response.url
                and response.request.method == "POST"
            ),
            timeout=10000,
        ) as response_info:
            version_row.get_by_role("button", name="Reject").click()

        assert response_info.value.ok
        assert _fetch_resource_version_review(fixture["version_id"]) == (
            "rejected",
            "Needs updated legal copy",
        )
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_resource_version_restore_updates_current_from_browser(
    playwright_session,
    circle_owner,
):
    """Owner restores a historical external-link version as the current resource."""
    user, circle = circle_owner
    fixture = _seed_resource_review_fixture(circle["id"], user["user_id"])
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])
        row = page.locator(".ccs-resource-row").filter(has_text=fixture["title"])
        expect(row).to_be_visible(timeout=10000)

        row.get_by_role("button", name="Versions").click()
        previous_row = row.locator(".ccs-resource-version-row").filter(has_text="v1").first
        expect(previous_row).to_be_visible(timeout=5000)

        page.once("dialog", lambda dialog: dialog.accept())
        with page.expect_response(
            lambda response: (
                f"/resources/{fixture['resource_id']}/versions/" in response.url
                and response.url.endswith("/restore")
                and response.request.method == "POST"
            ),
            timeout=10000,
        ) as response_info:
            previous_row.get_by_role("button", name="Restore").click()

        assert response_info.value.ok
        assert _fetch_resource_current_state(fixture["resource_id"]) == (
            "v1",
            fixture["previous_url"],
            ["v1"],
        )
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_resource_version_replacement_upload_from_browser(
    playwright_session,
    circle_owner,
    tmp_path,
):
    """Owner replaces a resource file and the uploaded version becomes current."""
    user, circle = circle_owner
    fixture = _seed_resource_review_fixture(circle["id"], user["user_id"])
    replacement_path = tmp_path / "replacement-report.pdf"
    replacement_path.write_bytes(
        b"%PDF-1.4\n"
        b"1 0 obj\n<< /Type /Catalog /Version /1.4 >>\nendobj\n"
        b"trailer\n<< /Root 1 0 R >>\n%%EOF\n"
    )
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])
        row = page.locator(".ccs-resource-row").filter(has_text=fixture["title"])
        expect(row).to_be_visible(timeout=10000)

        def accept_replacement_dialogs(dialog):
            if "Version label" in dialog.message:
                dialog.accept("replacement-v3")
            elif "Change note" in dialog.message:
                dialog.accept("Binary replacement from browser")
            else:
                dialog.accept()

        page.on("dialog", accept_replacement_dialogs)
        with page.expect_response(
            lambda response: (
                f"/resources/{fixture['resource_id']}/versions/upload" in response.url
                and response.request.method == "POST"
            ),
            timeout=20000,
        ) as response_info:
            with page.expect_file_chooser() as file_chooser_info:
                row.get_by_role("button", name="Replace file").click()
            file_chooser_info.value.set_files(str(replacement_path))

        assert response_info.value.ok
        expect(page.locator(".ccs-resource-row").filter(has_text=fixture["title"])).to_contain_text(
            "Current version: replacement-v3",
            timeout=10000,
        )
        uploaded = _fetch_resource_upload_state(fixture["resource_id"])
        assert uploaded is not None
        assert uploaded[1] == "replacement-v3"
        assert uploaded[2] == "uploaded"
        assert uploaded[3] == "application/pdf"
        assert uploaded[4] and uploaded[4] > 0
        assert uploaded[5] and len(uploaded[5]) == 64
        current = _fetch_resource_current_state(fixture["resource_id"])
        assert current is not None
        assert current[0] == "replacement-v3"
        assert current[1] != fixture["current_url"]
        assert current[2] == ["replacement-v3"]
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_ops_alert_workflow_state_updates_from_browser(playwright_session, circle_owner):
    """Owner moves an active Circle ops alert into a human workflow state."""
    user, circle = circle_owner
    fixture = _seed_ops_alert_fixture(circle["id"])
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])
        alert_row = page.locator(".ccs-ops-alert-row").filter(has_text=fixture["summary"])
        expect(alert_row).to_be_visible(timeout=10000)

        def accept_workflow_dialogs(dialog):
            if "Workflow state" in dialog.message:
                dialog.accept("waiting_on_policy")
            elif "Workflow note" in dialog.message:
                dialog.accept("Policy review pending")
            else:
                dialog.accept()

        page.on("dialog", accept_workflow_dialogs)
        with page.expect_response(
            lambda response: (
                f"/ops-alerts/{fixture['alert_id']}/action" in response.url
                and response.request.method == "POST"
            ),
            timeout=10000,
        ) as response_info:
            alert_row.get_by_role("button", name="Workflow").click()

        assert response_info.value.ok
        assert _fetch_ops_alert_workflow(fixture["alert_id"]) == (
            "acknowledged",
            "waiting_on_policy",
            "Policy review pending",
        )
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
