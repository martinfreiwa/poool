"""
Circle product journey E2E.

Exercises the canonical user path:
  /community/circles -> Open Circle -> Circle Feed -> Post -> Comment -> Manage.
"""

import uuid
import time

import pytest
from playwright.sync_api import expect

from community_helpers import BASE_URL, cleanup_user, make_context, mint_user, seed_circle


@pytest.mark.community
def test_my_circles_to_circle_feed_post_comment_and_manage(playwright_session):
    user = mint_user(prefix="e2e-circle-journey", display_name="Circle Journey")
    circle = seed_circle(
        user["user_id"],
        name=f"Journey Circle {uuid.uuid4().hex[:6]}",
    )
    ctx, page, errors = make_context(playwright_session, user)
    try:
        started = time.monotonic()
        page.goto(f"{BASE_URL}/community/circles", wait_until="domcontentloaded", timeout=15000)
        expect(page.locator("#community-circle-tab")).to_be_visible(timeout=10000)
        page.wait_for_function(
            "() => { const el = document.getElementById('cc-my-circles-list');"
            " return el && !el.textContent.includes('Loading your circles'); }",
            timeout=10000,
        )
        circles_loaded_ms = int((time.monotonic() - started) * 1000)
        assert circles_loaded_ms < 8000

        circle_link = page.locator("#cc-my-circles-list .cc-pill").filter(
            has_text=circle["name"]
        ).first
        expect(circle_link).to_be_visible(timeout=5000)
        expect(circle_link).to_have_attribute(
            "href",
            f"/community/circle/{circle['slug']}",
        )
        started = time.monotonic()
        circle_link.click()

        page.wait_for_url(f"**/community/circle/{circle['slug']}", timeout=10000)
        expect(page.locator("#circle-space-title")).to_have_text(circle["name"], timeout=10000)
        expect(page.locator(".circle-space-composer__scope")).to_contain_text(
            f"Post to: {circle['name']}"
        )
        expect(page.get_by_role("link", name="Manage Circle")).to_have_attribute(
            "href",
            f"/community/circle/{circle['slug']}/settings",
        )
        page.wait_for_function(
            "() => { const el = document.getElementById('community-feed-container');"
            " return el && !el.querySelector('.community-feed-skeleton'); }",
            timeout=10000,
        )
        circle_loaded_ms = int((time.monotonic() - started) * 1000)
        assert circle_loaded_ms < 8000

        post_content = f"Circle journey post {uuid.uuid4().hex[:8]}"
        page.fill("#post-content-input", post_content)
        with page.expect_response(
            lambda response: (
                f"/api/community/circles/{circle['id']}/posts" in response.url
                and response.request.method == "POST"
            ),
            timeout=10000,
        ) as post_response:
            page.click("#submit-post-btn")
        assert post_response.value.status in (200, 201)
        post_id = post_response.value.json()["id"]

        page.evaluate("document.body.dispatchEvent(new Event('reload-feed'))")
        post = page.locator(f'.feed-post[data-post-id="{post_id}"]').first
        expect(post).to_be_visible(timeout=10000)
        expect(post).to_contain_text(post_content)

        comment_toggle = post.locator(
            f"button.feed-reaction-btn[aria-controls='comments-section-{post_id}']"
        ).first
        comment_toggle.click()
        expect(page.locator(f"#comments-section-{post_id}")).to_be_visible(timeout=5000)

        comment_content = f"Circle journey comment {uuid.uuid4().hex[:8]}"
        page.fill(f"#comment-input-{post_id}", comment_content)
        with page.expect_response(
            lambda response: (
                response.url.endswith(f"/api/community/posts/{post_id}/comments")
                and response.request.method == "POST"
            ),
            timeout=10000,
        ) as comment_response:
            page.locator(f"#comments-section-{post_id}").get_by_role(
                "button",
                name="Send comment",
            ).click()
        assert comment_response.value.status == 200
        expect(page.locator(f"#comments-list-{post_id}")).to_contain_text(
            comment_content,
            timeout=10000,
        )

        started = time.monotonic()
        page.get_by_role("link", name="Manage Circle").click()
        page.wait_for_url(f"**/community/circle/{circle['slug']}/settings", timeout=10000)
        expect(page.locator("#ccs-root")).to_be_visible(timeout=10000)
        settings_loaded_ms = int((time.monotonic() - started) * 1000)
        assert settings_loaded_ms < 8000

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
        cleanup_user(user["user_id"])
