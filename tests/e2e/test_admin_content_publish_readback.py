import uuid

import psycopg2
import requests
from playwright.sync_api import expect

from tests.e2e.conftest import (
    BASE_URL,
    DB_URL,
    cleanup_test_user,
    create_e2e_user,
    get_db_connection,
)


def _seed_blog_taxonomy(marker: str):
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO blog_authors (name, slug, bio)
            VALUES (%s, %s, %s)
            RETURNING id
            """,
            (
                f"Workflow Author {marker}",
                f"workflow-author-{marker}",
                "E2E workflow author",
            ),
        )
        author_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO blog_categories (name, slug, description, color, icon, sort_order)
            VALUES (%s, %s, %s, '#1f7a4d', 'file-text', 999)
            RETURNING id
            """,
            (
                f"Workflow Content {marker}",
                f"workflow-content-{marker}",
                "Disposable workflow content category",
            ),
        )
        category_id = cur.fetchone()[0]
        conn.commit()
        return str(author_id), str(category_id), f"workflow-content-{marker}"
    finally:
        cur.close()
        conn.close()


def _cleanup_blog_records(article_id: str | None, marker: str, admin_user_id):
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    try:
        if article_id:
            cur.execute("DELETE FROM blog_articles WHERE id = %s", (article_id,))
        cur.execute("DELETE FROM blog_categories WHERE slug = %s", (f"workflow-content-{marker}",))
        cur.execute("DELETE FROM blog_authors WHERE slug = %s", (f"workflow-author-{marker}",))
        cur.execute(
            """
            DELETE FROM audit_logs
            WHERE actor_user_id = %s
              AND action LIKE %s
              AND new_state::text LIKE %s
            """,
            (admin_user_id, "blog.article.%", f"%workflow-content-{marker}%"),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _audit_actions_for(article_id: str):
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT action
            FROM audit_logs
            WHERE entity_type = 'blog'
              AND new_state ->> 'entity_id' = %s
            ORDER BY created_at
            """,
            (article_id,),
        )
        return [row[0] for row in cur.fetchall()]
    finally:
        cur.close()
        conn.close()


def test_admin_content_publish_archive_and_public_readback(quality_page):
    page, tracker = quality_page
    marker = uuid.uuid4().hex[:8]
    slug = f"workflow-content-{marker}"
    title = f"Workflow Content Publish {marker}"
    article_id = None
    admin = create_e2e_user(
        email_prefix="e2e-content-admin",
        display_name="E2E Content Admin",
        roles=("admin", "super_admin"),
    )
    author_id, category_id, category_slug = _seed_blog_taxonomy(marker)
    session = requests.Session()
    session.cookies.set("poool_session", admin["session_token"])

    try:
        csrf_page = session.get(f"{BASE_URL}/admin/blog", timeout=10)
        assert csrf_page.status_code == 200, csrf_page.text
        csrf_token = session.cookies.get("csrf_token")
        assert csrf_token, "Expected CSRF cookie from admin blog page"
        session.headers.update({"X-CSRF-Token": csrf_token})

        page.context.add_cookies(
            [{"name": "poool_session", "value": admin["session_token"], "url": BASE_URL}]
        )
        admin_response = tracker.navigate_and_check(f"{BASE_URL}/admin/blog")
        assert admin_response is not None and admin_response.status == 200
        expect(page.locator("body")).to_contain_text("Blog")

        payload = {
            "slug": slug,
            "title": title,
            "subtitle": "Workflow subtitle",
            "excerpt": "Disposable workflow article for browser publish readback.",
            "content": "## Workflow body\n\nThis article verifies admin publish readback.",
            "content_html": "<h2>Workflow body</h2><p>This article verifies admin publish readback.</p>",
            "author_id": author_id,
            "category_id": category_id,
            "tags": ["workflow", "e2e"],
            "cover_image_url": "https://images.unsplash.com/photo-1500530855697-b586d89ba3ee",
            "reading_time_minutes": 3,
            "featured": False,
            "schema_type": "BlogPosting",
            "status": "draft",
        }
        create_response = session.post(
            f"{BASE_URL}/api/blog/admin/articles", json=payload, timeout=10
        )
        assert create_response.status_code == 201, create_response.text
        article_id = create_response.json()["id"]

        draft_public = page.goto(f"{BASE_URL}/blog/{slug}", wait_until="domcontentloaded")
        assert draft_public is not None and draft_public.status == 404

        publish_response = session.post(
            f"{BASE_URL}/api/blog/admin/articles/{article_id}/publish", timeout=10
        )
        assert publish_response.status_code == 200, publish_response.text

        blog_response = tracker.navigate_and_check(f"{BASE_URL}/blog")
        assert blog_response is not None and blog_response.status == 200
        expect(page.locator("body")).to_contain_text(title)

        article_response = tracker.navigate_and_check(f"{BASE_URL}/blog/{slug}")
        assert article_response is not None and article_response.status == 200
        expect(page.locator("h1")).to_contain_text(title)
        expect(page.locator("body")).to_contain_text("Workflow body")

        category_response = tracker.navigate_and_check(f"{BASE_URL}/blog/category/{category_slug}")
        assert category_response is not None and category_response.status == 200
        expect(page.locator("body")).to_contain_text(title)

        archive_response = session.delete(
            f"{BASE_URL}/api/blog/admin/articles/{article_id}", timeout=10
        )
        assert archive_response.status_code == 200, archive_response.text

        archived_public = page.goto(f"{BASE_URL}/blog/{slug}", wait_until="domcontentloaded")
        assert archived_public is not None and archived_public.status == 404

        actions = _audit_actions_for(article_id)
        assert "blog.article.create" in actions
        assert "blog.article.publish" in actions
        assert "blog.article.archive" in actions

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures(ignore_status=[404])
    finally:
        _cleanup_blog_records(article_id, marker, admin["user_id"])
        cleanup_test_user(admin["user_id"])
