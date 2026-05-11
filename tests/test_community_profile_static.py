"""Static contract tests for the WS3 community-profile page.

These verify markup + JS contracts without booting a real server. They
guard against accidental rename of IDs, deletion of required script tags,
or drift between the page template and community-profile.js.
"""
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_profile_page_template_has_required_blocks():
    html = read("frontend/platform/community-profile.html")
    # Bootstrap globals consumed by community-profile.js
    assert "window.PROFILE_USER_ID" in html
    assert "window.PROFILE_IS_OWN" in html
    # Hero header
    assert 'class="community-profile-hero ds-card"' in html
    assert "community-profile-hero__avatar" in html
    assert "community-profile-hero__name" in html
    # Stat strip
    assert 'class="community-profile-stats"' in html
    for label in ["XP", "Level", "Followers", "Following", "Posts", "Streak"]:
        assert f">{label}<" in html
    # Tab nav contains every panel we wire up in JS
    for tab in [
        "posts", "comments", "followers", "following",
        "media", "circle", "activity",
    ]:
        assert f'data-tab="{tab}"' in html
    # is_own-only tabs gated by Jinja conditional
    assert '{% if is_own %}' in html
    assert 'data-tab="analytics"' in html
    assert 'data-tab="settings"' in html
    # JS module pulled in
    assert "'community-profile'" in html


def test_profile_js_has_required_exports_and_endpoints():
    js = read("frontend/platform/static/js/community-profile.js")
    # Endpoints invoked by each loader
    assert "/api/community/profile/${PROFILE_ID}/posts" in js
    assert "/api/community/profile/${PROFILE_ID}/comments" in js
    assert "/api/community/profile/${PROFILE_ID}/media" in js
    assert "/api/community/profile/${PROFILE_ID}/activity" in js
    assert "/api/community/profile/me/analytics" in js
    # Page-level helpers
    assert "window.communityProfile" in js
    assert "function setActiveTab" in js
    # Defers follow toggling to community-feed.js
    assert "window.toggleFollow" in js


def test_profile_css_has_required_classes():
    css = read("frontend/platform/static/css/community-profile.css")
    for cls in [
        ".community-profile-hero",
        ".community-profile-stats",
        ".community-profile-stat",
        ".community-profile-tabs",
        ".community-profile-tab",
        ".community-profile-panel",
        ".community-profile-media-tile",
        ".community-profile-activity-row",
        ".community-profile-circle-card",
        ".community-profile-analytics",
        ".community-profile-load-more",
    ]:
        assert cls in css, f"missing class {cls} in community-profile.css"


def test_backend_routes_registered():
    main = read("backend/src/main.rs")
    assert '"/community/me"' in main
    assert '"/community/u/:user_id"' in main
    assert "page_community_my_profile" in main
    assert "page_community_user_profile" in main
    assert "render_community_profile" in main


def test_backend_profile_endpoints_registered():
    routes = read("backend/src/community/routes.rs")
    assert '/api/community/profile/:id/posts"' in routes
    assert '/api/community/profile/:id/comments"' in routes
    assert '/api/community/profile/:id/media"' in routes
    assert '/api/community/profile/:id/activity"' in routes
    assert '/api/community/profile/me/analytics"' in routes
    # Migration 035 created the profile_views table that analytics reads.
    assert "profile_views" in routes


def test_cross_linking_from_modal_and_sidebar():
    html = read("frontend/platform/community.html")
    feed = read("frontend/platform/partials/community_feed.html")
    # User-profile modal exposes a "View full profile" link.
    assert "profile-modal-view-full" in html
    # Sidebar Edit-profile card has a "View my profile" link.
    assert "/community/me" in feed
