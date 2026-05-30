"""Static template-render checks for /developer/add-asset.

The add-asset page is the asset-type chooser. Real Estate is the only
currently-supported category; the other cards legitimately render
"Coming Soon" badges — this is the ONE page where that copy is OK, so
this test explicitly does NOT include "coming soon" in the global
forbidden list.

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_add_asset_static.py -v
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


ROUTE = "/developer/add-asset"

REQUIRED_IDS = {
    "developer-add-asset-body",
    "developer-add-asset-page",
    "developer-add-asset-sidebar",
    "developer-add-asset-main",
    "asset-type-grid",
    "asset-type-card-real-estate",
    "asset-type-card-commercial-property",
    "asset-type-card-commodities",
    "asset-type-card-business",
    "asset-type-card-startups",
    "asset-type-card-land-plots",
    "add-asset-next-btn",
}
REQUIRED_SCRIPT_HINTS = {
    "developer-add-asset.js",
    "profile-dropdown.js",
    "mobile-navigation.js",
}
REQUIRED_CSS_HINTS = {
    "developer-add-asset.css",
    "developer-application-form.css",
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


def test_has_required_ids(page):
    assert_required_ids(page, REQUIRED_IDS)


def test_real_estate_card_is_selected_by_default(page):
    # The Real Estate card carries .selected per the template; the others
    # carry .coming-soon. Read it from the class catalog.
    assert "selected" in page.classes
    assert "coming-soon" in page.classes


def test_required_scripts_loaded(page):
    assert_scripts_present(page, REQUIRED_SCRIPT_HINTS)


def test_required_stylesheets_loaded(page):
    assert_stylesheets_present(page, REQUIRED_CSS_HINTS)


def test_no_lorem_or_todo(response):
    # Coming Soon is intentional here — only check the universal blacklist.
    assert_no_forbidden_global_text(response)


def test_no_references_to_deleted_files(response):
    assert_no_deleted_file_refs(response)


def test_no_bare_anchor_placeholders(page):
    assert_no_placeholder_anchors(page)
