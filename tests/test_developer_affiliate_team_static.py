"""Static template-render checks for /developer/affiliate-team and sub-pages.

The Affiliate Team section is a tabbed shell with seven routes:
  * /developer/affiliate-team           (Analytics — canonical landing)
  * /developer/affiliate-team/analytics (alias of the above)
  * /developer/affiliate-team/members
  * /developer/affiliate-team/customers
  * /developer/affiliate-team/products
  * /developer/affiliate-team/settings
  * /developer/affiliate-team/tier

All seven share the same body ID (`developer-affiliate-team-body`), the
same shell partial (`_affiliate_team_shell.html` → which contributes the
`dat-page-content` anchor + `dat-team-meta-mount` element), and the
shared `developer-affiliate-team-shell.js` bundle. Each sub-page then
loads its own additional JS bundle.

This module parametrises over all seven routes and runs the structural
suite against each.

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_affiliate_team_static.py -v
"""
import os
import sys

import pytest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from _developer_static import (  # noqa: E402
    assert_meta_viewport,
    assert_no_deleted_file_refs,
    assert_no_forbidden_global_text,
    assert_no_placeholder_anchors,
    assert_required_ids,
    assert_scripts_present,
    assert_stylesheets_present,
    assert_title_non_empty,
    fetch_page,
    parse_page,
)


# Common per-sub-page contract: each sub-page is identified by route +
# the script bundle it pulls in addition to the shell + a representative
# ID unique to that sub-page.
AFFILIATE_TEAM_PAGES = [
    pytest.param(
        "/developer/affiliate-team",
        "developer-affiliate-team-analytics.js",
        "dat-analytics-root",
        id="analytics-landing",
    ),
    pytest.param(
        "/developer/affiliate-team/analytics",
        "developer-affiliate-team-analytics.js",
        "dat-analytics-root",
        id="analytics-alias",
    ),
    pytest.param(
        "/developer/affiliate-team/members",
        "developer-affiliate-team-members.js",
        "dat-members-tbody",
        id="members",
    ),
    pytest.param(
        "/developer/affiliate-team/customers",
        "developer-affiliate-team-customers.js",
        "dat-customers-tbody",
        id="customers",
    ),
    pytest.param(
        "/developer/affiliate-team/products",
        "developer-affiliate-team-products.js",
        "dat-products-tbody",
        id="products",
    ),
    pytest.param(
        "/developer/affiliate-team/settings",
        "developer-affiliate-team-settings.js",
        "dat-settings-form",
        id="settings",
    ),
    pytest.param(
        "/developer/affiliate-team/tier",
        "developer-affiliate-team-tier.js",
        "dat-tier-hero-title",
        id="tier",
    ),
]


# All sub-pages share these structural IDs (body + shell mount points).
SHARED_REQUIRED_IDS = {
    "developer-affiliate-team-body",
    "developer-affiliate-team-main",
    "dat-page-content",  # from _affiliate_team_shell.html (sr-only h2)
    "dat-team-meta-mount",
}
SHARED_SCRIPT_HINTS = {
    "developer-affiliate-team-shell.js",
    "profile-dropdown.js",
    "mobile-navigation.js",
}
SHARED_CSS_HINTS = {
    "developer-affiliate-team.css",
    "developer-dashboard.css",
    "developer-leaderboard-navbar.css",
}


@pytest.fixture(scope="module")
def fetched():
    """Lazy cache: route → (response, page). Skips once when prereqs miss."""
    cache: dict[str, tuple] = {}

    def get(route: str):
        if route not in cache:
            response = fetch_page(route)
            cache[route] = (response, parse_page(response))
        return cache[route]

    return get


@pytest.mark.parametrize("route,sub_script,sub_id", AFFILIATE_TEAM_PAGES)
def test_status_200(fetched, route, sub_script, sub_id):
    response, _ = fetched(route)
    assert response.status_code == 200, (
        f"{route}: expected 200, got {response.status_code}"
    )


@pytest.mark.parametrize("route,sub_script,sub_id", AFFILIATE_TEAM_PAGES)
def test_meta_viewport_present(fetched, route, sub_script, sub_id):
    _, page = fetched(route)
    assert_meta_viewport(page)


@pytest.mark.parametrize("route,sub_script,sub_id", AFFILIATE_TEAM_PAGES)
def test_title_non_empty(fetched, route, sub_script, sub_id):
    _, page = fetched(route)
    assert_title_non_empty(page)


@pytest.mark.parametrize("route,sub_script,sub_id", AFFILIATE_TEAM_PAGES)
def test_shared_shell_ids_present(fetched, route, sub_script, sub_id):
    _, page = fetched(route)
    assert_required_ids(page, SHARED_REQUIRED_IDS)


@pytest.mark.parametrize("route,sub_script,sub_id", AFFILIATE_TEAM_PAGES)
def test_sub_page_id_present(fetched, route, sub_script, sub_id):
    _, page = fetched(route)
    assert sub_id in page.ids, (
        f"{route}: missing distinctive sub-page ID {sub_id!r}. "
        f"Got: {sorted(page.ids)[:20]}…"
    )


@pytest.mark.parametrize("route,sub_script,sub_id", AFFILIATE_TEAM_PAGES)
def test_shared_scripts_loaded(fetched, route, sub_script, sub_id):
    _, page = fetched(route)
    assert_scripts_present(page, SHARED_SCRIPT_HINTS)


@pytest.mark.parametrize("route,sub_script,sub_id", AFFILIATE_TEAM_PAGES)
def test_sub_page_script_loaded(fetched, route, sub_script, sub_id):
    _, page = fetched(route)
    blob = page.script_blob
    assert sub_script in blob, (
        f"{route}: missing sub-page bundle {sub_script}. "
        f"Loaded scripts: {page.scripts}"
    )


@pytest.mark.parametrize("route,sub_script,sub_id", AFFILIATE_TEAM_PAGES)
def test_shared_stylesheets_loaded(fetched, route, sub_script, sub_id):
    _, page = fetched(route)
    assert_stylesheets_present(page, SHARED_CSS_HINTS)


@pytest.mark.parametrize("route,sub_script,sub_id", AFFILIATE_TEAM_PAGES)
def test_no_placeholder_or_lorem_text(fetched, route, sub_script, sub_id):
    response, _ = fetched(route)
    assert_no_forbidden_global_text(response, extra=("coming soon",))


@pytest.mark.parametrize("route,sub_script,sub_id", AFFILIATE_TEAM_PAGES)
def test_no_references_to_deleted_files(fetched, route, sub_script, sub_id):
    response, _ = fetched(route)
    assert_no_deleted_file_refs(response)


@pytest.mark.parametrize("route,sub_script,sub_id", AFFILIATE_TEAM_PAGES)
def test_no_bare_anchor_placeholders(fetched, route, sub_script, sub_id):
    _, page = fetched(route)
    assert_no_placeholder_anchors(page)
