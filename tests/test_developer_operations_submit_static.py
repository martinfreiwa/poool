"""Static template-render checks for /developer/villas/<asset_id>/operations/new.

The submit form for a single villa × period. The Rust handler serves the
template unconditionally for any well-formed asset_id; per-villa write
authorization happens at the API layer. We use the nil UUID for the URL
parameter — the page-side JS handles the empty hydrate gracefully and
still renders the static scaffold.

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_operations_submit_static.py -v
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
ROUTE = f"/developer/villas/{SAMPLE_UUID}/operations/new"

REQUIRED_IDS = {
    "developer-operations-submit-body",
    "dop-form",
    "dop-breadcrumb",
    "dops-asset-name",
    "dops-period-text",
    "dop-status",
    "dop-error",
    "btn-save-draft",
    "btn-submit",
    "dops-custom-expenses-list",
    "btn-add-expense",
}
# A representative subset of the per-expense field IDs the form ships.
EXPENSE_FIELD_IDS = {
    "dop-gross-rental",
    "dop-nights-available",
    "dop-nights-booked",
    "dop-expense-cleaning",
    "dop-expense-maintenance",
    "dop-expense-utilities",
    "dop-expense-other",
    "dop-expense-capex",
    "dop-ota-fees",
    "dop-payment-fees",
    "dop-refunds",
    "dop-mgmt-fee",
}
REQUIRED_SCRIPT_HINTS = {
    "developer-operations-submit.js",
}
REQUIRED_CSS_HINTS = {
    "developer-operations-submit.css",
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


def test_has_core_form_ids(page):
    assert_required_ids(page, REQUIRED_IDS)


def test_has_expense_field_ids(page):
    assert_required_ids(page, EXPENSE_FIELD_IDS)


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
