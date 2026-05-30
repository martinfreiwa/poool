from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SIDEBAR = ROOT / "frontend/platform/components/sidebar.html"
SIDEBAR_NAV_CSS = ROOT / "frontend/platform/static/css/sidebar-navigation.css"
SIDEBAR_BEM_CSS = ROOT / "frontend/platform/static/css/bem/sidebar.css"
ICONS = ROOT / "frontend/platform/static/images/icons"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def test_sidebar_inline_icons_use_fine_line_strokes():
    sidebar = read(SIDEBAR)

    assert 'stroke-width="2"' not in sidebar
    assert 'stroke-width="1.67"' not in sidebar
    assert 'stroke-width="1.66667"' not in sidebar
    assert 'id="sidebar-search-icon"' in sidebar
    assert 'stroke-width="1.4"' in sidebar
    assert 'stroke-width="1.25"' in sidebar


def test_investor_sidebar_svg_icons_use_fine_line_strokes():
    one_and_half_stroke_icons = [
        "home-05.svg",
        "chart-line-up.svg",
        "line-chart-up-02.svg",
        "wallet-02.svg",
        "star-01.svg",
        "shopping-cart-01.svg",
        "award-05.svg",
        "users-01.svg",
    ]
    one_point_four_stroke_icons = [
        "settings-01.svg",
        "message-chat-circle-grey.svg",
    ]

    for icon_name in one_and_half_stroke_icons:
        icon = read(ICONS / icon_name)
        assert 'stroke-width="1.5"' in icon
        assert 'stroke-width="2"' not in icon
        assert 'stroke-width="2.004"' not in icon

    for icon_name in one_point_four_stroke_icons:
        icon = read(ICONS / icon_name)
        assert 'stroke-width="1.4"' in icon
        assert 'stroke-width="1.67"' not in icon


def test_sidebar_css_sources_match_fine_line_inline_icon_weight():
    for css_path in [SIDEBAR_NAV_CSS, SIDEBAR_BEM_CSS]:
        css = read(css_path)
        assert "stroke-width: 1.4px;" in css
