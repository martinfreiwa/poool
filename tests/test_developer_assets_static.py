"""Static checks for /developer/assets.

This module combines two layers of checks:

1) Source-only assertions on the template, JS, and CSS files (the original
   pre-HTTP regression tests — kept verbatim below). These run without a
   backend.
2) HTTP template-render assertions (status code, IDs, scripts, no
   placeholders) that hit a running backend if `DEV_SESSION_COOKIE` is set.
   They skip cleanly otherwise.

Run:
    python3 -m pytest tests/test_developer_assets_static.py -v
    BASE_URL=http://localhost:8888 DEV_SESSION_COOKIE=<session> \\
        python3 -m pytest tests/test_developer_assets_static.py -v
"""
import os
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[1]

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


def read(path: str) -> str:
    return (ROOT / path).read_text()


# --------------------------------------------------------------------------
# Source-only regression tests (no backend required) ----------------------
# --------------------------------------------------------------------------

def test_developer_assets_uses_management_table_and_preview_panel():
    template = read("frontend/platform/developer/assets.html")

    assert "dev-assets-table" in template
    assert "dev-assets-preview" in template
    assert "dev-assets-summary" in template
    assert "dev-assets-workspace" in template
    assert 'id="dev-assets-empty-row"' in template
    assert "No assets match your filters" in template
    assert "property-card dev-asset-card" not in template
    assert "ghost-card" not in template


def test_developer_assets_table_exposes_operational_columns_and_actions():
    template = read("frontend/platform/developer/assets.html")

    for heading in ("Asset", "Status", "Funding", "Value", "Duration", "Raised", "Actions"):
        assert (
            f"<th>{heading}</th>" in template
            or f'class="dev-assets-table__actions">{heading}</th>' in template
            or f'<span class="table__header-text">{heading}</span>' in template
        )

    assert 'href="/developer/asset-detail?id={{ asset.id }}"' in template
    assert 'href="/developer/asset-detail?id={{ asset.id }}&edit=1"' in template
    assert 'aria-label="View {{ asset.title }}"' in template
    assert 'aria-label="Edit {{ asset.title }}"' in template
    assert 'title="View asset"' in template
    assert 'title="Edit asset"' in template
    assert 'data-funding-pct="{{ funded_percentage }}"' in template
    assert 'data-cover-url="{{ asset.cover_image_url }}"' in template
    assert 'data-duration="{% if asset.lease_term_years %}{{ asset.lease_term_years }} yrs{% else %}N/A{% endif %}"' in template
    assert "dev-assets-location-line" in template
    assert 'data-dev-assets-count="all"' in template
    assert 'data-dev-assets-count="available"' in template
    assert 'data-dev-assets-count="funded"' in template


def test_developer_assets_javascript_filters_rows_and_updates_preview():
    js = read("frontend/platform/static/js/developer-assets.js")

    assert "function applyAssetFilters()" in js
    assert "function updatePreview(row)" in js
    assert "function isFundedRow(row)" in js
    assert "function updateFilterCounts()" in js
    assert "function clearPreview()" in js
    assert "function formatLocationDisplay(value)" in js
    assert "dev-assets-preview-image" in js
    assert "dev-assets-empty-row" in js
    assert "row.hidden = !visible;" in js
    assert "dev-asset-row.is-selected" in js


def test_developer_assets_design_system_density_styles_are_present():
    css = read("frontend/platform/static/css/developer-assets.css")

    assert "--dev-assets-page-width: 1320px;" in css
    assert "grid-template-columns: minmax(0, 1fr) 300px;" in css
    assert "min-width: 760px;" in css
    assert ".dev-assets-table" in css
    assert ".dev-assets-preview" in css
    assert ".dev-assets-empty-row__content" in css
    assert "#dev-assets-preview-edit" in css
    assert ".dev-assets-action svg" in css
    assert "width: 30px;" in css
    assert ".dev-assets-low-count" in css
    assert ".dev-assets-location-line" in css
    assert "background: #FFFFFF;" in css
    assert "border: 1px solid var(--dev-assets-border);" in css


