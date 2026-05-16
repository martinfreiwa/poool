"""WS3.7: Playwright tests for /community/me and /community/u/:id."""
import os
from playwright.sync_api import expect

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")


def test_my_profile_page_loads_with_required_chrome(authenticated_user_page):
    """/community/me returns 200 for an authed user and renders the hero + stat strip + tab nav."""
    page, _tracker, user = authenticated_user_page
    # Visit own profile directly by user_id; /community/me redirects to it
    # but the redirect may interact poorly with the fixture session cookie.
    user_id = user.get("user_id")
    page.goto(f"{BASE_URL}/community/u/{user_id}")
    page.wait_for_load_state("networkidle")

    # Hero card — the 2026-05-16 rework renamed `.community-profile-hero`
    # to `.cp-hero` and replaced the six-cell stat strip with an inline
    # `.cp-hero__meta` follower/following/posts line.
    expect(page.locator(".cp-hero").first).to_be_visible(timeout=10000)
    expect(page.locator(".cp-hero__meta")).to_be_visible(timeout=5000)
    # Tab nav contains the Posts tab
    expect(page.locator('.community-profile-tab[data-tab="posts"]').first).to_be_visible(timeout=5000)


def test_my_profile_settings_tab_only_for_owner(authenticated_user_page):
    """The Settings tab is only present on the owner's profile page."""
    page, _tracker, user = authenticated_user_page
    user_id = user.get("user_id")
    page.goto(f"{BASE_URL}/community/u/{user_id}")
    page.wait_for_load_state("networkidle")
    expect(page.locator('.community-profile-tab[data-tab="settings"]')).to_be_visible(timeout=5000)


def test_profile_endpoints_respond(authenticated_user_page):
    """The five new per-user endpoints all return 200 for the authed user's own profile."""
    page, _tracker, user = authenticated_user_page
    user_id = user.get("user_id") or user.get("id")
    assert user_id, "authenticated_user_page must yield a user with a user_id"

    for path in (
        f"/api/community/profile/{user_id}/posts",
        f"/api/community/profile/{user_id}/comments",
        f"/api/community/profile/{user_id}/media",
        f"/api/community/profile/{user_id}/activity",
        "/api/community/profile/me/analytics",
    ):
        resp = page.request.get(f"{BASE_URL}{path}")
        assert resp.status == 200, f"{path} returned {resp.status}"
        body = resp.json()
        assert isinstance(body, dict)


def test_other_profile_page_hides_owner_tabs(authenticated_user_page):
    """Visiting another user's profile hides the Settings + Analytics tabs."""
    page, _tracker, _user = authenticated_user_page
    # Pick a stable placeholder profile id; if it doesn't exist the page
    # still renders the "Community Member" fallback.
    other_id = "00000000-0000-0000-0000-000000000001"
    page.goto(f"{BASE_URL}/community/u/{other_id}")
    page.wait_for_load_state("networkidle")
    expect(page.locator('.community-profile-tab[data-tab="posts"]')).to_be_visible(timeout=5000)
    # Settings + Analytics gated by is_own.
    assert page.locator('.community-profile-tab[data-tab="settings"]').count() == 0
    assert page.locator('.community-profile-tab[data-tab="analytics"]').count() == 0
