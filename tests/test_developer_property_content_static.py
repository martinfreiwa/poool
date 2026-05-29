"""Static template-render checks for /developer/property-content.

Includes a regression guard for C-6: the rental-yield and total-return
percent inputs MUST NOT ship with a hardcoded `value="10"` — the template
allowed the placeholder "10" but the actual value must be empty so the
form drives off the persisted draft.

Also asserts that #submitted-asset-title is absent here — it only lives on
the submission-success page.

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_property_content_static.py -v
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


ROUTE = "/developer/property-content"

REQUIRED_IDS = {
    "property-content-body",
    "property-content-page",
    "property-content-sidebar",
    "property-content-main",
    "property-content-form",
    "financials-section",
    "financials-title",
    "financials-row1",
    "financials-row2",
    "financials-row3",
    "rental-yield",
    "rental-yield-group",
    "capital-appreciation",
    "capital-appreciation-group",
    "total-return",
    "total-return-group",
}
REQUIRED_SCRIPT_HINTS = {
    "developer-property-content.js",
    "profile-dropdown.js",
}
REQUIRED_CSS_HINTS = {
    "developer-property-content.css",
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


def test_c6_rental_yield_has_no_hardcoded_value(page):
    """C-6 regression: rental-yield input must not have value="10"."""
    rental_yield_inputs = [i for i in page.inputs if i.get("id") == "rental-yield"]
    assert rental_yield_inputs, "rental-yield input not found"
    for inp in rental_yield_inputs:
        value = inp.get("value")
        assert value in (None, "", "—"), (
            f"#rental-yield must not ship with a hardcoded value (got value={value!r}). "
            "C-6 regression: the form must read from the persisted draft."
        )


def test_c6_total_return_readonly_and_no_hardcoded_value(page):
    total_returns = [i for i in page.inputs if i.get("id") == "total-return"]
    assert total_returns, "total-return input not found"
    for inp in total_returns:
        assert "readonly" in inp, "#total-return must be readonly (computed field)"
        value = inp.get("value")
        assert value in (None, "", "—"), (
            f"#total-return must not ship with a hardcoded value (got value={value!r})"
        )


def test_submitted_asset_title_id_is_absent_here(page):
    # That ID only exists on /developer/submission-success.
    assert "submitted-asset-title" not in page.ids, (
        "#submitted-asset-title should only live on /developer/submission-success"
    )


def test_required_scripts_loaded(page):
    assert_scripts_present(page, REQUIRED_SCRIPT_HINTS)


def test_required_stylesheets_loaded(page):
    assert_stylesheets_present(page, REQUIRED_CSS_HINTS)


def test_no_placeholder_or_lorem_text(response):
    assert_no_forbidden_global_text(response, extra=("coming soon",))


def test_no_value_equals_10_in_percent_inputs(response):
    """Belt-and-braces text check for the C-6 regression."""
    # An <input value="10"> followed by anything other than digits is the
    # forbidden form. Use a permissive regex that catches both quote styles.
    assert not re.search(
        r'<input[^>]*\bvalue\s*=\s*["\']10["\']',
        response.text,
        re.IGNORECASE,
    ), "Found <input value=\"10\"> — C-6 regression on percent fields"


def test_no_references_to_deleted_files(response):
    assert_no_deleted_file_refs(response)


def test_no_bare_anchor_placeholders(page):
    assert_no_placeholder_anchors(page)
