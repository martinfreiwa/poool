"""Static template-render checks for /developer/application-form.

Verifies the eleven core form fields each ship with their input id
(property-name, property-type, area, address, city, country, lease-type,
lease-term, land-size, building-size, bedrooms, bathrooms, status,
year-built, purchase-price, minimum-share-price) and the form action
buttons (back, save-exit, next).

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_application_form_static.py -v
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


ROUTE = "/developer/application-form"

SHELL_IDS = {
    "developer-application-form-body",
    "developer-application-form-page",
    "developer-application-form-sidebar",
    "developer-application-form-main",
    "property-details-section",
    "financials-section",
    "form-actions",
}
FIELD_IDS = {
    "property-name",
    "property-type",
    "area",
    "address",
    "city",
    "country",
    "lease-type",
    "lease-term",
    "land-size",
    "building-size",
    "bedrooms",
    "bathrooms",
    "status",
    "year-built",
    "purchase-price",
    "minimum-share-price",
}
ACTION_IDS = {
    "form-back-btn",
    "save-exit-btn",
    "form-next-btn",
}
REQUIRED_SCRIPT_HINTS = {
    "developer-application-form.js",
    "profile-dropdown.js",
    "mobile-navigation.js",
}
REQUIRED_CSS_HINTS = {
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


def test_has_shell_ids(page):
    assert_required_ids(page, SHELL_IDS)


def test_has_all_field_ids(page):
    assert_required_ids(page, FIELD_IDS)


def test_has_form_action_buttons(page):
    assert_required_ids(page, ACTION_IDS)


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
