"""Static template-render checks for /developer/submissions (layout only).

This file deliberately coexists with `test_developer_submissions_static.py`
which contains the older source-file-reading assertions. This module is
the HTTP-level layout checker: it fetches the rendered page and asserts
the structural skeleton survives across deploys.

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_submissions_layout_static.py -v
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


ROUTE = "/developer/submissions"

REQUIRED_IDS = {
    "developer-submissions-body",
    "developer-submissions-page",
    "developer-submissions-sidebar",
    "developer-submissions-main",
    "sub-stats-row",
    "stat-all",
    "stat-submitted",
    "stat-approved",
    "stat-rejected",
    "submissions-table-container",
    "submissions-table",
    "sub-toolbar",
    "sub-search-input",
    "sub-sort-trigger",
}
REQUIRED_SCRIPT_HINTS = {
    "developer-submissions.js",
    "profile-dropdown.js",
    "mobile-navigation.js",
}
REQUIRED_CSS_HINTS = {
    "developer-submissions.css",
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
    assert any("Submission" in t for t in page.titles)


def test_has_required_ids(page):
    assert_required_ids(page, REQUIRED_IDS)


def test_filter_tabs_include_rejected(page):
    """Submissions stat filter must include the `rejected` bucket."""
    data_filters = {v for (_t, k, v) in page.data_attrs if k == "data-filter"}
    assert "rejected" in data_filters, (
        f"`data-filter=\"rejected\"` missing — got {sorted(data_filters)}"
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
