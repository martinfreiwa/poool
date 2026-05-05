from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


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
        assert f"<th>{heading}</th>" in template or f'class="dev-assets-table__actions">{heading}</th>' in template

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
