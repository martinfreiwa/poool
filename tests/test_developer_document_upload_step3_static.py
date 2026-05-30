"""Static template-render checks for /developer/document-upload-step3.

The page renders six dropzone sections (one per required document type),
each with its own file-upload area, hidden file input, and uploaded-files
list container. The IDs follow a `<area>-<n>` and `documents-section-<n>`
naming convention; we assert at least sections 1–6 exist.

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_document_upload_step3_static.py -v
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


ROUTE = "/developer/document-upload-step3"

REQUIRED_IDS = {
    "developer-document-upload-step3-body",
    "developer-document-upload-step3-page",
    "developer-document-upload-step3-sidebar",
    "developer-document-upload-step3-main",
}
# Sections 1–6 each contribute three IDs: the section wrap, the dropzone
# area, the hidden file input, and the uploaded files list. Be conservative
# and assert sections 1–4 (the page may add/remove specific document types
# over time without breaking the dropzone scaffolding).
DROPZONE_INDICES = (1, 2, 3, 4)
DROPZONE_IDS = {
    f"documents-section-{n}" for n in DROPZONE_INDICES
} | {
    f"file-upload-area-{n}" for n in DROPZONE_INDICES
} | {
    f"file-input-{n}" for n in DROPZONE_INDICES
} | {
    f"uploaded-files-list-{n}" for n in DROPZONE_INDICES
}
REQUIRED_SCRIPT_HINTS = {
    "developer-document-upload.js",
    "profile-dropdown.js",
    "mobile-navigation.js",
}
REQUIRED_CSS_HINTS = {
    "developer-document-upload-step3.css",
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


def test_has_required_shell_ids(page):
    assert_required_ids(page, REQUIRED_IDS)


def test_has_required_dropzone_ids(page):
    assert_required_ids(page, DROPZONE_IDS)


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
