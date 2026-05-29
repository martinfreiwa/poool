import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_native_selects_share_design_system_contract():
    css = read("frontend/platform/static/css/ds-forms.css")

    for class_name in [
        ".ds-select",
        ".ds-input",
        ".settings-input",
        ".form-select",
        ".dropdown-select",
        ".support-select",
        ".ad-select",
        ".commissions-select",
        ".mp-sec__sort-select",
        ".mp-filter-select",
    ]:
        assert class_name in css
    assert "body:not(.admin-body) :where(select:not([multiple]):not(.admin-select):not(.admin-input):not(.dat-select):not(.dat-select--compact):not(.lb-select-mini):not(.dops-type-select):not([data-native-select]))" in css
    assert "--select-chevron-icon" in css
    assert "background-image: var(--select-chevron-icon);" in css
    assert "background-size: var(--select-chevron-size);" in css
    assert "border: 1px solid var(--input-border-color, #D0D5DD);" in css
    assert "box-shadow: 0 0 0 3px rgba(0, 0, 255, 0.08);" in css
    assert ":not(.admin-select):not(.admin-input)" in css
    assert ":not(.dat-select):not(.dat-select--compact):not(.lb-select-mini):not(.dops-type-select)" in css


def test_custom_dropdown_uses_same_form_tokens():
    css = read("frontend/platform/static/css/poool-dropdown.css")

    assert "--pd-primary: var(--input-border-focus, var(--primary-color, #0000FF));" in css
    assert "--pd-accent: var(--brand-green, #03FF88);" in css
    assert "--pd-text-main: var(--input-text-color, var(--page-title-color, #101828));" in css
    assert "--pd-border: var(--input-border-color, #D0D5DD);" in css
    assert "--pd-bg: var(--input-bg, #FFFFFF);" in css
    assert "box-shadow: 0 0 0 3px rgba(0, 0, 255, 0.08);" in css


def test_transactions_filter_no_longer_overrides_select_design_inline():
    html = read("frontend/platform/transactions.html")

    assert 'id="transactions-type-filter" class="ds-select" style="min-width:160px;"' in html
    assert 'id="transactions-type-filter" class="ds-select" style="min-width:160px;padding:8px 12px' not in html


def test_dropdown_auto_init_has_explicit_future_opt_in():
    js = read("frontend/platform/static/js/poool-dropdown-init.js")

    assert "select[data-poool-dropdown]" in js
    assert "select.admin-select" in js
    assert "Automatically converts opted-in native <select> elements" in js


def test_page_specific_dropdowns_reuse_shared_select_tokens():
    cases = {
        "frontend/platform/static/css/developer-asset-detail.css": [
            ".ad-select",
            "background-image: var(--select-chevron-icon);",
            "background-position: var(--select-chevron-position);",
            "box-shadow: 0 0 0 3px rgba(0,0,255,0.08);",
        ],
        "frontend/platform/static/css/marketplace-secondary.css": [
            ".mp-sec__sort-select",
            "background-image: var(--select-chevron-icon);",
            "box-shadow: 0 0 0 3px rgba(0, 0, 255, 0.08);",
        ],
        "frontend/platform/static/css/rewards.css": [
            "body#rewards-body .ds-select,",
            "body#rewards-body .commissions-select",
            "background-image: var(--select-chevron-icon) !important;",
        ],
        "frontend/platform/static/css/community.css": [
            ".ds-modal[role=\"dialog\"] select.ds-input",
            ".ccs-resource-form select.ds-input",
            "background-image: var(--select-chevron-icon);",
        ],
        "frontend/platform/static/css/developer-affiliate-team.css": [
            ".dat-select",
            ".dat-select--compact",
            "background-image: var(--select-chevron-icon);",
        ],
        "frontend/platform/static/css/leaderboard.css": [
            ".lb-select-mini",
            "background-image: var(--select-chevron-icon);",
        ],
        "frontend/platform/static/css/developer-operations-submit.css": [
            ".dops-type-select",
            "background-image: var(--select-chevron-icon);",
        ],
    }

    for path, snippets in cases.items():
        content = read(path)
        for snippet in snippets:
            assert snippet in content, f"{snippet} missing from {path}"


def test_generated_dropdowns_avoid_inline_legacy_select_design():
    commodities = read("frontend/platform/static/js/commodities-marketplace.js")
    p2p = read("frontend/platform/static/js/marketplace-p2p.js")
    secondary = read("frontend/platform/marketplace-secondary.html")

    assert '<select id="filter-commodity-type" class="ds-select">' in commodities
    assert 'id="filter-commodity-type"\n            style=' not in commodities
    assert '<select id="p2p-side" class="ds-select">' in p2p
    assert "background-image: var(--select-chevron-icon);" in p2p
    assert '<select id="interest-expiry" class="ds-select">' in secondary


def test_non_admin_inline_select_styles_only_control_sizing():
    allowed_props = {"width", "min-width", "max-width"}
    denied_props = {"height", "border", "border-radius", "padding", "font-size", "background", "appearance", "color"}
    roots = [
        ROOT / "frontend/platform",
        ROOT / "frontend/platform/static/js",
    ]

    for root in roots:
        for path in root.rglob("*"):
            if path.suffix not in {".html", ".js"}:
                continue
            rel = path.relative_to(ROOT).as_posix()
            if any(skip in rel for skip in ["/admin/", "/static/js/admin-", "/_archive/", "/vendor/", "charts-showcase"]):
                continue

            content = path.read_text()
            for tag in re.findall(r"<select\b[^>]*\bstyle=\"([^\"]*)\"[^>]*>", content):
                props = {part.split(":", 1)[0].strip().lower() for part in tag.split(";") if ":" in part}
                assert props <= allowed_props, f"{rel} has non-sizing inline select style: {props}"
                visual_props = {prop for prop in props if prop in denied_props or prop.startswith("background")}
                assert not visual_props, f"{rel} has visual inline select style: {visual_props}"
