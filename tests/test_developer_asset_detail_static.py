"""Static template-render checks for /developer/asset-detail.

The asset-detail page renders client-side (most of the body is populated by
JS after fetching the asset by ID). The static template still ships the
full tab strip, the Settings panel buttons (toggle-featured / btn-freeze)
and the breadcrumb shell — we assert those structural pieces survive.

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_asset_detail_static.py -v
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


ROUTE = "/developer/asset-detail"

REQUIRED_IDS = {
    "developer-asset-detail-body",
    "developer-assets-page",
    "developer-assets-sidebar",
    "asset-content",
    "asset-tabs",
    "panel-overview",
    "panel-media",
    "panel-documents",
    "panel-financials",
    "panel-milestones",
    "panel-captable",
    "panel-orders",
    "panel-settings",
    "toggle-featured",
    "btn-freeze",
    "loading-overlay",
}
REQUIRED_SCRIPT_HINTS = {
    "developer-asset-detail.js",
    "developer-asset-edit.js",
    "csrf.js",
    "profile-dropdown.js",
}
REQUIRED_CSS_HINTS = {
    "developer-asset-detail.css",
    "developer-assets.css",
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
    assert any("Asset Details" in t or "Asset" in t for t in page.titles)


def test_has_required_ids(page):
    assert_required_ids(page, REQUIRED_IDS)


def test_settings_tab_present(page):
    data_tabs = {v for (_t, k, v) in page.data_attrs if k == "data-tab"}
    assert "settings" in data_tabs, f"Missing Settings tab — data-tab values: {sorted(data_tabs)}"
    assert "overview" in data_tabs
    assert "documents" in data_tabs


def test_required_scripts_loaded(page):
    assert_scripts_present(page, REQUIRED_SCRIPT_HINTS)


def test_required_stylesheets_loaded(page):
    assert_stylesheets_present(page, REQUIRED_CSS_HINTS)


def test_no_placeholder_or_lorem_text(response):
    # The Coming Soon copy lives only on add-asset; should not bleed here.
    assert_no_forbidden_global_text(response, extra=("coming soon",))


def test_no_references_to_deleted_files(response):
    assert_no_deleted_file_refs(response)


def test_no_bare_anchor_placeholders(page):
    assert_no_placeholder_anchors(page)
