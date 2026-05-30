"""Static template-render checks for /developer/submission-success.

The success page is the *only* developer template that ships the
`#submitted-asset-title` element. It is hidden by default and the inline
script populates it from a query parameter / sessionStorage. Confirm both
that the ID is present and that it is hidden.

This page intentionally has no dedicated bundle (`developer-submission-success.js`
was removed and is now in the FORBIDDEN_DELETED_FILES list — we assert it
is not re-referenced).

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_submission_success_static.py -v
"""
import os
import re
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


ROUTE = "/developer/submission-success"

REQUIRED_IDS = {
    "developer-submission-success-body",
    "developer-submission-success-page",
    "developer-submission-success-sidebar",
    "developer-submission-success-main",
    "submitted-asset-title",
}
REQUIRED_SCRIPT_HINTS = {
    "profile-dropdown.js",
    "mobile-navigation.js",
}
REQUIRED_CSS_HINTS = {
    "developer-submission-success.css",
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
    assert any("Success" in t or "Submission" in t for t in page.titles)


def test_has_required_ids(page):
    assert_required_ids(page, REQUIRED_IDS)


def test_submitted_asset_title_is_hidden_by_default(response):
    """The element must be present but hidden until JS fills it."""
    match = re.search(
        r'<p[^>]*id="submitted-asset-title"[^>]*>',
        response.text,
    )
    assert match, "submitted-asset-title element not found"
    tag = match.group(0)
    # `hidden` attribute OR `display:none` style — either acceptable.
    assert "hidden" in tag or "display:none" in tag.replace(" ", ""), (
        f"#submitted-asset-title must be hidden by default — got: {tag}"
    )


def test_required_scripts_loaded(page):
    assert_scripts_present(page, REQUIRED_SCRIPT_HINTS)


def test_required_stylesheets_loaded(page):
    assert_stylesheets_present(page, REQUIRED_CSS_HINTS)


def test_no_placeholder_or_lorem_text(response):
    assert_no_forbidden_global_text(response, extra=("coming soon",))


def test_no_references_to_deleted_files(response):
    # This asserts the deleted developer-submission-success.js is not
    # re-introduced as a script src on this page.
    assert_no_deleted_file_refs(response)


def test_no_bare_anchor_placeholders(page):
    assert_no_placeholder_anchors(page)
