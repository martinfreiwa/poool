"""Static template-render checks for /developer/operations.

The operations dashboard renders a matrix of villas × periods plus a
five-tile stats strip. Asserts the matrix wrap, stat tiles, year-tab strip
and the action-queue / empty-state containers are present.

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_operations_dashboard_static.py -v
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


ROOT = Path(__file__).resolve().parents[1]
ROUTE = "/developer/operations"

REQUIRED_IDS = {
    "developer-operations-body",
    "developer-operations-page",
    "developer-operations-sidebar",
    "ops-content",
    "ops-skeleton",
    "ops-stats",
    "stat-missing",
    "stat-drafts",
    "stat-review",
    "stat-published",
    "stat-docs",
    "ops-matrix-wrap",
    "ops-matrix-thead",
    "ops-matrix-tbody",
    "ops-year-tabs",
    "ops-mobile-list",
    "ops-empty",
}
REQUIRED_SCRIPT_HINTS = {
    "developer-operations-dashboard.js",
    "profile-dropdown.js",
    "mobile-navigation.js",
}
REQUIRED_CSS_HINTS = {
    "developer-operations.css",
}


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_operations_empty_state_uses_branded_art():
    template = read("frontend/platform/developer/operations-dashboard.html")
    css = read("frontend/platform/static/css/developer-operations.css")

    assert 'class="dae-empty__operations-art"' in template
    assert 'class="dae-empty__operations-lockup"' in template
    assert 'src="/static/images/icons/logo-pool.svg"' in template
    assert "ON TIME" not in template
    assert "AWAITING VILLAS" not in template
    assert 'id="ops-bg-grad"' not in template

    assert "body#developer-operations-body .dae-empty__operations-art {" in css
    assert "body#developer-operations-body .dae-empty__operations-lockup {" in css
    assert "background: #03FF88;" in css
    assert "linear-gradient(135deg, #0000FF 0%, #001DCA 62%, #07107C 100%)" in css
    assert "body#developer-operations-body .dae-empty__operations-progress span" in css


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
    assert any("Operations" in t for t in page.titles)


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
