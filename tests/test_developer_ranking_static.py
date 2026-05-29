"""Static template-render checks for /developer/ranking.

The developer Ranking page is the investor leaderboard embedded in the
developer shell — same template that loads `leaderboard.js` directly via
a `<script>` tag at the bottom of the page (NOT via the `extra_js`
list). We assert the script tag survives and the leaderboard layer IDs
are present.

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_ranking_static.py -v
"""
import os
import sys
from pathlib import Path

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


ROUTE = "/developer/ranking"

REQUIRED_IDS = {
    "leaderboard-body",
    "lb-loading-layer",
    "lb-error-layer",
    "lb-empty-layer",
    "lb-content-layer",
    "lb-my-rank-card",
    "lb-my-rank",
    "lb-rankings-table",
    "lb-rankings-body",
    "lb-pagination-controls",
}
REQUIRED_SCRIPT_HINTS = {
    "leaderboard.js",
    "profile-dropdown.js",
}
REQUIRED_CSS_HINTS = {
    "leaderboard.css",
}
REPO_ROOT = Path(__file__).resolve().parents[1]
RANKING_TEMPLATE = REPO_ROOT / "frontend/platform/developer/ranking.html"
LEADERBOARD_CSS = REPO_ROOT / "frontend/platform/static/css/leaderboard.css"


@pytest.fixture(scope="module")
def response():
    return fetch_page(ROUTE)


@pytest.fixture(scope="module")
def page(response):
    return parse_page(response)


def test_status_200(response):
    assert response.status_code == 200


def test_meta_viewport_present(page):
    assert_meta_viewport(page)


def test_title_non_empty(page):
    assert_title_non_empty(page)
    assert any("Ranking" in t or "Leaderboard" in t for t in page.titles)


def test_has_required_ids(page):
    assert_required_ids(page, REQUIRED_IDS)


def test_required_scripts_loaded(page):
    assert_scripts_present(page, REQUIRED_SCRIPT_HINTS)


def test_required_stylesheets_loaded(page):
    assert_stylesheets_present(page, REQUIRED_CSS_HINTS)


def test_no_placeholder_or_lorem_text(response):
    assert_no_forbidden_global_text(response, extra=("coming soon",))


def test_no_references_to_deleted_files(response):
    assert_no_deleted_file_refs(response)


def test_no_bare_anchor_placeholders(page):
    assert_no_placeholder_anchors(page)


def test_empty_state_uses_logo_and_centered_cta_contract():
    html = RANKING_TEMPLATE.read_text(encoding="utf-8")
    css = LEADERBOARD_CSS.read_text(encoding="utf-8")
    empty_state = html.split('id="lb-empty-layer"', 1)[1].split("<!-- 4. CONTENT STATE -->", 1)[0]

    assert "lb-empty-logo" in empty_state
    assert "/static/images/logos/Logo%20Pool.svg" in empty_state
    assert 'class="empty-icon"' not in empty_state
    assert ".retry-btn" in css
    assert "display: inline-flex;" in css
    assert "align-items: center;" in css
    assert "justify-content: center;" in css
