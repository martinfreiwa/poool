"""Static template-render checks for /developer/villas/<asset_id>/annual/<year>.

The Villa-Returns C3 annual data page. Renders three cards: a capex
intake form, a forecast intake form, and an evidence-document upload
form. The Rust handler serves the template for any well-formed asset_id
and year — actual data hydration happens client-side via JS.

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_annual_data_static.py -v
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


SAMPLE_UUID = "00000000-0000-0000-0000-000000000000"
SAMPLE_YEAR = 2026
ROUTE = f"/developer/villas/{SAMPLE_UUID}/annual/{SAMPLE_YEAR}"

REQUIRED_IDS = {
    "developer-annual-data-body",
    "developer-annual-data-page",
    "developer-annual-data-sidebar",
    "dad-back",
    "dad-breadcrumb",
    "dad-summary",
    # Capex card
    "dad-capex-form",
    "dad-capex-date",
    "dad-capex-amount",
    "dad-capex-category",
    "dad-capex-description",
    "dad-capex-evidence",
    "btn-capex-submit",
    "dad-capex-list",
    "capex-error",
    # Forecast card
    "dad-forecast-form",
    "btn-forecast-submit",
    "dad-forecast-list",
    "forecast-error",
    # Documents card
    "dad-doc-form",
    "dad-doc-type",
    "dad-doc-file",
    "btn-doc-upload",
}
# The user spec mentions the grid container & capex form & evidence input
# in particular — make sure those land:
SPEC_HIGHLIGHTED_IDS = {
    "dad-capex-form",
    "dad-capex-evidence",
}
REQUIRED_SCRIPT_HINTS = {
    "developer-annual-data.js",
    "profile-dropdown.js",
    "mobile-navigation.js",
}
REQUIRED_CSS_HINTS = {
    "leaderboard.css",  # base ds-* tokens
}


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
    assert any("Annual" in t for t in page.titles)


def test_has_required_ids(page):
    assert_required_ids(page, REQUIRED_IDS)


def test_user_spec_highlighted_ids_present(page):
    """Per the contract: capex form + evidence input must be present."""
    assert_required_ids(page, SPEC_HIGHLIGHTED_IDS)


def test_dad_grid_container_present(response):
    """The annual data page wraps its cards in a `.dad-grid` container."""
    assert "dad-grid" in response.text, (
        "Expected `.dad-grid` CSS class to wrap annual data cards"
    )


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