def test_developer_assets_empty_hero_uses_branded_art():
    template = read("frontend/platform/developer/assets.html")
    css = read("frontend/platform/static/css/developer-assets.css")

    assert 'class="dae-empty__brand-art"' in template
    assert 'class="dae-empty__brand-lockup"' in template
    assert 'src="/static/images/icons/logo-pool.svg"' in template
    assert "67%" not in template
    assert "$50K" not in template
    assert 'id="dae-bg"' not in template

    assert ".dae-empty__brand-art {" in css
    assert ".dae-empty__brand-lockup {" in css
    assert "background: #03FF88;" in css
    assert "linear-gradient(135deg, #0000FF 0%, #001DCA 62%, #07107C 100%)" in css
    assert ".dae-empty__brand-progress span" in css


def test_developer_assets_empty_metrics_use_quiet_card_style():
    css = read("frontend/platform/static/css/developer-assets.css")

    metrics_start = css.index(".dae-empty__metric {")
    metrics_end = css.index(".dae-empty__metric .ds-stat-card__label", metrics_start)
    metric_css = css[metrics_start:metrics_end]

    assert "padding: 20px 22px 18px;" in metric_css
    assert "border-radius: 12px !important;" in metric_css
    assert ".dae-empty__metric::before" in css
    assert "background: linear-gradient(90deg, #0000FF 0%, #03FF88 100%);" in css
    assert "border-image" not in metric_css

    value_start = css.index(".dae-empty__metric .ds-stat-card__value {")
    value_end = css.index(".dae-empty__metric-hint", value_start)
    value_css = css[value_start:value_end]
    assert "font-size: 32px;" in value_css
    assert "line-height: 1;" in value_css


def test_developer_assets_empty_step_cards_match_submissions_design():
    css = read("frontend/platform/static/css/developer-assets.css")

    assert ".dae-empty__metrics {\n  display: grid;\n  grid-template-columns: repeat(3, minmax(0, 1fr));\n  gap: 24px;" in css
    assert ".dae-empty__steps {\n  display: grid;\n  grid-template-columns: repeat(3, minmax(0, 1fr));\n  gap: 24px;" in css
    assert ".dae-empty__step {\n  display: flex;\n  flex-direction: column;\n  gap: 14px;" in css
    assert "transform: translateY(-2px);" in css
    assert ".dae-empty__step-num {\n  display: inline-flex;" in css
    assert "background: linear-gradient(135deg, #0000FF 0%, #3344FF 100%);" in css
    assert "color: #03FF88;" in css


# --------------------------------------------------------------------------
# HTTP template-render tests (skip when backend unreachable) -------------
# --------------------------------------------------------------------------

ROUTE = "/developer/assets"

REQUIRED_IDS = {
    "developer-assets-body",
    "developer-assets-page",
    "developer-assets-sidebar",
    "developer-assets-main",
    "dev-assets-search-input",
}
REQUIRED_SCRIPT_HINTS = {
    "developer-assets.js",
    "property-card.js",
    "profile-dropdown.js",
    "mobile-navigation.js",
}
REQUIRED_CSS_HINTS = {
    "developer-assets.css",
    "developer-leaderboard-navbar.css",
}


@pytest.fixture(scope="module")
def response():
    return fetch_page(ROUTE)


@pytest.fixture(scope="module")
def page(response):
    return parse_page(response)


def test_http_status_200(response):
    assert response.status_code == 200


def test_http_meta_viewport_present(page):
    assert_meta_viewport(page)


def test_http_title_non_empty(page):
    assert_title_non_empty(page)


def test_http_has_required_ids(page):
    assert_required_ids(page, REQUIRED_IDS)


def test_http_required_scripts_loaded(page):
    assert_scripts_present(page, REQUIRED_SCRIPT_HINTS)


def test_http_required_stylesheets_loaded(page):
    assert_stylesheets_present(page, REQUIRED_CSS_HINTS)


def test_http_status_tabs_present(page):
    # The "all" / "available" / "funded" filter tabs drive the JS row filter.
    found = [v for (_t, k, v) in page.data_attrs if k == "data-dev-assets-tab"]
    assert "all" in found, f"data-dev-assets-tab=\"all\" missing — found {found}"


def test_http_no_placeholder_or_lorem_text(response):
    assert_no_forbidden_global_text(response, extra=("coming soon",))


def test_http_no_references_to_deleted_files(response):
    assert_no_deleted_file_refs(response)


def test_http_no_bare_anchor_placeholders(page):
    assert_no_placeholder_anchors(page)
