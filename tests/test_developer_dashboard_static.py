"""Static template-render checks for /developer/dashboard.

These tests GET the route against a running backend and assert the HTML
contains the expected structure. They do NOT validate data — for that, see
the Rust HTTP integration tests in backend/tests/developer_*_http.rs.

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_dashboard_static.py -v
"""
import os
import sys
from pathlib import Path

import pytest

# Make the helper importable whether pytest invokes us as a script or as
# part of the `tests` package — both modes happen in CI.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from _developer_static import (  # noqa: E402  pylint: disable=wrong-import-position
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


ROUTE = "/developer/dashboard"
REPO_ROOT = Path(__file__).resolve().parents[1]
DASHBOARD_HTML = REPO_ROOT / "frontend/platform/developer/dashboard.html"
DASHBOARD_CSS = REPO_ROOT / "frontend/platform/static/css/developer-dashboard.css"

REQUIRED_IDS = {
    "developer-dashboard-body",
    "dashboard-content-wrapper",
    "dashboard-main-content",
    "dashboard-insights-grid",
    "metrics-section",
    "activity-snapshot-section",
    "activity-snapshot-grid",
    "sales-chart-section",
}
REQUIRED_SCRIPT_HINTS = {
    "developer-dashboard.js",
    "profile-dropdown.js",
    "mobile-navigation.js",
}
REQUIRED_CSS_HINTS = {
    "developer-dashboard.css",
    "developer-leaderboard-navbar.css",
}


@pytest.fixture(scope="module")
def response():
    return fetch_page(ROUTE)


@pytest.fixture(scope="module")
def page(response):
    return parse_page(response)


def test_status_200(response):
    assert response.status_code == 200, (
        f"Expected 200, got {response.status_code} — "
        f"check auth cookie and that the route is registered"
    )


def test_meta_viewport_present(page):
    assert_meta_viewport(page)


def test_title_non_empty(page):
    assert_title_non_empty(page)
    assert any("Dashboard" in t or "Developer" in t for t in page.titles), (
        f"Title does not look like the developer dashboard: {page.titles}"
    )


def test_has_required_ids(page):
    assert_required_ids(page, REQUIRED_IDS)


def test_required_scripts_loaded(page):
    assert_scripts_present(page, REQUIRED_SCRIPT_HINTS)


def test_required_stylesheets_loaded(page):
    assert_stylesheets_present(page, REQUIRED_CSS_HINTS)


def test_no_placeholder_or_lorem_text(response):
    # Dashboard must never legitimately show "coming soon" copy.
    assert_no_forbidden_global_text(response, extra=("coming soon",))


def test_no_references_to_deleted_files(response):
    assert_no_deleted_file_refs(response)


def test_no_bare_anchor_placeholders(page):
    assert_no_placeholder_anchors(page)


def test_review_banner_status_pill_uses_green_background_with_blue_text():
    css = DASHBOARD_CSS.read_text(encoding="utf-8")
    block = css.split(".dev-review-banner__status", 1)[1].split("}", 1)[0]

    assert "background: var(--checkbox-selected-check-color, #03FF88);" in block
    assert "color: var(--btn-primary-bg, #0000FF);" in block


def test_review_banner_logo_lockup_uses_green_background_with_blue_logo():
    css = DASHBOARD_CSS.read_text(encoding="utf-8")
    brand_block = css.split(".dev-review-banner__brand", 1)[1].split("}", 1)[0]
    logo_svg = (
        REPO_ROOT / "frontend/platform/static/images/icons/logo-pool.svg"
    ).read_text(encoding="utf-8")

    assert "background: var(--checkbox-selected-check-color, #03FF88);" in brand_block
    assert 'fill="#001DCA"' in logo_svg


def test_review_banner_has_no_green_top_strip():
    css = DASHBOARD_CSS.read_text(encoding="utf-8")
    banner_block = css.split(".dev-review-banner {", 1)[1].split("}", 1)[0]
    before_block = css.split(".dev-review-banner::before", 1)[1].split("}", 1)[0]

    assert "border: 1px solid rgba(112, 132, 255, 0.46);" in banner_block
    assert "content: none;" in before_block
    assert "display: none;" in before_block


def test_dashboard_insights_grid_pairs_chart_left_with_six_activity_tiles():
    html = DASHBOARD_HTML.read_text(encoding="utf-8")
    css = DASHBOARD_CSS.read_text(encoding="utf-8")

    insights = html.split('id="dashboard-insights-grid"', 1)[1].split("<script>", 1)[0]
    assert insights.index('id="sales-chart-section"') < insights.index('id="activity-snapshot-section"')
    assert "loop.index <= 10" in insights

    grid_block = css.split(".dashboard-insights-grid {", 1)[1].split("}", 1)[0]
    activity_grid_block = (
        css.split(".dashboard-insights-grid .developer-secondary-metrics__grid", 1)[1]
        .split("}", 1)[0]
    )
    assert "grid-template-columns: minmax(0, 1.42fr) minmax(360px, 0.92fr);" in grid_block
    assert "grid-template-columns: repeat(2, minmax(0, 1fr));" in activity_grid_block
