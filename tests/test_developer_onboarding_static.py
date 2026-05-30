"""Static template-render checks for /developer/onboarding.

The onboarding flow is a 3-step prototype with its own stepper and review
screen. Two specific guards apply here:

* The ToS link in step 3 must point to `/terms`, NOT to `#`. That route is
  a real legal page; a bare-hash href was the bug we are protecting
  against.
* The submit CTA on step 3 (`#btn-submit`) must be present.

Note: this page does NOT load a dedicated developer-onboarding.js bundle.
The behaviour is inline (the template embeds its own scripts) so we only
assert the shared `profile-dropdown.js` is present.

Run:
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_onboarding_static.py -v
"""
import os
import re
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


ROUTE = "/developer/onboarding"

REQUIRED_IDS = {
    "developer-onboarding-body",
    "onb-stepper",
    "step-ind-1",
    "step-ind-2",
    "step-ind-3",
    "content-step-1",
    "content-step-2",
    "content-step-3",
    "btn-submit",
    "ob-first-name",
    "ob-last-name",
    "ob-email",
    "ob-phone",
    "ob-nationality",
    "ob-country",
}
REQUIRED_SCRIPT_HINTS = {
    "profile-dropdown.js",
    "mobile-navigation.js",
}
REQUIRED_CSS_HINTS = {
    "developer-onboarding.css",
    "developer-leaderboard-navbar.css",
}
REPO_ROOT = Path(__file__).resolve().parents[1]
ONBOARDING_CSS = REPO_ROOT / "frontend/platform/static/css/developer-onboarding.css"


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


def test_tos_link_points_to_terms_not_hash(response):
    """The Terms of Service link must go to /terms — never to #."""
    body = response.text
    # The href="/terms" link must be present.
    assert 'href="/terms"' in body, "Onboarding step 3 must link to /terms"
    # And we must not have a `Terms of Service</a>` after an href="#".
    bad = re.search(r'href="#"[^>]*>[^<]*Terms of Service', body, re.IGNORECASE)
    assert bad is None, "Found href=\"#\" placeholder for Terms of Service link"


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


def test_stepper_done_state_uses_blue_background_contract():
    css = ONBOARDING_CSS.read_text(encoding="utf-8")
    block = css.split(".onb-step--done .onb-step__circle", 1)[1].split("}", 1)[0]

    assert "background: var(--btn-primary-bg, #0000FF);" in block
    assert "color: var(--checkbox-selected-check-color, #03FF88);" in block
